# MOBILE APP:

Support for Android 15 and up (API level 35).

### Mobile stack:

Initial thoughts on libraries. not set in stone. Change if something better is available.

* UI: Jetpack Compose + Compose Material 3.
* Image Loading: Coil 3 (`coil-compose` + `coil-network-okhttp`)
    * (Supports Compose natively, coroutines-first, customizable disk caches, and handles both `android.resource://`,
      `content://` MediaStore URIs, and HTTP/AVIF URLs).
* Local Database: Room
    * (with SQLite WAL mode for fast parallel reads/writes).
* Networking, JSON & Protobuf: Retrofit + OkHttp
    * with the official Java/Kotlin Protobuf lite runtime (compiled from proto schemas) and `kotlinx.serialization` for
      JSON endpoints.
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
    * long-press and drag-to-select on timeline grid for multi-selection
* view photo & video, pinch/double tap to zoom, share, copy to clipboard, download to Downloads, rotate, reprocess, add
  to album. generally native gallery gestures & interactions
* smooth UI feel, quick app start
* browse/view albums
* manually upload (TUS resumable upload, backend is already there)
* folder selection for what to backup. Default to `DCIM/*` (excluding Screenshots/Screen recordings), optionally let
  user add more folders
* cache timeline / other responses so opening app can be instant
* view timeline items offline
    * bounded LRU disk cache for cloud thumbs (1 GB in Coil `image_cache`) + dedicated LRU cache (`full_res_cache`, ~
      500 MB) for full-res viewer/share/download files
    * for photos physically on the phone, load directly from Android's local thumbs instead of querying the server
* delete action: "Delete from Device", "Delete from Server" (soft delete / trash), and "Delete Everywhere"
* "Free up space" on phone button: deletes local copies of items already backed up (`SYNCED`) on the server
* in media-viewer
    * swipe up for info/gps/date pane (also reachable via "About" in top-right menu)
    * swipe down to return back to timeline (and sync timeline scroll position to viewed item on dismiss)
    * bottom pill actions: "Share" (always shares actual file), "Add to" (album), "Bin"
    * top-right menu: Download (for `CLOUD_ONLY`), About, Copy (photos only), Rotate clockwise, Reprocess, video
      Playback speed & video loop toggle
* initially, theming based on dynamic color (built into Android): `dynamicLightColorScheme` / `dynamicDarkColorScheme`
  plus `LocalSemanticColors` for sync/status badges

## LATER ON

* "cast" button (note: API 37 requires `ACCESS_LOCAL_NETWORK` permission for local discovery)
* semantic search (backend is already there), wouldn't need offline items to be searchable
* map view (and embedded mini-map in media info sheet)
* edit media date/time & GPS location (`PUT /api/photos/{id}/item`)
* daily cards on front page (similar to cards in Google Photos), with notifications. Things like "On this day (1 year
  ago)", or "Cuddles and purrs" (cat pics).
* people page, where clustered faces are shown and can be labeled, and face clusters can be joined together.
* bin/trash, place where deleted items are, where they can be permanently deleted or restored to library.
* settings page - mostly app settings. On web frontend this is theme settings, UI settings, concurrent uploads limit,
  album backup management, and missing media list (items that exist on DB but not on disk, can be mass deleted from DB
  there). This is not all required for mobile. Mobile is more user focused rather than server management focused.
  Settings can include only sync on Wi-Fi, only while charging, not when roaming.
* User-friendly upload status UI
* set as wallpaper/lock screen option
* share photo to Ruurd Photos app to upload it (receive share intent)
* pinch to change grid density in timeline
* settings for theme -> switch to materialkolor theme engine

## BONUS, NOT REQUIRED

* motion photo viewer
* search offline items as well (may need design considerations early on, depends on added complexity / drawbacks)
* 3d panorama viewer
* explore page (backend endpoints are there already, just about showing the data. Some tables, histograms in nice UI,
  top visited places), includes location page where all photos/videos taken in that location are shown.
* biometric lock for app? optionally of course

## ADMIN STUFF

even lower prio, don't want this in the app

