# Google Photos Takeout ZIP streaming plan

**Date:** 2026-11-03  
**Status:** Proposed design; no implementation changes made.

## Goal

Import a multi-ZIP Google Photos Takeout while preserving the current behavior:

- Deduplicate against the user's existing media by sanitized filename.
- Combine album memberships when a filename occurs in multiple Takeout albums or ZIPs.
- Create or reuse albums using Takeout album metadata.
- Apply Takeout sidecar timestamps as the current fallback when media analysis has low confidence.
- Place new media in the normal media root and enqueue the existing metadata, thumbnail, and analysis jobs.
- Keep the uploaded ZIPs available until the batch is safely committed, then remove them.

Avoid expanding the entire ZIP set into `app_data/cache/takeout_tmp`. The ZIPs are about 148.4 GB and expand to about 148.37 GB, so the current approach temporarily needs both copies on app data.

## Current behavior

The current batch handler extracts every ZIP, then walks the extracted `Takeout/Google Photos` folders to create albums, index media and sidecars, deduplicate, move new files to the media root, and queue ingest. The expanded tree is only consumed by the Takeout batch handler; the later ingest worker reads normal media files from the media root and gets Takeout album IDs and fallback timestamps from its job payload ([current batch handler](../server/crates/apps/worker/src/handlers/ingest_takeout_batch.rs), [metadata ingest handler](../server/crates/apps/worker/src/handlers/ingest_metadata.rs)).

## Proposed flow

### 1. Keep the HTTP start route fast

The `/api/upload/takeout/start` route should validate the user's completed Takeout upload metadata, identify unclaimed upload IDs, enqueue the batch job, and return. It should not reconstruct multi-gigabyte ZIPs inline.

The worker already calls `repair_tus_upload` before opening each ZIP. Keep that worker-side repair and move expensive repair work out of the API path. This matters because the supplied production Nginx log shows a 504 for the start request after it timed out waiting for the backend.

Separately, raise the `/api` proxy timeout in `web/nginx.conf` enough for long final TUS PATCH requests. Moving repair out of the start route does not by itself change the time needed to finalize an upload.

### 2. Preflight and index ZIP entries without extracting media

For each upload ID in the batch:

1. Repair interrupted TUS finalization in the worker.
2. Confirm the final ZIP exists and its file size matches the declared upload size before parsing it.
3. Open the ZIP central directory and inspect entry names, sizes, and indices. Do not write media entries to disk during this pass.
4. Find the Google Photos root in each archive, supporting the layouts the current importer accepts: `Takeout/Google Photos`, `Google Photos`, or the existing fallback layout.
5. Normalize and validate paths using the same trimming and safe-path rules as the current importer.

Keep a compact in-memory manifest of relevant entries:

- Album folder path and the source ZIP entry for that folder's `metadata.json`.
- Media entries with the sanitized filename, source ZIP ID, entry index, album folder, and uncompressed size.
- Sidecar entries keyed by normalized album folder and lowercased sanitized sidecar filename.

The ZIP central directories contain about 79,000 entries for this dataset. The manifest stores paths and identifiers, not image or video contents.

### 3. Reproduce album and sidecar behavior from the manifest

Read the small `metadata.json` and sidecar JSON entries directly from their ZIPs. Do not persist those JSON files to a staging directory.

Preserve the existing semantics:

- Use each top-level Google Photos folder as an album source.
- Prefer the album title and description from `metadata.json`, falling back to the folder name.
- Reuse an existing album by name for the user, as the current importer does.
- Preserve all sidecar candidate-name variants, extension-case handling, and `-edited` fallbacks.
- For a media filename repeated across folders or ZIPs, union its album IDs and choose one deterministic source media entry.
- Continue filename-based deduplication against the user's database records; do not silently switch this import to content-hash deduplication.

The current code's map retains one source per sanitized filename and accumulates album IDs. The new manifest should make the source-choice rule deterministic—for example, archive order followed by entry path—while preserving the album union.

### 4. Look up existing media and prepare album links

Query existing media by the manifest's unique sanitized filenames. For existing items, skip writing the file and group the album links for insertion, matching current behavior.

For new filenames, retain:

- The selected source ZIP and entry index.
- The union of album IDs.
- The matching sidecar's fallback timestamp, if any.
- The destination filename/path selected by the existing unique-filename policy.

Create or reuse the albums before enqueueing ingest so ingest jobs never refer to missing album IDs.

### 5. Stream only new media entries to the media volume

Group new media entries by source ZIP so each archive can be opened once and entries can be read in a predictable order. Read each selected ZIP entry as a stream, decompressing directly into a temporary file on the media volume. After the stream completes and the expected byte count is verified, atomically rename the temporary file to its final managed Takeout path.

Use temporary names that cannot be mistaken for media files and clean abandoned temporary files after a failed attempt. Keep temporary output outside any watcher path where practical; only the final rename should publish the media file.

This writes the new media to its final storage volume without first making an expanded copy on app data. The normal ingest worker can then process the final file path exactly as it does today.

### 6. Preserve the existing job and album behavior

