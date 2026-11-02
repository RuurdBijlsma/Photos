# API runtime stall investigation

**Investigation date:** 2026-11-02  
**Conclusion:** The reproduced multi-second Tokio worker stalls coincide with `ClusterFaces` and `ClusterPhotos` jobs. In the confirmed run, ingest jobs had completed before the cluster jobs associated with the later stalls began.

## Introduction

Ruurd Photos serves its HTTP API and runs background jobs in the same `photos_app` process. The API and its worker scaler share that process's Tokio runtime. This investigation began after timeline scrolling produced thumbnail requests that waited for many seconds or were cancelled by the browser. The initial hypothesis was that thumbnail serving itself was slow or that the API runtime had stopped making progress.

To distinguish an application-runtime delay from a process-wide pause, a temporary watchdog recorded two heartbeats: one from a plain operating-system thread and one from a Tokio task. It also sampled per-worker Tokio busy time. Additional timestamps were added around worker initialization, metadata analysis, and thumbnail generation. Finally, job `started_at` and `finished_at` values from the local Postgres `jobs` table were compared with the watchdog timestamps.

In this report, “Tokio stall” refers to a gap in the Tokio task heartbeat while the OS-thread heartbeat continues. It does not mean that every Tokio worker stopped. The per-worker metrics show that one runtime worker was busy for nearly the duration of each observed gap.

## Findings

The process stayed alive during the stalls. The OS-thread heartbeat continued at approximately one-second intervals while the Tokio heartbeat had gaps of approximately 23–25 seconds. The per-worker runtime metrics attributed approximately 23–25 seconds of busy time to one Tokio worker for each gap; the other workers were largely idle during those intervals.

The job table gives the most direct correlation. The four stalls in the latest reproduction align with active cluster jobs:

| Watchdog time (Europe/Berlin) | Tokio drift | Busy Tokio worker | Job active during gap (UTC) | Job duration |
|---|---:|---:|---|---:|
| 21:12:54 | 24.514 s | worker 0: 25.279 s | `cluster_faces` #96059, 20:12:29.039–20:13:04.492 | 35.453 s |
| 21:14:50 | 23.692 s | worker 14: 24.397 s | `cluster_photos` #96147, 20:14:26.200–20:14:51.896 | 25.696 s |
| 21:16:00 | 24.272 s | worker 14: 25.114 s | `cluster_faces` #96166, 20:15:35.564–20:16:10.173 | 34.609 s |
| 21:16:34 | 23.054 s | worker 14: 23.660 s | `cluster_photos` #96165, 20:16:10.175–20:16:34.627 | 24.452 s |

The watchdog timestamps are local Europe/Berlin time. Database timestamps are UTC. The final column is the database interval from `started_at` to `finished_at`; job duration includes the full handler, including asynchronous work, and does not by itself measure CPU time.

For the two copied files in this reproduction, all three ingest stages had completed before the two cluster jobs listed last began:

| Job | Type | Started (UTC) | Finished (UTC) | Duration |
|---:|---|---|---|---:|
| 96173 | `ingest_metadata` | 20:15:32.224 | 20:15:32.266 | 0.042 s |
| 96170 | `ingest_metadata` | 20:15:32.268 | 20:15:32.311 | 0.044 s |
| 96174 | `ingest_thumbnails` | 20:15:35.206 | 20:15:35.241 | 0.035 s |
| 96171 | `ingest_thumbnails` | 20:15:35.255 | 20:15:35.279 | 0.024 s |
| 96175 | `ingest_analysis` | 20:15:35.324 | 20:15:35.466 | 0.143 s |
| 96172 | `ingest_analysis` | 20:15:35.469 | 20:15:35.563 | 0.094 s |

The worker log also shows metadata analysis beginning at 20:15:32 UTC and thumbnail generation at 20:15:35 UTC. These operations completed before the cluster jobs associated with the 21:16 gaps. Thus those gaps cannot be attributed to the three ingest stages for these two files.

Earlier in the same log, metadata analysis and thumbnail generation overlapped with some cluster activity. That overlap alone could not distinguish their contribution. The database timestamps from the later reproduction resolve that ambiguity for its 21:16 stalls: the ingest jobs were done, while the cluster jobs were active.

## Code evidence

Both cluster handlers perform CPU work synchronously while running as async worker jobs:

- [`cluster_faces.rs`](../server/crates/apps/worker/src/handlers/cluster_faces.rs) calls `clustering::run_hdbscan(...)` at line 380 from its async handler.
- [`cluster_photos.rs`](../server/crates/apps/worker/src/handlers/cluster_photos.rs) calls the same synchronous clustering function at line 449 from its async handler. Its tag-embedding phase also calls the synchronous `text_embedder.embed_texts(...)` at line 139.
- [`common/clustering.rs`](../server/crates/apps/worker/src/handlers/common/clustering.rs) defines `run_hdbscan` as a synchronous function. It calls `clusterer.cluster()` directly at line 56.

The cluster handler call sites do not wrap these operations in `tokio::task::spawn_blocking`. The worker scaler runs worker tasks in the `photos_app` process on the same Tokio runtime as the API. The worker profiles exclude some job types, but the `ml` profile does not exclude `ClusterFaces` or `ClusterPhotos`; the `thumbnails` and `light` profiles do exclude `ClusterPhotos`.

## Root cause

The observed API-runtime stalls are caused by CPU-bound work in the `ClusterFaces` and `ClusterPhotos` job handlers running synchronously on Tokio runtime worker threads. The watchdog shows a Tokio worker occupied for nearly the duration of each stall, and the job table shows the corresponding cluster job active over the same interval. The reproduced 21:16 stalls occur after the copied files’ metadata, thumbnail, and analysis jobs have all completed.

The evidence establishes cluster work as the cause of the confirmed stalls. It does not establish that every prior slow response had the same cause, or assign every millisecond of each cluster job’s database duration to synchronous CPU work.

## Evidence sources

- `C:\Users\Ruurd\Documents\ruurd_photos_app_data\logs\stall.log` — OS and Tokio heartbeats, drift, and per-worker busy-time deltas.
- `C:\Users\Ruurd\Documents\ruurd_photos_app_data\logs\photos.2026-11-02.log` — worker startup and ingest-stage markers from the instrumented run.
- Read-only query of the local `photos` database `jobs` table — job type, status, `started_at`, and `finished_at` for the cluster and ingest job IDs listed above.