* activity view, where user can see how the backend is progressing during ingest
* admin page - see background job queue, manage user accounts on server
* manage storage - see how full the server disk is, and get possibility to clear up storage by deleting large items

---

# Phased Implementation of CORE Features

decisions:

* For cloud-only photos, load the WebP/AVIF thumbnail from `GET /thumbnails/{id}/{size}p.avif` (no auth) or
  `/api/photos/{id}/thumbnail?size={size}` when `hasThumbnails == false`, and let Coil's bounded disk cache (1 GB
  LRU disk cache) cache what the user actually scrolls past. Keep full-resolution `/api/photos/{id}/download` files
  in a separate ~500 MB `cacheDir/full_res_cache/` LRU directory so they never evict grid thumbnails and can be
  reused by Share, Copy, and Download.
* grid: justified horizontal row layout

implement notes

* Compose `LazyColumn` Performance:
    * Timeline scrolling will stutter if items perform heavy formatting on the main thread. Ensure dates, timestamps,
      and URLs are pre-computed in Room or mapped once off the main thread, never inside the Compose `@Composable` item
      body. Pass stable lambdas/IDs.

---

## Phase 0: Backend Changes & Proto Refactor

* [✅ Done] **Database migration(s) (`server/migrations`):**
    * Add `deleted_media_item` tombstone table (`media_item_id`, `user_id`, `deleted_at`) + `AFTER DELETE ON media_item`
      trigger (do **not** add an `ON DELETE CASCADE` FK to `app_user`), plus index on `(user_id, deleted_at)`.
    * Add composite index `media_item (user_id, updated_at)` for fast delta sync queries.
* [✅ Done] **Audit `updated_at = now()` across `common_services`:**
    * Ensure every `UPDATE media_item` path (soft-delete/restore in trash/removal, `MediaItemStore::upsert`,
      missing/unmark paths, metadata edits) updates `updated_at = now()`.
* [✅ Done] **Scheduled tombstone pruning (`CleanDB job handler`):**
    * Periodically prune `deleted_media_item` rows older than retention (e.g. 30–90 days).
* [✅ Done] **Refactor protobuf schemas:**
    * [✅ Done] Split `timeline.proto` into `common.proto`, `web.proto`, and `mobile.proto` (updating
      `common_types/build.rs` and
      `web/package.json` `proto:gen`).
    * Define `TimelineSyncResponse` and `CompactTimelineItem` (`id`, `sort_timestamp`, `ratio`, `is_video`,
      `has_thumbnails`, `size_bytes`, `duration_ms`, `timezone_offset_minutes`).
* [✅ Done] **Compact sync endpoint (`GET /api/timeline/sync?since=...`):**
    * Return `Protobuf<TimelineSyncResponse>` with `upserted` (joining `media_features` for `size_bytes` and deriving
      `timezone_offset_minutes`), `deleted_ids` (union of soft-deleted `media_item` and `deleted_media_item`
      tombstones),
      `sync_timestamp`, and `is_full_snapshot` (when `since` is omitted or older than tombstone retention).
    * Verify `CompressionLayer` in `server.rs` gzip-compresses the protobuf response when `Accept-Encoding: gzip` is
      sent.
* [✅ Done] **Hash check endpoint:**
    * Implement `POST /api/upload/check-hashes` querying `media_item.hash` (BLAKE3 lowercase hex, indexed by
      `idx_media_item_user_hash`) and returning all non-deleted matching `cloud_ids` per hash.
* [✅ Done] **Verification:**
    * Add `test_integration` tests for full snapshot, delta upserts, soft-delete/restore, hard-delete tombstones,
      retention fallback, and `check-hashes`;

---

## Phase 1: Foundation & Read-Only Timeline

### 1A: App Bootstrap, Auth & Cloud Sync

* [✅ Done] Set up Android project architecture (DI, Room database with SQLite WAL mode, M3 Dynamic Color +
  `LocalSemanticColors`,
  top-level navigation shell, and API 37 `network_security_config.xml` for local `http://` dev testing).
