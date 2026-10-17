# PLAN — Google Takeout Import

## 0. TL;DR

User drops the takeout zips into the **existing** upload dropzone. `handle_web_upload`
detects takeout zips by filename, stages them, and enqueues one new `TakeoutImport`
job per zip. That job unzips into `{user media folder}/takeout/{batch_id}/`, creates
one album per folder under `Google Photos/` (title/description from `metadata.json`),
dedups by filename, and enqueues plain `IngestMetadata` jobs for files not yet
imported. Album membership is resolved via a small **pending-link table** that
`ingest_metadata` consumes in the same transaction that inserts the media item —
no phase tricks, no finalize job, no races. One migration.

---

## 1. Verified facts about the takeout format (from your sample)

- Zips: `takeout-{YYYYMMDD}T{HHMMSS}Z-{export#}-{chunk#}.zip`, e.g.
  `takeout-20261015T162830Z-1-001.zip`. Chunks are **independent archives** (Google
  never splits a single file across zips) — each zip can be processed standalone.
  The timestamp is a natural batch id.
- Inner structure: `Takeout/Google Photos/` containing **one flat level of folders**:
    - Album folders (with `metadata.json`: `title`, `description`, `date`,
      `enrichments` → enrichments ignored per spec).
    - `Photos from YYYY` folders (no `metadata.json`) → become albums titled as the
      folder name (your proposal — agreed, it simplifies everything: *every* folder
      under `Google Photos/` is an album, uniformly).
    - `Archive` → treated as a regular album named "Archive" (Google uses it for
      hidden state, but per "every folder = album" it just becomes an album).
    - Root-level files in `Google Photos/`: only `shared_album_comments.json` and
      `user-generated-memory-titles.json` — both ignored.
- Per-photo sidecars: `IMG_x.jpg.supplemental-metadata.json` (title, taken time,
  geo, url). Ignored for import (files carry EXIF already); a future enhancement
  could use `photoTakenTime`/`geoData` for files with stripped EXIF.
- Duplicate names inside one folder get Google's `(1)` suffix
  (`20160809_115820.jpg` + `20160809_115820(1).jpg`) — those are *different files*
  with different filenames, so filename dedup never sees them as equal. Good.
- The same photo frequently appears in 2–3 folders (album A, album B,
  `Photos from YYYY`) — this is the main duplication to handle.

## 2. Flow

```
User drags zips onto existing dropzone
  └─ tus upload (50 MiB chunks, resumable)  → no frontend changes at all
       └─ UploadFinished hook → HandleWebUpload job (existing)
            └─ filename starts with "takeout" (case-insensitive) and ends .zip?
                 ├─ no  → existing behavior, unchanged
                 └─ yes → parse batch_id from filename ("takeout" fallback),
                          move file from tus/uploads to {app_data}/takeout/{batch_id}/,
                          enqueue TakeoutImport job (payload: zip path, batch_id)
       └─ TakeoutImport job (new, global, phase 20)
            ├─ stream-extract zip → {media_root}/{user}/takeout/{batch_id}/...
            ├─ create/find albums (takeout_id = batch_id)
            ├─ filename dedup → keep one canonical copy per filename,
            │   delete the rest, register pending links
            └─ bulk_enqueue_full_ingest for the new files
                 └─ IngestMetadata (existing) → upsert item
                      └─ consumes pending links for (user_id, filename) in same tx
                          → add_media_items per album + set thumbnail if album has none
```

Key property: **registration commits before enqueue**, so whichever zip
registers a filename first wins the ingest, and every later occurrence becomes a
pending link row. Whether the ingest for that filename runs before, during, or
after any other zip's registration, the outcome is the same:

- item already in DB at registration time → link attached immediately (direct
  attach, no job);
- item not in DB yet → link row pending → consumed by the ingest itself.

No ordering guarantee between zips is required. This is the whole reason for the
pending-link design (see §4 pushback).

## 3. Why not phase-based ordering

