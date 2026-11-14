# MOBILE APP:

Support for Android 17 and up (API level 37).

### Mobile stack:

Initial thoughts on libraries. not set in stone. Change if something better is available.

* UI: Jetpack Compose + Compose Material 3.
* Image Loading: Coil 3 (`coil-compose` + `coil-network-okhttp`)
    * (Supports Compose natively, coroutines-first, customizable disk caches, and handles both `android.resource://`,
      `content://` MediaStore URIs, and HTTP/AVIF URLs).
* Local Database: Room
    * (with SQLite WAL mode for fast parallel reads/writes).
* Networking & Protobuf: Retrofit + OkHttp
    * with the official Java/Kotlin Protobuf lite runtime (compiled from proto schemas).
* Background Tasks: WorkManager (with CoroutineWorker) for auto-uploading and background synchronization.
* Video Playback: Jetpack Media3 (ExoPlayer) + `media3-datasource-okhttp`.
* Zoom/pan gestures: Telephoto (`zoomable-image-coil3`).

## CORE FEATURES

* auth (login, stay logged in)
* connectivity / sync indicator (on mobile bad internet is much more common; don't error on no internet, show subtle
  indicators like Connecting..., Syncing..., Offline)
* auto upload when a new photo/video is taken
* view timeline (including offline photos, in case no internet connection)
    * includes local vs cloud deduplication
    * show badge on timeline for cloud / local / uploading
    * scrub through timeline with scrubber on the right edge with floating date pill (similar to web frontend)