* [✅ Done] Set up Retrofit + OkHttp with a persistent `CookieJar` to persist cookie-based session auth across app
  launches,
  dynamic base URL support, and `dispatcher.maxRequestsPerHost = 12–16`.
* [✅ Done] Handle expiring tokens safely via a mutex-guarded OkHttp `Authenticator` (preventing concurrent
  `/api/auth/refresh`
  races and 401 loops).
* [✅ Done] Build `AuthScreen` (custom server URL input validated via `GET /api/health`, email/password login).
* [✅ Done] Add `NetworkMonitor` (`ConnectivityManager.NetworkCallback`) and subtle Offline / Syncing status indicator so
  missing
  internet degrades gracefully without errors.
* [✅ Done] Compile `.proto` schemas (`common.proto` / `mobile.proto`) via the Protobuf Gradle plugin.
* [✅ Done] Configure Coil 3 (`coil-network-okhttp` sharing `OkHttpClient`, 500 MB LRU disk cache, hardware bitmaps, AVIF
  via
  `ImageDecoder`).
* [✅ Done] Implement `SyncTimelineUseCase`: initial full snapshot and delta sync (`since - 60_000L` overlap) directly
  into Room's
  `media_item` table (non-unique `cloudId`, unique `localContentUri`, immutable surrogate `id`). When
  `is_full_snapshot == true`, diff existing Room `cloud_id`s against `syncedCloudIds` in Kotlin and run the
  cloud-deletion transition on `staleCloudIds`.

### 1B: Local MediaStore & Pass 1 Deduplication

* [✅ Done] Request `READ_MEDIA_IMAGES` and `READ_MEDIA_VIDEO`, and handle Android 14+ partial access
  (`READ_MEDIA_VISUAL_USER_SELECTED`) cleanly.
* [✅ Done] Add `DeviceFolderEntity` + default discovery rules (`DCIM/*` excluding `DCIM/Screenshots` and `DCIM/Screen*`
  get
  `showInTimeline = true`, `backupEnabled = true`; all other folders default to `false`) and denormalized
  `is_visible_in_timeline` on `MediaEntity`.
* [✅ Done] Implement `ScanLocalMediaUseCase`: query `MediaStore` (`IS_PENDING = 0 AND IS_TRASHED = 0 AND SIZE > 0`,
  swapping
  dimensions for `90`/`270` orientation, falling back from `DATE_TAKEN` to `DATE_MODIFIED * 1000L`), gating
  cursor-omission deletion strictly on full per-type permissions plus an empty-cursor safety guard.
* [✅ Done] Run Pass 1 zero-hash deduplication against `CLOUD_ONLY` Room rows matching `size_bytes`, aspect `ratio`, and
  exact or
  15-minute timezone-aligned timestamps (`±2s` within `±24h`) to flip matches to `SYNCED` with zero disk I/O.
* [✅ Done] Register custom `MediaStoreThumbnailFetcher` scoped to a `MediaStoreThumbnail` wrapper type calling
  `ContentResolver.loadThumbnail` with coroutine `CancellationSignal`, `SecurityException` cloud fallback, and
  subsampled `ImageDecoder` / `VideoFrameDecoder` fallbacks—leaving raw `content://` URIs untouched for Phase 2
  Telephoto.

### 1C: Justified Timeline UI & Fast Scrubber

* [✅ Done] Implement 2-stage Room timeline queries over the composite index `(is_visible_in_timeline, year_month DESC,
  sort_timestamp DESC, id DESC)`: Stage 1 (top 2 `year_month`s for fast cold-start LCP) and Stage 2 (full library),
  with `.debounce(150)` on background `Flow` updates.
* [✅ Done] Build `CalculateTimelineLayoutUseCase` on `Dispatchers.Default` (`@Immutable` UI models, `ImmutableList`,
  pixel-snapped item widths, `rowHeightDp * density` thumbnail ladder capped at `720`/`480`, and `TimelineLayoutIndex`).