You don't need it, and it wouldn't help: the "when do I attach albums" problem is
cross-job, cross-zip state, not a scheduling problem. Phases only guarantee
*claim* order for jobs that already exist — a zip 3 upload can arrive minutes
after zip 1's ingest already ran, so no static ordering can express "attach album
B to the item from zip 1". A durable pending record + consume-on-ingest is the
only stateless-looking, race-free option. Concretely the claim query
(`claim_next_job`) isn't touched at all.

## 4. Pushback: multi-album `IngestMetadataPayload` alone is not enough

You asked to extend `IngestMetadataPayload { album_id, remote_user_identity }` to
carry multiple albums. I recommend **not** doing that for takeout, and instead
using the pending-link table. Reasoning:

1. **Dedup is by `relative_path`, not filename** (`MediaItemStore::upsert` keys on
   it). The same photo lives at several extracted paths
   (`.../Album A/F.jpg`, `.../Album B/F.jpg`, `.../Photos from 2016/F.jpg`).
   If we enqueued an ingest per occurrence with a multi-album payload, we'd get
   **multiple media_item rows** — exactly what you want to avoid. So only one
   occurrence may be ingested; the others must be *linked*, not ingested.
2. Linking an already-registered-but-not-yet-ingested file means mutating a job
   that is queued (`UPDATE jobs SET payload...`) — racy if the worker already
   claimed/started it (handler reads payload once at start), and ugly otherwise.
3. The pending-link table is your "pending IngestMetadataPayload" idea made
   durable: instead of albums-in-payload, it's pending-album-rows-per-filename
   consumed atomically at ingest time. It supports arbitrarily many albums per
   file, survives restarts, and needs **no changes at all** to
   `IngestMetadataPayload` (which stays single-album, still used by the s2s
   `ImportAlbumItem` flow).