* view photo, pinch/double tap to zoom, share, add to album. generally native gallery gestures & interactions
* smooth UI feel, quick app start
* browse/view albums
* manually upload (TUS resumable upload, backend is already there)
* folder selection for what to backup. Default to DCIM/* (excluding Screenshots), optionally let user add more
  folders
* cache timeline / other responses so opening app can be instant
* view timeline items offline
    * bounded LRU disk cache for cloud thumbs?
    * For photos physically on the phone, load directly from Android’s local thumbs instead of querying the server.
* delete action, consider how to best do this, there's delete from device, delete from server, and soft delete (db sets
  deleted=true)
* in media-viewer
    * swipe up for info/gps/date pane
    * swipe down to return back to timeline (and sync timeline scroll position to viewed item)
* initially, themeing based on dynamic color (built into android). dynamicLightColorScheme / dynamicDarkColorScheme.
  LocalSemanticColors for sync/status badges

## LATER ON

* "Free up space" on phone button, deletes locally from phone from media items where it's detected it's already on
  server
* "cast" button (note: API 37 requires `ACCESS_LOCAL_NETWORK` permission for local discovery)
* semantic search (backend is already there), wouldnt need offline items to be searchable
* map view
* daily cards on front page (similar to cards in google photos), with notifications. Things like "On this day (1 year
  ago)", or "Cuddles and purrs" (cat pics).
* people page, where clustered faces are shown and can be labeled, and face clusters can be joined together.
* bin/trash, place where deleted items are, where they can be permanently deleted or restored to library.
* settings page - mostly app settings. on frontend this is theme settings, UI settings, concurrent uploads limit, album
  backup management, and missing media list (items that exist on DB but not on disk, can be (mass) deleted from db
  there). This is not all required for mobile. Mobile is more user focused rather that server management focused.
  Settings can include only sync on wifi, only while charging, not when roaming.
* User-friendly upload status UI
* set as wallpaper/lock screen option
* share photo to Ruurd Photos app to upload it (receive share intent)
* timeline
    * drag to select in timeline
        * long press to enter selection mode, the drag to select many items quickly.
    * pinch to change grid density
* settings for theme -> switch to materialkolor theme engine

## BONUS, NOT REQUIRED

* motion photo viewer
* search offline items as well (may need design considerations early on, depens on added complexity / drawbacks)
* 3d panorama viewer
* explore page (backend endpoints are there already, just about showing the data. Some tables, histograms in nice UI,
  top visited places), includes location page where all photos/videos taken in that location are shown.
* biometric lock for app? optionally of course

## ADMIN STUFF

even lower prio, dont want this in the app

* activity view, where user can see how the backend is progressing during ingest
* admin page - see background job queue, manage user accounts on server
* manage storage - see how full the server disk is, and get possibility to clear up storage by deleting large items

---

# Phased Implementation of CORE Features

decisions:

* For cloud-only photos, load the WebP/AVIF thumbnail from GET /thumbnails/{id}/{size}p.avif (no auth) or
  `/api/photos/{id}/thumbnail?size={size}` when `hasThumbnails == false`, and let Coil’s bounded disk cache (e.g. 500 MB
  LRU disk cache) cache what the user actually scrolls past.
* grid: justified horizontal row layout

implement notes

* Compose LazyColumn Performance:
    * Timeline scrolling will stutter if items perform heavy formatting on the main thread. Ensure dates, timestamps,
      and URLs are pre-computed in Room or mapped once off the main thread, never inside the Compose `@Composable` item
      body. Pass stable Lambdas/IDs.

## Phase 1: Foundation & Read-Only Timeline

* Set up Retrofit + OkHttp configured with a persistent `CookieJar` to handle and persist cookie-based session auth
  across app launches.
* Handle expiring tokens safely via a mutex-guarded OkHttp `Authenticator` (no concurrent `/api/auth/refresh` races).
* Build `AuthScreen` (custom server URL input validated via `GET /api/health`, email/password login).
* Configure Coil 3 (`coil-network-okhttp` sharing `OkHttpClient`, 500 MB LRU disk cache, and custom
  `MediaStoreThumbnailFetcher` scoped to a `MediaStoreThumbnail` wrapper type calling `ContentResolver.loadThumbnail`
  with coroutine `CancellationSignal`—leaving raw `content://` URIs untouched for Telephoto full-res/subsampling and
  falling back to cloud thumbnails on `SecurityException`).
* Add `NetworkMonitor` (`ConnectivityManager.NetworkCallback`) and subtle Offline / Syncing status indicator so missing
  internet degrades gracefully without errors.
* Add dedicated compact timeline sync endpoint on backend (`GET /api/mobile/timeline/sync?since=...`) returning
  `CompactTimelineItem` protobufs (including `id`, `sort_timestamp`, `ratio`, `is_video`, `has_thumbnails`,
  `size_bytes`, `duration_ms`, `timezone_offset_minutes`) plus `is_full_snapshot`, `deleted_media_item` tombstone
  trigger (without cascading FK to `app_user`), and `(user_id, updated_at)` index.
* Compile proto schemas (`common.proto` / `mobile.proto`) via the Protobuf Gradle plugin to generate Kotlin/Java models.
* Implement repository sync: initial full sync and delta sync (using `since` timestamp with overlap window) directly
  into Room as the Single Source of Truth (non-unique `cloudId`, unique `localContentUri`, immutable surrogate `id`, and
  canonical `is_visible_in_timeline` invariant so duplicate local copies never crash or duplicate in the grid). When
  `is_full_snapshot == true`, diff existing Room `cloud_id`s against `syncedCloudIds` in Kotlin and run the
  cloud-deletion transition on `staleCloudIds`.
* Query local `MediaStore` (`IS_PENDING = 0 AND IS_TRASHED = 0 AND SIZE > 0`, handling `READ_MEDIA_IMAGES`,
  `READ_MEDIA_VIDEO`, and `READ_MEDIA_VISUAL_USER_SELECTED`) and merge into Room using two-pass deduplication, gating
  cursor-omission deletion strictly on full per-type permissions (`READ_MEDIA_IMAGES` / `READ_MEDIA_VIDEO`) plus an
  empty-cursor safety guard:
    * Fast pass: match `(size_bytes, ratio, timezone-aligned timestamp)` against `CLOUD_ONLY` Room records for instant,
      zero-hash `SYNCED` reconciliation without disk I/O.
    * Lazy crypto pass: defer SHA-256 hashing to when unmatched items are staged for upload/backup.
* Build the Compose timeline (`LazyColumn` of precomputed justified rows + `MonthHeaderUiItem`s) and the draggable
  right-edge scrubber with a floating date pill (`isScrubbing` image-load gating, instant `scrollToItem`, and thumb
  position derived from `estimatedOffsetTopDp / totalHeightDp`).

## Phase 2: Media Viewer & Gestures

* Implement fullscreen viewer with swipe-between-items via `HorizontalPager`.
* Integrate `telephoto` (`zoomable-image-coil3`) for double-tap and pinch-to-zoom gestures.
* Add native gallery vertical gestures: swipe down to dismiss back to the timeline (syncing timeline scroll position if
  the viewed item is out of viewport), and swipe up to expand the info/GPS/date details sheet (fetching full details via
  `GET /api/photos/{id}/item` when online and falling back to Room metadata offline).
* Integrate Media3 (`ExoPlayer` + `OkHttpDataSource.Factory`), attaching the single shared `ExoPlayer` surface only to
  the settled `HorizontalPager` page (`settledPage`) with thumbnail placeholder until the first frame renders:
    * Play `localContentUri` directly when `LOCAL_ONLY` or `SYNCED`.
    * For `CLOUD_ONLY`: if `hasThumbnails == false`, stream `/api/photos/{id}/video` (original quality, Range-enabled,
      requires auth). If `hasThumbnails == true`, check phone codec playability: let the user pick quality from
      available WebM qualities + original (`/api/photos/{id}/video`) when playable, or fall back to transcoded WebM
      qualities (`/thumbnails/{id}/480p.webm`, etc., no auth) when not natively playable.
* Add bottom action bar: Share (`Intent.ACTION_SEND`), Info panel toggle, and Delete actions.

## Phase 3: Backup Engine & Manual Upload

* Implement `sha256` column on backend `media_item` and `POST /api/upload/check-hashes` endpoint for Pass 2
  deduplication (returning all non-deleted matching `cloud_ids` per hash; Room prefers merging into an unlinked
  `CLOUD_ONLY` row if one matches, or otherwise links to the first returned `cloud_id` as `SYNCED` with
  `is_visible_in_timeline = 0` when an active timeline row for that `cloud_id` already exists).
* UI screen for backup settings / `DeviceFoldersScreen`: browse non-timeline device folders, toggle auto-backup, select
  source folders (default `DCIM/*` excluding Screenshots), set Wi-Fi / charging constraints, and allow manual upload
  (per-item action / picker).
* Room `upload_queue` table to track progress, errors, `UPLOADED_PENDING_SYNC` state (reconciled via
  `POST /api/upload/check-hashes` before applying `SyncTimelineUseCase` snapshots, with max-age/retry `FAILED`
  handling), and resumable state (keeping upload ticks out of `media_item` so timeline layout isn't recomputed during
  uploads).
* `UploadWorker` implementation leveraging a derived `OkHttpClient` (with `120s+` read/write timeouts and `404`/`410`
  stale URL recovery) for TUS chunked resumable uploads.
* Timeline badge overlays showing item status: syncing spinner, local-only icon, or cloud checkmark.

```
The Problem: Standard Android CoroutineWorker instances have a ~10-minute execution window, while Android 15–17 also
enforces a 6-hour/24h cap on `dataSync` Foreground Services and blocks starting them from the background.

Fix:
- Use TUS chunking (e.g., 5 MB chunks) and persist the TUS upload URL/UUID and offset in Room's `upload_queue` table
  after every chunk so interrupted runs resume seamlessly without re-uploading bytes.
- Run routine background auto-backups as normal constrained `WorkManager` batches (no `setForeground` needed).
- Reserve `setForeground(createForegroundInfo())` (guarded against `ForegroundServiceStartNotAllowedException`) or
  User-Initiated Data Transfer (UIDT) for explicit user-triggered "Back up now" / manual upload sessions started in the
  foreground.
```

## Phase 4: Albums, Management & Deletion

* Album browsing, viewing album items, creating new albums, and adding/removing items to/from albums.
* Drag-to-select and long-press multi-selection on the timeline grid.
* Unified deletion dialog: "Delete from Device", "Delete from Server" (soft delete / trash), and "Delete Everywhere".
    * Deleting a `SYNCED` item from the server sets `auto_backup_suppressed = true` when reverting the local row to
      `LOCAL_ONLY`, preventing auto-backup from immediately re-uploading it.
    * Deleting a local file with sibling local copies sharing the same `cloud_id` removes only that row (promoting a
      sibling to `is_visible_in_timeline = 1` if needed), rather than reverting the cloud item to `CLOUD_ONLY` while
      another local copy still exists.
    * On Android 11+ (API 30+), deleting media from MediaStore that belongs to other apps or was created before the app
      was installed requires user consent via the OS dialog (`MediaStore.createDeleteRequest`).
* "Free up space" feature: query items where `syncStatus == SYNCED` (across both canonical and secondary duplicate local
  rows), compute local savings, and trigger `MediaStore.createDeleteRequest` to purge local files.