* [✅ Done] Build `TimelineScreen`: Compose `LazyColumn` of typed `MonthHeaderUiItem` and `TimelineRowUiItem`s (no
  `crossfade` in
  grid cells) plus draggable right-edge `TimelineScrubber` (`Modifier.systemGestureExclusion()`, instant `scrollToItem`,
  thumb tracking via `estimatedOffsetTopDp / totalHeightDp`, `isScrubbing` image-load gating, and floating
  `DateOverlay`).

---

## Phase 2: Media Viewer & Gestures

*(Scope excludes motion photo video playback and 3D panorama viewers — panoramas and motion photos render as regular
still photos.)*

### 2A: Navigation Shell, Photo Viewer & Gestures (`Telephoto`)

* Set up `NavHost` (replacing/wrapping `MainScreen`'s manual destination switch) with a generic `ViewerSource`
  contract (`Timeline` in Phase 2, reusable for `DeviceFolder` in Phase 3 and `Album` in Phase 5).
* Wire `TimelineScreen` grid tile clicks to open fullscreen `MediaViewerScreen` on a true `#000000` edge-to-edge canvas
  with `HorizontalPager` anchored by `mediaId` (so live Room updates never jump `currentPage` to a different item).
* Integrate `me.saket.telephoto:zoomable-image-coil3` (`ZoomableAsyncImage`) with double-tap, pinch-to-zoom, and
  pan/Ultra HDR support:
    * Lock horizontal paging (`userScrollEnabled = !isCurrentPageZoomed`) when zoomed in, and reset zoom when swiping
      off-page.
    * For `localContentUri != null`: pass the raw `content://` `Uri` (with grid placeholder cache key), falling back to
      cloud `1440p` on `SecurityException` (revoked partial access) or `FileNotFoundException`.
    * For `CLOUD_ONLY` items (and panoramas): show cached grid thumbnail → `1440p` (
      `hasThumbnails ? /thumbnails/{id}/1440p.avif : /api/photos/{id}/thumbnail?size=1440`), then fetch
      `/api/photos/{id}/download` on a short delay (or immediately on zoom) into a dedicated
      `cacheDir/full_res_cache/` (~250–300 MB LRU) and display via `file://` / `SubSamplingImageSource.file(...)` (
      preserving gainmaps without evicting Coil thumbnails; skip `/download` for non-decodable formats like TIFF/TGA).
* Build viewer chrome:
    * Tap to toggle floating chrome (`Back` button + title/counter pill, bottom floating pill with `Share` plus
      placeholder/disabled `Add to` and `Bin` buttons for Phases 4–5, and top-right overflow `⋮` menu).
    * Swipe down (gated on unzoomed state) to dismiss back to the timeline, syncing `TimelineScreen`'s `LazyColumn`
      scroll position via `findRowIndexForMedia` on dismiss if the final item is outside the viewport.

### 2B: JSON Details, Info Sheet, Share, Download, Copy, Rotate & Reprocess

* Add `kotlinx-serialization-json` + Retrofit `converter-kotlinx-serialization` and
  `androidx.exifinterface:exifinterface`.
* Implement `GET /api/photos/{id}/item` (`MediaItemWithAlbums` snake_case DTOs) + `MediaDetailsRepository` (in-memory
  LRU cache + `currentPage ± 1` neighbor prefetch) and `MediaRefreshCoordinator` (centralized delta sync + detail/image
  cache invalidation so Room `Flow` updates all active screens naturally).
* Build the swipe-up **Media Info Bottom Sheet** (`30.dp` rounded `surfaceContainerHigh`, also opened via **About** in
  the top-right overflow menu):
    * Display capture date/time in photo's local timezone (`taken_at_local` online;
      `sortTimestamp + timezoneOffsetMinutes` offline), filename, resolution/size, user caption, weather summary,
      camera/lens/exposure pills, albums list (read-only chips), and location string + coordinates with **"Open in Maps"
      ** (`geo:`) intent (no embedded map until MapLibre is added).
    * Offline fallback via `MediaStore` display metadata + `ExifInterface` (photos) / `MediaMetadataRetriever` (videos)
      when `localContentUri != null`.
