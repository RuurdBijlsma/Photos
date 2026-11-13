# MOBILE APP:

Support for Android 17 and up (API level 37).

### Mobile stack:

Initial thoughts on libraries. not set in stone. Change if something better is available.

* UI: Jetpack Compose + Compose Material 3.
* Image Loading: Coil 3
    * (Supports Compose natively, coroutines-first, customizable disk caches, and handles both android.resource://,
      content:// MediaStore URIs, and HTTP/AVIF URLs).
* Local Database: Room
    * (with SQLite WAL mode for fast parallel reads/writes).
* Networking & Protobuf: Retrofit + OkHttp
    * with the official Java/Kotlin Protobuf lite runtime (compiled from proto schemas).
* Background Tasks: WorkManager (with CoroutineWorker) for auto-uploading and background synchronization.
* Video Playback: Jetpack Media3 (ExoPlayer) + `media3-datasource-okhttp`.
* Zoom/pan gestures: Telephoto (`zoomable-image-coil`).

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
* Add `NetworkMonitor` (`ConnectivityManager.NetworkCallback`) and subtle Offline / Syncing status indicator so missing
  internet degrades gracefully without errors.
* Add dedicated compact timeline sync endpoint on backend (`GET /api/mobile/timeline/sync?since=...`) returning
  `CompactTimelineItem` protobufs (including `id`, `sort_timestamp`, `ratio`, `is_video`, `has_thumbnails`,
  `size_bytes`, `duration_ms`, `timezone_offset_minutes`), plus `deleted_media_item` tombstone trigger (without cascading
  FK to `app_user`) and `(user_id, updated_at)` index.
* Compile proto schemas (`common.proto` / `mobile.proto`) via the Protobuf Gradle plugin to generate Kotlin/Java models.
* Implement repository sync: initial full sync and delta sync (using `since` timestamp with overlap window) directly
  into Room as the Single Source of Truth.
* Query local `MediaStore` (handling `READ_MEDIA_IMAGES`, `READ_MEDIA_VIDEO`, and `READ_MEDIA_VISUAL_USER_SELECTED`) and
  merge into Room using two-pass deduplication:
    * Fast pass: match `(size_bytes, ratio, timezone-aligned timestamp)` against Room records for instant, zero-hash
      `SYNCED` reconciliation without disk I/O.
    * Lazy crypto pass: defer SHA-256 hashing to when unmatched items are staged for upload/backup.
* Build the Compose timeline (`LazyColumn` of precomputed justified rows + `MonthHeaderUiItem`s) and the draggable
  right-edge scrubber with a floating date pill (`isScrubbing` image-load gating, instant `scrollToItem`).

## Phase 2: Media Viewer & Gestures

* Implement fullscreen viewer with swipe-between-items via `HorizontalPager`.
* Integrate `telephoto` / `zoomable` for double-tap and pinch-to-zoom gestures.
* Add native gallery vertical gestures: swipe down to dismiss back to the timeline (syncing timeline scroll position if
  the viewed item is out of viewport), and swipe up to expand the info/GPS/date details sheet.
* Integrate Media3 (`ExoPlayer` + `OkHttpDataSource.Factory`):
    * Play `localContentUri` directly when `LOCAL_ONLY` or `SYNCED`.
    * For `CLOUD_ONLY`: if `hasThumbnails == false`, stream `/api/photos/{id}/video` (original quality, Range-enabled,
      requires auth). If `hasThumbnails == true`, check phone codec playability: use `/api/photos/{id}/video` when
      playable, or fall back to `/thumbnails/{id}/480p.webm` (no auth) when not natively playable.
* Add bottom action bar: Share (`Intent.ACTION_SEND`), Info panel toggle, and Delete actions.

## Phase 3: Backup Engine & Manual Upload

* Implement `sha256` column on backend `media_item` and `POST /api/upload/check-hashes` endpoint for Pass 2
  deduplication.
* UI screen for backup settings / Device Folders: toggle auto-backup, select source folders (default `DCIM/*` excluding
  Screenshots, opt-in for WhatsApp, Downloads, etc.), and set Wi-Fi / charging constraints.
* Room `upload_queue` table to track progress, errors, `UPLOADED_PENDING_SYNC` state, and resumable state (keeping
  upload ticks out of `media_item` so timeline layout isn't recomputed during uploads).
* `UploadWorker` implementation leveraging OkHttp for TUS chunked resumable uploads.
* Timeline badge overlays showing item status: syncing spinner, local-only icon, or cloud checkmark.

```
The Problem: Standard Android CoroutineWorker instances have a strict 10-minute maximum execution window. If a user uploads a 4K 60fps video or a large batch on a typical home upload speed, Android's OS will kill the worker mid-stream.

Fix:
    Call setForeground(createForegroundInfo()) inside UploadWorker to promote it to a Foreground Service with an ongoing notification showing progress.
    Use TUS chunking (e.g., 5 MB chunks) and persist the TUS upload URL/UUID and offset in Room's upload_queue table after every chunk. If killed, the next run resumes seamlessly without re-uploading uploaded bytes.
```

## Phase 4: Albums, Management & Deletion

* Album browsing, viewing album items, and creating new albums.
* Drag-to-select and long-press multi-selection on the timeline grid.
* Unified deletion dialog: "Delete from Device", "Delete from Server" (soft delete / trash), and "Delete Everywhere".
    * On Android 11+ (API 30+), deleting media from MediaStore that belongs to other apps or was created before the app
      was installed requires user consent via the OS dialog (`MediaStore.createDeleteRequest`).
* "Free up space" feature: query items where `syncStatus == SYNCED`, compute local savings, and trigger
  `MediaStore.createDeleteRequest` to purge local files.
