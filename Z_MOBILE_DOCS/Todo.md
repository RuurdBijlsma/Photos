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
* view photo & video, pinch/double tap to zoom, share, copy to clipboard, rotate, reprocess, add to album. generally
  native gallery gestures & interactions
* smooth UI feel, quick app start
* browse/view albums
* manually upload (TUS resumable upload, backend is already there)
* folder selection for what to backup. Default to `DCIM/*` (excluding Screenshots/Screen recordings), optionally let
  user add more folders
* cache timeline / other responses so opening app can be instant
* view timeline items offline
    * bounded LRU disk cache for cloud thumbs (1 GB in Coil `image_cache`) + dedicated LRU cache (`full_res_cache`, ~
      500 MB) for full-res viewer/share/copy files
    * for photos physically on the phone, load directly from Android's local thumbs instead of querying the server
* delete action: "Delete from Device", "Delete from Server" (soft delete / trash), and "Delete Everywhere"
* "Free up space" on phone button: deletes local copies of items already backed up (`SYNCED`) on the server
* in media-viewer
    * swipe up for info/gps/date pane (also reachable via "About" in top-right menu)
    * swipe down to return back to timeline (timeline stays visible underneath during drag; syncs timeline scroll
      position to viewed item on dismiss)
    * bottom pill actions: "Share" (always shares actual file), "Add to" (album), "Bin"
    * top-right menu: About, Copy (photos only), Rotate clockwise, Reprocess, video Playback speed & video loop toggle
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
  reused by Share and Copy.
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

* Add `org.jetbrains.kotlin.plugin.serialization` + `kotlinx-serialization-json` (for type-safe `@Serializable` routes)
  and set up `NavHost` (inside `SessionViewModelStoreHolder` wrapping `MainScreen`, preserving tab/scroll state across
  `Photos`/`Collections`/`Explore`/`Search` and keeping `timeline_list` / `timeline_scrubber` test tags intact) with a
  generic `ViewerSource` contract (`Timeline` in Phase 2, reusable for `DeviceFolder` in Phase 3 and `Album` in
  Phase 5).
* Host `MediaViewerScreen` in a `SharedTransitionLayout` / translucent overlay above `NavHost` (rather than an opaque
  `NavHost` `composable` replacement) so `TimelineScreen` stays live and visible underneath during shared-element
  transitions and interactive swipe-down dismiss.
* Wire `TimelineScreen` grid tile clicks (passing surrogate `MediaEntity.id`, not `cloudId`) to open fullscreen
  `MediaViewerScreen` on a `#000000` edge-to-edge canvas (forcing light system bar icons via
  `isAppearanceLightStatusBars = false` / `isAppearanceLightNavigationBars = false`) with `HorizontalPager` anchored by
  `mediaId` (so live Room Stage 2 loads, sync prepends, or rotations never jump `currentPage` to a different item).
    * Load `mediaRepository.getById(initialMediaId)` directly for frame-1 render while `ObserveTimelineUseCase` loads
      the full pager list, and guard cleanly if a background sync deletes the active item.