* Implement **Share** (bottom pill — always shares the actual photo/video file via `Intent.ACTION_SEND`, using
  `localContentUri` directly when local or `full_res_cache` / `share_cache/` + `FileProvider` when `CLOUD_ONLY`).
* Implement **Copy** (top-right menu, photos only — hidden on videos; copies image URI to Android clipboard via
  `ClipData.newUri`, reusing `full_res_cache` + `FileProvider` when `CLOUD_ONLY`).
* Implement **Download** (top-right menu, visible for `CLOUD_ONLY` items only): streams `/api/photos/{id}/download` (
  reusing `full_res_cache` if cached) into `MediaStore` `Downloads` (`Environment.DIRECTORY_DOWNLOADS`,
  `IS_PENDING = 1 → 0`), then links `localContentUri` onto the `MediaEntity` (`syncStatus = SYNCED`,
  `isVisibleInTimeline = true`).
* Implement **Rotate clockwise** (top-right menu, photos only; 2.5s debounce + immediate flush on swipe/exit; disabled
  while an item is uploading in Phase 3's `upload_queue`; evicts Coil `mediastore:$uri` + `uri` cache keys):
    * `CLOUD_ONLY`: `PUT /api/photos/{oldId}/item` → swap `"cloud_$oldId"` → `"cloud_$newId"` in Room, update viewer's
      active `mediaId`, fetch `GET /api/photos/{newId}/download`.
    * `SYNCED`: request `MediaStore.createWriteRequest` *before* calling the server → `PUT /api/photos/{oldId}/item` →
      immediately persist `cloudId = newId` + swapped `ratio` + `hasThumbnails = false` in Room → download
      `GET /api/photos/{newId}/download` and overwrite `localContentUri` (keeping local bytes, `size_bytes`, and BLAKE3
      identical to the server without setting `autoBackupSuppressed`).
    * `LOCAL_ONLY` (JPEG/PNG/WebP): request `MediaStore.createWriteRequest` → update EXIF orientation via
      `ExifInterface` → update Room `ratio` & `sizeBytes`.
* Implement **Reprocess** (top-right menu when `cloudId != null`): call `POST /api/photos/{id}/reprocess`, invalidate
  `MediaDetailsRepository` cache, and trigger `MediaRefreshCoordinator` delta sync.

### 2C: Video Playback (`Media3` / `ExoPlayer`)

* Add `androidx.media3:media3-exoplayer`, `media3-datasource-okhttp`, and `media3-ui-compose` /
  `media3-ui-compose-material3`.
* Configure a single shared `ExoPlayer` wired with `OkHttpDataSource.Factory(okHttpClient)` (preserving session
  cookies & 401 refresh), lifecycle pause on `ON_STOP`, and persisted **Loop** (`REPEAT_MODE_ONE` / `REPEAT_MODE_OFF`) +
  **Playback speed** preferences.
* Attach `PlayerSurface` only to `pagerState.settledPage`, keeping the photo thumbnail rendered underneath until the
  first video frame renders (`onRenderedFirstFrame`).
* Implement source selection + fallbacks:
    * Play `localContentUri` directly when `LOCAL_ONLY` or `SYNCED` (falling back to cloud on `SecurityException` /
      `FileNotFoundException`).
    * For `CLOUD_ONLY`: if `hasThumbnails == false`, stream `/api/photos/{id}/video` (original quality, Range-enabled,
      requires auth). If `hasThumbnails == true`, default to **Original** (`/api/photos/{id}/video`) when on unmetered
      Wi-Fi without Data Saver and **`480p.webm`** otherwise; offer a quality picker (`Original`, `480p`, `144p` using
      `sourceHeight` from `/item`) and automatically fall back to `480p.webm` if `Original` hits a codec/container
      `PlaybackException`.
* Build floating video controls integrated with tap-to-toggle chrome (play/pause overlay, seek bar + buffered
  indicator + `tnum` duration/position, mute toggle, quality selector) and expose **Playback speed** (`0.25x`–`3.0x`
  selector) and **Loop video** toggle in the top-right overflow menu.

---

## Phase 3: Backup Engine, Device Folders & Manual Upload

* Build `DeviceFoldersScreen` ("On this device") & `BackupSettingsScreen`:
    * Browse all discovered folders (including non-timeline folders like Screenshots, Download, WhatsApp).
    * Toggle per-folder `backup_enabled` (which auto-enables `show_in_timeline`) and `show_in_timeline`.
    * Configure Wi-Fi / charging constraints and trigger manual upload (per-item action or "Back up now").
* Add Room `upload_queue` table (kept separate from `media_item` so upload progress ticks never re-trigger
  `CalculateTimelineLayoutUseCase`) to track progress, errors, TUS upload URL/offset, and `UPLOADED_PENDING_SYNC` state
  (reconciled via `POST /api/upload/check-hashes` before applying `SyncTimelineUseCase` snapshots, with max-age/retry
  `FAILED` handling). Also wire `upload_queue` active state into Phase 2B's Rotate guard so items actively uploading
  cannot be rotated mid-transfer.
* Implement Pass 2 lazy BLAKE3 hashing + `POST /api/upload/check-hashes` pre-upload deduplication:
    * Prefer merging into an unlinked `CLOUD_ONLY` row if one matches; otherwise link to the first returned `cloud_id`
      with `is_visible_in_timeline = 0` when a canonical visible row for that `cloud_id` already exists.
* Implement `UploadWorker` with a derived `OkHttpClient` (`120s+` read/write timeouts and `404`/`410` stale upload URL
  recovery) for 5 MB TUS chunked resumable uploads:
    * Run routine background auto-backups as normal constrained `WorkManager` batches (persisting TUS URL and offset in
      `upload_queue` after every chunk).
    * Reserve `setForeground(createForegroundInfo())` (guarded against `ForegroundServiceStartNotAllowedException`) or
      UIDT for explicit user-triggered "Back up now" / manual upload sessions started in the foreground.
* Wire `MediaStoreContentObserver` / `LocalScanWorker` (`TriggerContentUri` + periodic scan) → `EnqueueBackupUseCase`
  for `LOCAL_ONLY` items where `auto_backup_suppressed = false` and `device_folder.backup_enabled = 1`.
* Add timeline badge overlays showing item status: static local-only / cloud-only / synced indicators, plus live syncing
  spinner observed from `upload_queue`.

---

## Phase 4: Multi-Select, Unified Deletion & Free Up Space

* Implement long-press + drag-to-select multi-selection on the timeline grid with a contextual selection top bar.
* Wire the **Bin** button in `MediaViewerScreen`'s bottom pill and implement unified deletion dialog (usable from both
  `MediaViewerScreen` and multi-selection): "Delete from Device",
  "Delete from Server" (soft delete / trash), and "Delete Everywhere".
    * Deleting a `SYNCED` item from the server sets `auto_backup_suppressed = true` when reverting the local row to
      `LOCAL_ONLY`, preventing auto-backup from immediately re-uploading it.
    * Deleting a local file with sibling local copies sharing the same `cloud_id` removes only that row (promoting a
      sibling to `is_visible_in_timeline = 1` if needed), rather than reverting the cloud item to `CLOUD_ONLY` while
      another local copy still exists.
    * On Android 11+ (API 30+), delete local MediaStore items via OS consent (`MediaStore.createDeleteRequest`).
* Implement "Free up space": query items where `syncStatus == SYNCED` (across both canonical and secondary duplicate
  local rows), compute reclaimable local storage, and trigger `MediaStore.createDeleteRequest` to purge local files.

---

## Phase 5: Albums

* Add Room `AlbumEntity` / `AlbumDao`, `AlbumRepository`, and album API integration.
* Build `AlbumsScreen` (grid of album cards) and `AlbumDetailScreen` (reusing the Phase 1 justified horizontal row
  grid and Phase 2's `MediaViewerScreen` via `ViewerSource.Album`).
* Support creating new albums and wire the **Add to** button in `MediaViewerScreen`'s bottom pill as well as Phase 4's
  multi-select action bar.