For existing media items, add the accumulated album links.

For newly written media, enqueue the same full-ingest jobs as today. The metadata job payload should continue to carry `album_ids` and `fallback_taken_at`; the latter is consumed by `IngestMetadata` when media analysis has low timestamp confidence. If applicable, the current worker writes the fallback timestamp into the media file and updates the database fields.

Continue to use the current bulk job enqueue path unless implementation profiling shows it needs chunking. The local run successfully inserted a batch for 21,035 new items.

### 7. Make partial failures retry-safe

There is no single transaction spanning media files and PostgreSQL, so retries need explicit idempotency behavior:

- Keep the source ZIPs until all destination files, album links, and ingest jobs are safely recorded.
- Use stable destination names or a small per-item checkpoint so a rerun reuses a file already written by the previous attempt instead of creating a `(1)` duplicate.
- Write to temporary files and publish with atomic rename; remove partial temporary files on failure.
- If a failure occurs after files are materialized but before jobs are committed, a retry must discover and enqueue those existing files.
- Clean the batch's temporary output on failure or retry without deleting source ZIPs.
- Remove the TUS ZIP and `.info` files only after the import batch reaches its successful completion point.

### 8. Report progress at batch and ZIP boundaries

Log the batch ID, ZIP count, source ZIP currently being scanned, number of indexed albums/media/sidecars, new versus existing media counts, bytes written, and ingest jobs enqueued. Log the failing ZIP entry and operation when archive reading or output writing fails. Keep these logs in the worker so a slow import is visible as a job rather than a long HTTP request.

## Storage implications

With this approach, app data still stores the uploaded ZIPs—about 148.4 GB for this batch—until cleanup. It no longer needs another roughly 148.37 GB for the expanded media tree. A TUS finalization repair can still need transient space for a ZIP-sized temporary reconstruction; for these archives, the largest one is about 10.7 GB. If reducing app-data use below the uploaded ZIP size is also a goal, moving the TUS upload store to a dedicated location on the larger media volume is a separate change.

The media drive must have room for the new media that will remain after import. The user's reported 1.6 TB free should comfortably exceed this batch's roughly 148 GB of archive contents, subject to how many filenames are already present and any final unique-name collisions.

## Pros

- Avoids the largest temporary app-data requirement: the full expanded copy of the Takeout.
- Reduces app-data writes and reads by avoiding a complete extracted tree.
- Sends only the unique new media files to the media volume; already-ingested media only gains album links.
- Preserves album metadata and sidecar timestamp behavior without saving JSON sidecars on disk.
- Keeps the regular ingest pipeline unchanged: it still sees ordinary media files and the existing payload fields.
- Makes import progress and failures visible in the background job system instead of holding an HTTP request open.

## Cons and risks

- More importer logic is needed than the current filesystem walk: ZIP entry paths, album structure, sidecar matching, source selection, and stream failures must all be handled.
- Batch-wide filename dedup requires a pre-scan and an in-memory manifest before files are written. This is modest for this dataset, but grows with entry count and metadata volume.
- A source ZIP may need to be reopened or held open while processing. Grouping work by ZIP limits file-handle churn and seek overhead.
- Direct stream decompression is synchronous CPU/I/O work in the ZIP crate. Run it through a bounded blocking worker pool rather than occupying an async runtime thread.
- Filesystem writes and database job insertion cannot be made one atomic transaction. Retry-safe destination selection or an import-item checkpoint is required to avoid orphan files or duplicate suffixes.
- TUS uploads still occupy app data while the import runs; this plan removes the expanded copy, not the uploaded source archives.
- Long TUS finalization requests can still encounter the web proxy timeout. The proxy timeout and slow repair placement require separate changes.
- The importer must preserve safe ZIP path validation and must not allow entries outside the expected Google Photos tree to escape the media destination.

## Alternatives

### Extract one ZIP at a time to media-volume scratch

This is closer to the existing code and would cap expanded scratch at roughly one ZIP (about 8.7–10.7 GB) instead of the whole 148 GB batch. It still needs batch-wide indexing or a durable filename/album map to preserve deduplication and sidecar behavior across ZIP boundaries. It also requires watcher-safe scratch placement and cleanup.

### Move the entire app-data volume to larger storage

This requires fewer code changes, but retains the current expanded copy and its roughly 297 GB peak import footprint on app data. It is an operational workaround rather than a lower-space import design.

## Recommended implementation order

1. Make `/api/upload/takeout/start` enqueue quickly and leave large-file repair to the worker; ensure worker logs identify repair and batch progress.
2. Add a ZIP manifest/index layer that handles roots, album metadata, media entries, sidecars, and existing filename semantics without extracting media.
3. Stream selected new media entries to media-volume temporary files and atomically publish them.
4. Add retry-safe destination/checkpoint behavior and cleanup of partial output.
5. Set suitable proxy timeouts for the API and TUS upload path.
6. Verify behavior against the existing Takeout integration scenarios, including repeated filenames across albums, sidecar naming variants, existing media linking, interrupted finalization, and a failed/retried batch.