* Integrate `me.saket.telephoto:zoomable-image-coil3` (`ZoomableAsyncImage`) with double-tap, pinch-to-zoom, and
  pan/Ultra HDR support:
    * Key `rememberZoomableState` / `rememberZoomableImageState` on `mediaId` (not image URL), lock `HorizontalPager`
      (`userScrollEnabled` checks only the **active page's** `zoomFraction <= 0.01f`) when zoomed in, and reset zoom
      when swiping off-page.
    * Resolve/capture the actual `MemoryCache.Key` (from `imageLoader.memoryCache?.keys` matching `"mediastore:$uri"` /
      `1440p` URL, or via `onSuccess`) for `placeholderMemoryCacheKey` instead of passing a bare `String` (since Coil 3
      includes `coil#size` in `extras` when `isSampled = true`).
    * For `localContentUri != null`: pass the raw `content://` `Uri` with the resolved `MemoryCache.Key` placeholder,
      falling back to cloud `1440p` on `SecurityException` (revoked partial access) or `FileNotFoundException`.
    * For `CLOUD_ONLY` **photos** (`!isVideo`, including panoramas): show cached grid thumbnail → `1440p` (
      `hasThumbnails ? /thumbnails/{id}/1440p.avif : /api/photos/{id}/thumbnail?size=1440`), wait for `/item` so
      `mime_type` and `missing_since == null` are known, then fetch `/api/photos/{id}/download` on a short delay (or
      immediately on zoom) into a dedicated `cacheDir/full_res_cache/` (~500 MB LRU, deduplicated per `cloudId` with
      temp-file atomic rename) and pass the `file://` URI (with `diskCachePolicy(CachePolicy.DISABLED)` and `1440p`
      `MemoryCache.Key` placeholder) to `ZoomableAsyncImage` (so JPEGs/PNGs/WebPs subsample with gainmaps while full-res
      AVIF/HEIC fall back to Coil without `BitmapRegionDecoder` failing; skip `/download` for videos, TIFF/TGA, or
      `missing_since != null`).
* Prefetch `1440p` Coil thumbnails for `currentPage ± 1` (for photo neighbors) alongside `MediaDetailsRepository`
  metadata prefetch.
* Build viewer chrome & gestures:
    * Tap to toggle floating chrome (and hide/restore system bars cleanly, restoring both `systemBars()` visibility and
      `isAppearanceLightStatusBars` / `isAppearanceLightNavigationBars` inside `DisposableEffect` on viewer exit):
      `Back` button + title/counter pill, bottom floating pill with `Share` plus placeholder/disabled `Add to` and `Bin`
      buttons for Phases 4–5, and top-right overflow `⋮` menu.
    * Single-pointer swipe down / swipe up arbitration on both photo and video pages (`pointerCount == 1`,
      `zoomFraction <= 0.01f`, `|dy| > |dx|` after touch slop so horizontal/diagonal swipes go cleanly to
      `HorizontalPager` and pinch gestures aren't stolen): swipe down interactively drags/shrinks the viewer over the
      visible timeline and dismisses, syncing `TimelineScreen`'s `LazyColumn` scroll position via
      `findRowIndexForMedia(activeMediaId)` if the final item is outside the viewport.

### 2B: JSON Details, Info Sheet, Share, Copy, Rotate & Reprocess

* Add Retrofit `converter-kotlinx-serialization`, `androidx.exifinterface:exifinterface`, `ACCESS_MEDIA_LOCATION` in
  `AndroidManifest.xml` + `MediaPermissions.requestedPermissions`, and `FileProvider` (`AndroidManifest.xml` +
  `res/xml/file_paths.xml`).
* Implement `GET /api/photos/{id}/item` (`MediaItemWithAlbums` `snake_case` DTOs with nullable `panorama_config`, noting
  that `PUT /api/photos/{id}/item` uses `camelCase` `mediaItemId` and `POST /api/photos/{id}/reprocess` returns an empty
  `200 OK` body declared as `Response<Unit>`), `MediaDetailsRepository` (in-memory LRU cache + `currentPage ± 1`
  neighbor prefetch), and `MediaRefreshCoordinator` (centralized delta sync + detail cache invalidation so Room
  `Flow` updates all active screens naturally).
* Build the swipe-up **Media Info Bottom Sheet** (`30.dp` rounded `surfaceContainerHigh`, gated on single-pointer
  `zoomFraction <= 0.01f` and also opened via **About** in the top-right overflow menu):
    * Display capture date/time in photo's local timezone (parse naive `taken_at_local` via `LocalDateTime.parse(...)`
      online; `sortTimestamp + timezoneOffsetMinutes` offline), filename, resolution/size, user caption, weather
      summary, camera/lens/exposure pills, albums list (read-only chips), and location string + coordinates with
      **"Open in Maps"** (`geo:` intent with web fallback, no embedded map until MapLibre is added).
    * Offline fallback via `MediaStore` display metadata + `ExifInterface` (photos, using
      `MediaStore.setRequireOriginal(uri)` guarded by `ACCESS_MEDIA_LOCATION` check + `try/catch` fallback) /
      `MediaMetadataRetriever` (videos) when `localContentUri != null`.
* Implement **Share** (bottom pill — always shares the actual photo/video file via `Intent.ACTION_SEND`, using
  `localContentUri` directly when local, or staging `full_res_cache` into `share_cache/{id}/{filename}` +
  `FileProvider` when `CLOUD_ONLY` so receiving apps get the real filename and extension; fall back to
  `Content-Disposition` or `"photo_$cloudId.jpg"`/`"video_$cloudId.mp4"` if `/item` isn't loaded yet, stream large
  `CLOUD_ONLY` videos directly to `share_cache` without evicting the 500 MB photo cache, and prune stale `share_cache`
  files periodically).
* Implement **Copy** (top-right menu, photos only — hidden on videos; copies image URI to Android clipboard via
  `ClipData.newUri`, using `localContentUri` or `share_cache` + `FileProvider` when `CLOUD_ONLY`).
* Implement **Rotate clockwise** (top-right menu; only for still, non-panorama, non-missing photos with supported MIME
  types `jpeg/jpg/tiff/png/webp` on server or `jpeg/png/webp` on `LOCAL_ONLY`; wait for `orientation` from `/item` on
  cloud/synced items; disabled while uploading in Phase 3's `upload_queue`):
    * Immediate optimistic visual rotation (`0° → 90° → 180° → 270°`, using the 8-entry EXIF orientation lookup table
      as web) + 2.5s debounce + immediate flush on swipe/exit (short-circuiting if net rotation is `0°`), executing the
      commit in `applicationScope` via `MediaRefreshCoordinator` so exiting the viewer doesn't cancel an in-flight
      mutation. Only swap `ratio` when `(netDegrees % 180) != 0`.
    * Request `MediaStore.createWriteRequest` on the **first Rotate tap** for `SYNCED`/`LOCAL_ONLY` (via a launcher
      hosted above the viewer overlay) and gate commit on *both* the debounce finishing and write consent succeeding
      (never committing while the OS confirmation dialog is open, reverting visual rotation if denied).
    * Evict Coil caches by scanning `imageLoader.memoryCache?.keys` where
      `it.key == "mediastore:$uri" || it.key == uri` and calling `imageLoader.diskCache?.remove(uri)`.
    * `CLOUD_ONLY`: `PUT /api/photos/{oldId}/item` → fetch `GET /api/photos/{newId}/download` into `full_res_cache`
      *before* clearing visual rotation (because `try_extract_exif_thumbnail` returns raw `IFD1` bytes while
      `IngestThumbnails` is pending) → atomically swap `"cloud_$oldId"` → `"cloud_$newId"` in Room, update viewer's
      active `mediaId` + `redirectedIds` without pager jump (and sync timeline scroll to `"cloud_$newId"` on dismiss),
      and evict `oldId` from `MediaDetailsRepository`.
    * `SYNCED`: `PUT /api/photos/{oldId}/item` → immediately persist `cloudId = newId` + adjusted `ratio` +
      `hasThumbnails = false` on that Room row (leaving any un-rotated sibling copies on `oldId` to detach on next
      sync) → download `GET /api/photos/{newId}/download` and overwrite `localContentUri` (keeping local bytes,
      `size_bytes`, and BLAKE3 identical to the server without setting `autoBackupSuppressed`).
    * `LOCAL_ONLY` (JPEG/PNG/WebP): update EXIF orientation via `ExifInterface`, trigger and await
      `MediaScannerConnection.scanFile` so `MediaStore`'s `ORIENTATION` updates promptly before `ScanLocalMediaUseCase`
      runs, and update Room `ratio` & `sizeBytes`.
* Implement **Reprocess** (top-right menu when `cloudId != null`): call `POST /api/photos/{id}/reprocess`, invalidate
  `MediaDetailsRepository` cache, and trigger `MediaRefreshCoordinator` delta sync. Because reprocess keeps the same
  `id` and isn't meant to change visual appearance, leaving Coil's `immutable` cached thumbnails intact is intentional.

### 2C: Video Playback (`Media3` / `ExoPlayer`)

* Add `androidx.media3:media3-exoplayer`, `media3-datasource-okhttp`, and `media3-ui-compose` (plus optionally
  `media3-ui-compose-material3`).
* Configure a single shared `ExoPlayer` wired with `OkHttpDataSource.Factory(okHttpClient)` (preserving session
  cookies & 401 refresh), `AudioAttributes` (`handleAudioFocus = true`) + `setHandleAudioBecomingNoisy(true)`,
  lifecycle pause on `ON_STOP`, and persisted **Loop** (`REPEAT_MODE_ONE` / `REPEAT_MODE_OFF`) + **Playback speed**
  preferences.
* Attach `ContentFrame` (with `SURFACE_TYPE_TEXTURE_VIEW`) only to `pagerState.settledPage`, using
  `rememberPresentationState(player)` and `ContentFrame`'s `shutter` slot to keep the photo thumbnail rendered
  underneath until the first video frame renders.
* Implement source selection + fallbacks:
    * Play `localContentUri` directly when `LOCAL_ONLY` or `SYNCED` (falling back to cloud on `SecurityException` /
      `FileNotFoundException`).
    * For `CLOUD_ONLY`: if `hasThumbnails == false`, stream `/api/photos/{id}/video` (original quality, Range-enabled,
      requires auth; show an explicit processing/unsupported error if codec playback fails). If
      `hasThumbnails == true`, default to **Original** (`/api/photos/{id}/video`) when on unmetered Wi-Fi without Data
      Saver and **`480p.webm`** otherwise; offer a quality picker (`Original`, `480p`, `144p` using
      `sourceHeight = min(width, height)` from `/item`) preserving `currentPosition` and `playWhenReady` across switches,
      and fall back transiently (without overwriting the saved preference) to `480p.webm` only if `Original` hits a
      codec/container/extractor `PlaybackException` (`ERROR_CODE_PARSING_*`, `ERROR_CODE_DECODER_*`,
      `ERROR_CODE_DECODING_*`, `ERROR_CODE_AUDIO_TRACK_*` — never on `ERROR_CODE_IO_*`/auth errors or if already on
      `480p.webm`).
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
