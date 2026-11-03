# Google Photos Takeout import investigation

**Date:** 2026-11-03  
**Status:** The storage mismatch and proxy timeout are established from code and supplied logs. The production job's final database status was not obtained.

## Summary

The production symptoms have three related parts:

1. The current worker expands all uploaded ZIPs under the app-data cache before it creates albums or queues media ingestion. The supplied 15 ZIPs contain about 148.4 GB of compressed data and about 148.37 GB of expanded entries. With only 14 GB free on app data after upload, this implementation cannot complete extraction there.
2. The supplied production Nginx log records a `504` for `POST /api/upload/takeout/start` after it timed out waiting for the backend response. The request can take a long time because the API route repairs completed TUS uploads synchronously before it enqueues the batch.
3. The `Repaired interrupted tus finalization` warnings confirm that at least two uploads had complete TUS offsets but incomplete assembled files. The warning is emitted while the start route is inspecting uploads; it does not establish that the batch was enqueued.

The most likely explanation for no media appearing is the app-data capacity mismatch if the batch worker ran. The 504 explains why the browser did not receive a successful start response, but the supplied output does not establish whether the backend continued and inserted a job after the proxy returned 504.

## Evidence gathered

### Local Takeout archive size

The 15 ZIPs in `C:\Users\Ruurd\Desktop\takeout\raw_download` were inspected through their ZIP central directories; the media payloads were not extracted for this measurement.

| Measure | Bytes | Approximate decimal size |
|---|---:|---:|
| ZIP files | 148,401,638,992 | 148.4 GB |
| Sum of ZIP entry sizes | 148,369,580,035 | 148.37 GB |

The archives are already compressed by Google Takeout and their expanded entry sizes are nearly the same as their ZIP sizes. There are approximately 78,867 entries across the 15 archives, including media and JSON metadata.

The production warnings named upload sizes of 9,503,032,427 bytes and 9,793,358,820 bytes. Those sizes exactly match ZIPs 004 and 011 in this local set. This ties the two warnings to real archives in the upload batch.

### Local development run

The local log shows the expected path:

- `POST /api/upload/takeout/start` returned HTTP 200 in 236 ms.
- A worker picked up `IngestTakeoutBatch` six seconds later.
- The worker scanned 24,713 unique media filenames.
- It linked 3,678 already-ingested media items to albums and enqueued ingest for 21,035 new items.
- It logged the batch as finished, after which regular ingest jobs began.

This confirms that the Takeout semantics work on the local dataset when the worker can extract and process it.

### Production TUS finalization and proxy logs

The production excerpts include:

- Backend PATCH request latencies of 61,460 ms and 64,433 ms.
- A repair warning at 10:46 UTC for the 9,503,032,427-byte upload.
- A repair warning at 17:30 UTC for the 9,793,358,820-byte upload.
- Nginx's `photos_web` log reporting `upstream timed out ... while reading response header from upstream` for `POST /api/upload/takeout/start`, followed by an HTTP 504 at 17:31:19 UTC.

The application web image uses Nginx to proxy `/api` to the backend in [web/nginx.conf](../web/nginx.conf). The API location does not set an explicit `proxy_read_timeout`, so it uses Nginx's configured default unless overridden elsewhere. The official [NGINX proxy module documentation](https://nginx.org/en/docs/http/ngx_http_proxy_module.html) documents a 60-second default for `proxy_read_timeout`.

The supplied Nginx log proves that the **start request** timed out at the proxy. The 61–64 second PATCH durations are consistent with long TUS finalization requests, but the supplied 504 line is for `/api/upload/takeout/start`, not a PATCH. Therefore the log does not by itself prove that a proxy timeout caused either incomplete finalization.

The repair warning itself is meaningful: the application found a completed TUS upload whose final file needed reconstruction from its part files. The warning is logged before the API route finishes collecting upload IDs and inserting the batch job. It is not a batch-start confirmation.

### Why the current worker needs more app-data space

The current flow is implemented in [ingest_takeout_batch.rs](../server/crates/apps/worker/src/handlers/ingest_takeout_batch.rs):

1. It creates an extraction path under `cache_root/takeout_tmp/<job id>`.
2. `cache_root` is derived from `app_data_root/cache` in [settings.rs](../server/crates/libs/app_state/src/settings.rs).
3. It extracts every ZIP into that directory.
4. Only after all ZIPs have been extracted does it read album folders, create/reuse albums, index sidecars and media, perform filename deduplication, move new media into the media root, and enqueue ingest jobs.
5. It removes the extraction directory and TUS ZIPs only at successful completion.

Therefore, during the current implementation's peak, app data holds the uploaded ZIPs plus nearly the same amount of expanded data: roughly **297 GB attributable to this import**, before other app-data use and transient repair space. Since the ZIPs were already present when the user reported 14 GB free, the current flow would need about another 148 GB of free space for extraction, plus operational headroom.

If extraction fails, the handler returns before album creation. Its cleanup is at the successful end of the job, so a failed attempt may leave a partial extraction directory that continues consuming app-data space.

### What later ingest jobs consume

The Takeout batch does use the extracted tree today, but later ingestion does not depend on that tree:

- The batch reads album `metadata.json` and sidecar JSON to derive album metadata and optional fallback timestamps.
- It places new media files in the user's managed media folder and enqueues normal ingest jobs.
- `IngestMetadata` reads the media file from `media_root/relative_path` and reads the optional `fallback_taken_at` value from the job payload. It does not read `takeout_tmp` or a Takeout sidecar file.
- Existing media items are linked to albums without importing a duplicate file.

This means the expanded directory is an implementation convenience for the batch parser, not a downstream ingest requirement.

## What is confirmed and what remains unknown

### Confirmed

- The supplied ZIP set expands to about 148.37 GB.
- The production app-data volume had only 14 GB free after the ZIP uploads, according to the user's report.
- The current worker extracts all ZIPs under the app-data cache before it creates albums.
- Two completed uploads needed finalization repair.
- The production web proxy returned HTTP 504 for the Takeout start request.

### Not confirmed

The production database status and app-data free space after the import attempt were not obtained. In the pasted server output, the `docker compose exec` commands for the database and backend returned `no configuration file provided: not found`; they did not run. The `photos_web` container log command did run and showed the 504.

Consequently, the available evidence does not establish whether the backend inserted a batch job after the proxy timed out, whether a worker ran and failed while extracting, or whether the request stopped before enqueueing. A database query from the correct Compose project directory and the backend job logs would distinguish those cases.

## Conclusion

The current import strategy cannot handle this batch on the reported app-data free space. The most direct design improvement is to inspect the ZIP entries without expanding the entire archive set, then stream only new media files to the media volume. Separately, the API start route should return promptly rather than repairing large files synchronously, and the web proxy timeout should accommodate long TUS PATCH requests.