If you feel strongly about the payload variant we can do both (payload for the
canonical occurrence's albums, links for the late duplicates), but the payload
adds nothing the table doesn't already do — I'd skip it.

## 5. Migration (single file: `migrations/{ts}_takeout.sql`)

```sql
-- Albums created by a takeout import. NULL = regular album.
-- Doubles as the "created by takeout" flag you asked for.
ALTER TABLE album
    ADD COLUMN takeout_id TEXT NULL;
CREATE INDEX idx_album_takeout ON album (owner_id, takeout_id, name);

-- One row per (album, filename) occurrence in any takeout.
-- Pending (media_item_id IS NULL) = waiting for ingest to consume it.
CREATE TABLE takeout_album_link
(
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id       INTEGER     NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    batch_id      TEXT        NOT NULL,
    album_id      VARCHAR(10) NOT NULL REFERENCES album (id) ON DELETE CASCADE,
    filename      TEXT        NOT NULL,
    media_item_id VARCHAR(10) NULL REFERENCES media_item (id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX uq_takeout_album_link ON takeout_album_link (user_id, album_id, filename);
-- hot path: consume_pending_links at ingest time
CREATE INDEX idx_takeout_link_pending
    ON takeout_album_link (user_id, filename) WHERE media_item_id IS NULL;

-- filename dedup lookups (registered files + "already imported?" checks)
CREATE INDEX idx_media_item_user_filename ON media_item (user_id, filename);
```

Notes:

- `media_item_id ON DELETE CASCADE`: if an item is later deleted, its consumed
  links vanish, so a re-import would correctly re-ingest.
- `batch_id` is per-row (not only on album) so cleanup/reporting can be scoped per
  import without touching album semantics.

After writing it: run `scripts/recreate_db.ps1` (dev DB is disposable), and after
the new queries are in code, `scripts/update_sqlx.ps1` to refresh the sqlx
offline data.

## 6. Code changes, file by file

### `common_services`

- `database/jobs.rs`: add `JobType::TakeoutImport`; `phase()` → 20 (same phase as
  IngestMetadata). Scope is `global` (the zip lives in app_data staging, not under
  media_root, so there is no meaningful `relative_path` for path-scoped scheduling)
  but the job still carries `user_id` — "global" only affects claim gating, not user
  ownership. Phase 20 (not 10) matters: a running global job lowers
  `min_global_phase`, so at phase 10 the multi-minute zip extraction would block
  every `IngestMetadata` (path, phase 20) system-wide; at phase 20 they stay
  claimable (`20 <= 20`) and multiple TakeoutImport jobs (one per zip) can run
  concurrently. Only global jobs with phase > 20 (Scan, CleanDb, clusters) wait —
  same as during any ordinary ingest activity.
- `database/media_item_store.rs`: `find_ids_by_filenames(tx, user_id, &filenames)
  -> Vec<(filename, id)>` (chunked `= ANY($2)`, ~1000 per query).
- `database/album_store.rs`: `find_or_create_takeout_album(tx, user_id, name,
  description, takeout_id)` — `SELECT ... WHERE owner_id = $1 AND takeout_id = $2
  AND name = $3`, else `INSERT` with default sort mode (`DateAsc`, same as
  normal `create_album`; date-sorted albums never write ranks → zero deadlock
  risk, exactly what the album-rank rewrite unblocked per PLAN-album-rank §8).
- new `database/takeout_link_store.rs`:
    - `insert_links(tx, user_id, batch_id, album_id, filenames)` (bulk, ON CONFLICT
      DO NOTHING)
    - `consume_pending(tx, user_id, filename, media_item_id) -> Vec<album_id>`
      — `UPDATE ... SET media_item_id = $3 WHERE user_id = $1 AND filename = $2
    AND media_item_id IS NULL RETURNING album_id` (one statement, atomic)
    - `find_known_filenames(tx, user_id, &filenames) -> HashSet<String>` — any link
      row for those filenames (pending ⇒ ingest already enqueued; consumed ⇒ item
      already exists)
    - shared helper `attach_albums_if_missing_thumbnail(...)` — extract the
      thumbnail-if-none snippet currently inlined in `ingest_metadata.rs
    store_media_item` so both attach paths reuse it.
- `api/album` response model: include `takeout_id` (nullable) so the frontend can
  badge these albums later.

### `worker` crate

- `handlers/handle_web_upload.rs`: after sanitizing the filename, if
  `filename.starts_with("takeout")` (ci) `&& ends_with(".zip")`:
  parse batch_id via regex `^takeout-(\d{8}T\d{6}Z)-\d+-\d+\.zip$` (ci), fallback
  `batch_id = "takeout"`; move the tus file to
  `{app_data_root}/takeout/{batch_id}/{sanitized_filename}` (delete its `.info`);
  enqueue `JobType::TakeoutImport` with payload
  `TakeoutImportPayload { zip_app_data_path, batch_id, original_filename }`.
  Otherwise: existing behavior, byte-for-byte unchanged.
- new `handlers/takeout_import.rs` (the meat):
    1. `require_media_mounted`, resolve user media folder, read+delete zip
       (extract into final location; delete zip after success so 60 GB of zips
       don't double the disk usage).
    2. **Extract pass** — stream the archive entry-by-entry with the `zip` crate
       (already a workspace dep in `apps/api` — move/extend the dependency as
       needed):
        - sanitize every entry path (zip-slip: reject `..`, absolute paths);
        - strip leading `Takeout/` component; keep `Google Photos/{folder}`;
          destination root `{user_media_folder}/takeout/{batch_id}/`;
        - skip `*.json` / `*.html` files, **except** capture `*/metadata.json`
          contents into an in-memory map `folder → (title, description)`;
        - set extracted file mtime from the zip entry timestamp (helps files with
          no EXIF — analyzer falls back to mtime);
        - per-entry errors: log + continue (one bad file must not kill a 10 GB
          extraction); track skipped counts in the job log.
    3. **Register pass** — walk extracted media files (reuse
       `IngestSettings::is_media_file` + size > 0):
        - resolve folder → album id (`find_or_create_takeout_album`; folders with no
          metadata.json → title = folder name, description = null). Root-level loose
          media files (none expected) get ingested without an album.
        - batch-check "known filenames" = DB items by filename (`find_ids_by_filenames`)
          ∪ link rows (`find_known_filenames`) ∪ in-memory set from this zip.
        - known → `fs::remove_file` the extracted copy, then **direct attach**:
          `add_media_items(album_id, [existing_item_id])` + link row with
          `media_item_id` set + thumbnail-if-none. (If the item only exists as a
          *pending* ingest from an earlier zip: just insert the pending link row,
          no file delete decision needed — the file is still deleted since its
          filename already has a canonical copy coming.)
        - new → keep file, insert pending link row, remember relative_path.
        - commit registration (links + albums) **before** enqueueing.
    4. `bulk_enqueue_full_ingest(&new_canonical_paths)` — it can't carry payloads
       and doesn't need to (links do the attaching). Returns `JobResult::Done`.
    5. Hygiene: at the start, opportunistically
       `DELETE FROM takeout_album_link WHERE user_id = $1 AND media_item_id IS NULL
       AND created_at < now() - interval '7 days'` — clears links for files whose
       ingest permanently failed, so they can be re-imported later.
- `handlers/ingest_metadata.rs`: in `store_media_item`, after `upsert`:
  ```rust
  let filename = Path::new(relative_path).file_name()...;
  let album_ids = TakeoutLinkStore::consume_pending(&mut tx, user_id, filename, item_id).await?;
  for album_id in album_ids { AlbumStore::add_media_items(...); }
  attach_albums_if_missing_thumbnail(&mut tx, &album_ids, item_id).await?;
  ```
  Unconditional (one cheap indexed query per ingest; pending index is empty for
  everyone not importing a takeout). No payload changes, no special-casing, and
  it's inside the upsert tx ⇒ atomic.
- `handlers/mod.rs`: dispatch `JobType::TakeoutImport`.

### Watcher + Scan (double-ingest guard)

- `apps/watcher/src/watcher.rs`: `EXCLUDED_WATCH_FOLDER` → add a new
  `TAKEOUT_FOLDER` constant (`"takeout"`, in `app_state/src/constants.rs` next to
  `ALBUM_IMPORT_FOLDER` / `USER_UPLOAD_FOLDER`).
- `apps/worker/src/handlers/scan.rs`: skip paths under the same constant (a
  scheduled 24 h Scan firing mid-import would otherwise re-enqueue payload-less
  ingest jobs for registered-but-not-yet-ingested files; harmless because of
  upsert, but wasteful for 10k+ files).

### API crate

- No new endpoints. The existing `/jobs/ingest/*` overview + running feed covers
  progress for both the `TakeoutImport` job and the thousands of per-file ingest
  jobs.

### Web frontend

- `src/scripts/types/api/admin.ts`: add `takeout_import` to `JobType`.
- `IngestRunningFeed`/`RunningJobPill` label: today they strip `ingest_`; add a
  tiny `takeout_import → "takeout import"` mapping (or generic humanize).
- Upload UX: **unchanged** — existing dropzone, tus progress, retry, resume all
  work as-is for 10 GB zips.
- Optional polish (separate small PR): album card badge / subtitle when
  `takeoutId` is set (e.g. "from Google Takeout"), since the album response now
  carries it.

## 7. Edge cases handled

| Case                                                                  | Behavior                                                                                                                                                              |
|-----------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Same photo in album A + album B + `Photos from 2016`                  | One media item (first-registered copy kept), member of all three albums. Extra extracted copies deleted from disk.                                                    |
| Photo already in user's library (uploaded normally)                   | No new item; takeout occurrence only links the album.                                                                                                                 |
| Same filename, different photos (e.g. `IMG_0001.jpg` from two phones) | Known limitation of filename dedup — second one is skipped and only album-linked. Accepted per your constraint; hash-based dedup can come later.                      |
| `-edited` variants / `(1)` suffixes                                   | Distinct filenames → imported as their own items.                                                                                                                     |
| Album folder split across two zips                                    | Destination paths are deterministic (`takeout/{batch}/Google Photos/{folder}/…`), album find-or-create is DB-backed ⇒ second zip's folder resolves to the same album. |
| Zip re-uploaded / job retried after partial run                       | Registration is DB-driven and idempotent: re-extracted files are recognized as known (link rows / items), re-deleted, nothing double-ingested.                        |
| Corrupt/failed ingest of a canonical file                             | Link stays pending; swept after 7 days by the hygiene delete (see §6).                                                                                                |
| 0-byte files, non-media files, sidecars                               | Skipped (same rules as Scan).                                                                                                                                         |
| Weird folder names (unicode, spaces, `_`)                             | Handled — album name comes from `metadata.json` `title` when present, else folder name as-is.                                                                         |
| Takeout imported twice (different export)                             | `takeout_id` differs → separate album sets, no merging with the older import. Same export re-imported → same `takeout_id` → albums reused.                            |

## 8. Integration tests (`crates/test_integration`)

New `tests/test_takeout.rs` + a fixture helper that builds takeout zips **in
memory** with the `zip` crate using existing files from `assets/media_dir`
(folder layout `Takeout/Google Photos/...`, small hand-written `metadata.json`s).
Upload them through the real tus endpoints (drive the fileloft router with
reqwest: `GET /upload/jwt` → `POST /api/files` with `Upload-Length` + metadata →
`PATCH` the bytes), then poll the jobs/DB until the queue drains — same pattern
as `test_onboarding.rs`.

Cases:

1. **Happy path**: one zip with `Album A` (metadata.json with title +
   description), `Photos from 2016`, a shared file. Assert: albums created with
   right title/description + `takeout_id` set; one media item per unique
   filename; shared file member of both albums; `Photos from 2016` album exists
   and contains its files; extracted files live under
   `takeout/{batch}/Google Photos/...` and DB `relative_path` matches fs.
2. **Chunked + fallback batch id**: two zips both named
   `takeout-renamed-manual.zip` (no matching pattern) → both get
   `batch_id = "takeout"`; an album spanning both zips is created **once** and
   contains files from both. Also assert a properly-named pair
   (`takeout-20260101T000000Z-1-001/002.zip`) shares batch
   `20260101T000000Z`.
3. **Pre-existing file dedup**: upload `IMG_x.jpg` through the normal dropzone
   first, then import a takeout containing `IMG_x.jpg` → media_item count
   unchanged; album contains the existing item.
4. **Intra-zip duplicate**: file in `Album A/` and `Photos from 2016/` in the
   same zip → single item, two album memberships, duplicate copy not on disk.
5. **Idempotent re-import**: upload the same zip twice → no new items, no
   duplicate album memberships, albums not recreated.
6. **Ignored files**: `*.json`, `*.html`, `shared_album_comments.json` produce no
   media items and no albums.

## 9. Implementation order

1. Migration `2026..._takeout.sql` → `scripts/recreate_db.ps1` → write new store
   methods + `JobType` → `scripts/update_sqlx.ps1` (compile check via sqlx).
2. `takeout_link_store.rs` + `consume_pending` hook in `ingest_metadata.rs`
   (+ thumbnail helper extraction). Small, independently testable.
3. `handle_web_upload.rs` detection/staging + `TakeoutImport` handler
   (extraction, registration, bulk enqueue) + watcher/scan exclusions.
4. Integration tests (they'll need the tus upload helper — worth writing first,
   since cases 1–6 all reuse it).
5. Frontend type + label mapping; optional album badge.
6. `just check` (fmt + clippy pedantic/nursery + tests) and a manual smoke test
   by uploading a couple of the real 10 GB zips.

## 10. Questions for you

1. **`-edited` variants**: import them as separate items (current plan), or skip
   them? (Google's edited version is a distinct file with its own filename, so
   they pass dedup either way.)
2. **Extracted layout** — `{user_media_folder}/takeout/{batch_id}/Google
   Photos/{Album}/file.jpg` OK, or do you prefer dropping the
   `Google Photos` component?
3. **Album sort**: takeout albums created with the default `DateAsc` — OK?
4. **Badge UI now or later?** I'd defer the frontend album badge; the data is
   there once we add `takeout_id` to the album API response.
