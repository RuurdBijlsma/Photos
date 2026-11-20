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
* view photo & video, pinch/double tap to zoom, share, copy to clipboard, rotate, edit date/time, reprocess, add to
  album. generally native gallery gestures & interactions
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
    * swipe up for info/gps/date pane (also reachable via "About" in top-right menu; edit date/time & timezone from the
      info pane)
    * swipe down to return back to timeline (timeline stays visible underneath during drag; syncs timeline scroll
      position to viewed item cleanly without frame-1 hitches)
    * bottom pill actions: "Share" (always shares actual file), "Add to" (album), "Bin"
    * top-right menu: About, Copy (photos only), Rotate clockwise, Reprocess, video Playback speed & video loop toggle
* initially, theming based on dynamic color (built into Android): `dynamicLightColorScheme` / `dynamicDarkColorScheme`
  plus `LocalSemanticColors` for sync/status badges

## LATER ON

* "cast" button (note: API 37 requires `ACCESS_LOCAL_NETWORK` permission for local discovery)
* semantic search (backend is already there), wouldn't need offline items to be searchable
* map view (and embedded mini-map in media info sheet)
* edit media GPS location (`PUT /api/photos/{id}/item`)
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
      `common_types/build.rs` and `web/package.json` `proto:gen`).
    * Define `TimelineSyncResponse` and `CompactTimelineItem` (`id`, `sort_timestamp`, `ratio`, `is_video`,
      `has_thumbnails`, `size_bytes`, `duration_ms`, `timezone_offset_minutes`).
* [✅ Done] **Compact sync endpoint (`GET /api/timeline/sync?since=...`):**
    * Return `Protobuf<TimelineSyncResponse>` with `upserted` (joining `media_features` for `size_bytes` and deriving
      `timezone_offset_minutes`), `deleted_ids` (union of soft-deleted `media_item` and `deleted_media_item`
      tombstones), `sync_timestamp`, and `is_full_snapshot` (when `since` is omitted or older than tombstone retention).
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
  `LocalSemanticColors`, top-level navigation shell, and API 37 `network_security_config.xml` for local `http://` dev
  testing).
* [✅ Done] Set up Retrofit + OkHttp with a persistent `CookieJar` to persist cookie-based session auth across app
  launches, dynamic base URL support, and `dispatcher.maxRequestsPerHost = 12–16`.
* [✅ Done] Handle expiring tokens safely via a mutex-guarded OkHttp `Authenticator` (preventing concurrent
  `/api/auth/refresh` races and 401 loops).
* [✅ Done] Build `AuthScreen` (custom server URL input validated via `GET /api/health`, email/password login).
* [✅ Done] Add `NetworkMonitor` (`ConnectivityManager.NetworkCallback`) and subtle Offline / Syncing status indicator so
  missing internet degrades gracefully without errors.
* [✅ Done] Compile `.proto` schemas (`common.proto` / `mobile.proto`) via the Protobuf Gradle plugin.
* [✅ Done] Configure Coil 3 (`coil-network-okhttp` sharing `OkHttpClient`, 1 GB LRU disk cache, hardware bitmaps, still
  AVIF via `LibavifDecoder` with `ImageDecoder` fallback for animated AVIF).
* [✅ Done] Implement `SyncTimelineUseCase`: initial full snapshot and delta sync (`since - 60_000L` overlap) directly
  into Room's `media_item` table (non-unique `cloudId`, unique `localContentUri`, immutable surrogate `id`). When
  `is_full_snapshot == true`, diff existing Room `cloud_id`s against `syncedCloudIds` in Kotlin and run the
  cloud-deletion transition on `staleCloudIds`.

### 1B: Local MediaStore & Pass 1 Deduplication

* [✅ Done] Request `READ_MEDIA_IMAGES` and `READ_MEDIA_VIDEO`, and handle Android 14+ partial access
  (`READ_MEDIA_VISUAL_USER_SELECTED`) cleanly.
* [✅ Done] Add `DeviceFolderEntity` + default discovery rules (`DCIM/*` excluding `DCIM/Screenshots` and `DCIM/Screen*`
  get `showInTimeline = true`, `backupEnabled = true`; all other folders default to `false`) and denormalized
  `is_visible_in_timeline` on `MediaEntity`.
* [✅ Done] Implement `ScanLocalMediaUseCase`: query `MediaStore` (`IS_PENDING = 0 AND IS_TRASHED = 0 AND SIZE > 0`,
  swapping dimensions for `90`/`270` orientation, falling back from `DATE_TAKEN` to `DATE_MODIFIED * 1000L`), gating
  cursor-omission deletion strictly on full per-type permissions plus an empty-cursor safety guard.
* [✅ Done] Run Pass 1 zero-hash deduplication against `CLOUD_ONLY` Room rows matching `size_bytes`, aspect `ratio`, and
  exact or 15-minute timezone-aligned timestamps (`±2s` within `±24h`) to flip matches to `SYNCED` with zero disk I/O.
* [✅ Done] Register custom `MediaStoreThumbnailFetcher` scoped to a `MediaStoreThumbnail` wrapper type calling
  `ContentResolver.loadThumbnail` with coroutine `CancellationSignal`, `SecurityException` cloud fallback, and
  subsampled `ImageDecoder` / `MediaMetadataRetriever` fallbacks—leaving raw `content://` URIs untouched for Phase 2
  Telephoto.

### 1C: Justified Timeline UI & Fast Scrubber

* [✅ Done] Implement 2-stage Room timeline queries over the composite index `(is_visible_in_timeline, year_month DESC,
  sort_timestamp DESC, id DESC)`: Stage 1 (top 2 `year_month`s for fast cold-start LCP) and Stage 2 (full library),
  with `.debounce(150)` on background `Flow` updates.
* [✅ Done] Build `CalculateTimelineLayoutUseCase` on `Dispatchers.Default` (`@Immutable` UI models, `ImmutableList`,
  pixel-snapped item widths, `rowHeightDp * density` thumbnail ladder capped at `720`/`480`, and `TimelineLayoutIndex`).
* [✅ Done] Build `TimelineScreen`: Compose `LazyColumn` of typed `MonthHeaderUiItem` and `TimelineRowUiItem`s (no
  `crossfade` in grid cells) plus draggable right-edge `TimelineScrubber` (`Modifier.systemGestureExclusion()`, instant
  `requestScrollToItem`, thumb tracking via `estimatedOffsetTopDp / maxScrollDp`, `isScrubbing` image-load gating, and
  floating `DateOverlay`).

---

## Phase 2: Media Viewer & Gestures

*(Scope excludes motion photo video playback and 3D panorama viewers — panoramas and motion photos render as regular
still photos.)*

### 2A: Navigation Shell, Photo Viewer & Dismiss Gestures (`Telephoto`)

* [✅ Done] Add `org.jetbrains.kotlin.plugin.serialization` + `kotlinx-serialization-json` (for type-safe `@Serializable` routes;
  `androidx.navigation:navigation-compose` is already in `gradle/libs.versions.toml`) and set up `NavHost` inside
  `SessionViewModelStoreHolder`: keep `MainScreen` mounted as the root destination (preserving `timelineListState`,
  tab state across `Photos`/`Collections`/`Explore`/`Search`, and `timeline_list` / `timeline_scrubber` test tags),
  with pushed routes (`DeviceFolders`, `AlbumDetail`, `BackupSettings`) and a generic `ViewerSource` contract (
  `Timeline`
  in Phase 2, reusable for `DeviceFolder` in Phase 3 and `Album` in Phase 5).
* [✅ Done] Host `MediaViewerScreen` in a `SharedTransitionLayout` / translucent overlay above `NavHost` rather than an opaque
  `NavHost` `composable` replacement so `TimelineScreen` stays live and visible underneath during shared-element
  transitions and interactive swipe-down dismiss:
    * Use `sharedElementWithCallerManagedVisibility` (or explicit bounds capture) since `MainScreen` inside `NavHost`
      and `MediaViewerScreen` above `NavHost` belong to separate visibility scopes, suppress accessibility and focus on
      the underlying `NavHost` container (`hideFromAccessibility()` / non-focusable) while the viewer overlay is open,
      bypass shared-element modifiers on `TimelineTile` while `isScrubbing` is true (or attach only for the active
      viewer item), and hoist a scroll-sync bridge/callback to `MainScreen`'s `timelineListState`.
    * Layer `BackHandler` precedence cleanly (`MediaInfoBottomSheet` / `DateTimeEditDialog` > `MediaViewerScreen` >
      `MainScreen` Search).
* [✅ Done] Share Stage 2's `ImmutableList<TimelineMediaProjection>` (with `@Immutable` on `TimelineMediaProjection`) via a hot
  `TimelineProjectionStore` across `TimelineViewModel` and `MediaViewerViewModel`—modeled as `Loading` vs.
  `Loaded(items)` (never initialized to `persistentListOf()`) so `MediaViewerViewModel` doesn't mistake an uninitialized
  store for a deleted library—while keeping `ObserveTimelineUseCase` compatible with `MediaRepository` (preserving
  `ObserveTimelineUseCaseTest`):
    * Open `MediaViewerScreen` on frame 1 in `0 ms` from the shared Stage 2 list (loading
      `mediaRepository.getById(initialMediaId)` directly only as fallback while the store is `Loading`) and anchor
      `HorizontalPager(beyondViewportPageCount = 1)` by `mediaId` so live Room updates, sync prepends, rotations, or
      date edits never jump `currentPage` to a different item.
    * Never run the Stage 1 top-2-months query in the viewer (which would miss older photos and trigger false deletions
      or page `0` jumps).
* [✅ Done] Integrate `me.saket.telephoto:zoomable-image-coil3` (`ZoomableAsyncImage`) with double-tap, pinch-to-zoom, and
  pan/Ultra HDR support:
    * Key the surrounding page via `HorizontalPager(key = ...)` / `key(mediaId)` (since `rememberZoomableState()` does
      not accept a `key` parameter directly), lock `HorizontalPager` (`userScrollEnabled` checks only the **active
      page's** `zoomFraction <= 0.01f`) when zoomed in, and reset zoom when swiping off-page.
    * Wire tap-to-toggle chrome via `ZoomableAsyncImage`'s `onClick` / `onLongClick` parameters (never
      `Modifier.clickable`, which Telephoto overrides for double-tap zoom).
    * **O(1) `MemoryCache.Key` handoff (`MemoryCacheKeyRegistry`):** Capture `result.memoryCacheKey` inside `onSuccess`
      on `TimelineTile` and `ZoomableAsyncImage` into a small `LruCache<String, MemoryCache.Key>` rather than scanning
      `imageLoader.memoryCache?.keys` under lock on every swipe.
    * **Two-tier preview-first loading for fast swipes (`localContentUri != null` & `CLOUD_ONLY`):**
        * Never decode/subsample full-res (`content://` `BitmapRegionDecoder` / full-res `HEIC`/`AVIF`) on adjacent
          `beyondViewportPageCount = 1` neighbors or during active pager drags (avoiding `saket/telephoto#165` OOMs and
          starving `LibavifDecoder`'s 4-permit pool).
        * Render `MediaStoreThumbnail` (at screen-capped height for photos or grid tier for videos, falling back to
          cloud `1440p` for photos / `<= 720p` for videos on `SecurityException` / `FileNotFoundException`) on frame 1
          and on `N ± 1` offscreen pages for local/synced items, upgrading photo pages to the raw `content://` `Uri` (
          with
          the captured `MemoryCache.Key` placeholder) only ~150–250ms after `settledPage == pageIndex` or immediately
          when zoom starts, and cancelling on swipe away.
        * For `CLOUD_ONLY` **photos** (`!isVideo`, including panoramas): render cached grid thumbnail → `1440p`
          (`hasThumbnails ? /thumbnails/{cloudId}/1440p.avif : /api/photos/{cloudId}/thumbnail?size=1440`); for
          `CLOUD_ONLY` **videos**, render the cached grid thumbnail tier (`<= 720p.avif` / on-demand `size=720`,
          matching `video_options.height: 720` in `settings.yaml`). Run `currentPage ± 1` preview prefetch inside
          `snapshotFlow { pagerState.settledPage }.collectLatest` after a ~120–150ms settle delay (cancelling superseded
          prefetches so rapid swiping doesn't saturate OkHttp's `maxRequestsPerHost = 16` or `LibavifDecoder`). Tune
          `LibavifDecoder` `threads = 2` (or benchmark hero `1440p` decodes vs. grid decodes) and verify `1440p.avif`
          Coil disk-cache snapshot closure (`saket/telephoto#181`); if `#181` reproduces, render non-subsamplable
          AVIF/GIF previews with `AsyncImage` + `Modifier.zoomable(...)` (from `me.saket.telephoto:zoomable`) until
          upgrading to `ZoomableAsyncImage` on the local `content://` or `full_res_cache` `file://` URI. *(Full-res
          `/download` upgrade wires in 2B once `/item` is available.)*
* [✅ Done] Build viewer chrome & gestures:
    * Tap to toggle floating chrome (using dark translucent scrims without `Modifier.blur` on pills, updating top bar
      title/counter text directly without `animateContentSize()` during pager drags so counter width changes don't
      cause spring relayouts mid-swipe, hiding/restoring `systemBars()` visibility and `isAppearanceLightStatusBars` /
      `isAppearanceLightNavigationBars` on exit inside `DisposableEffect`, and restoring light system bar icons as soon
      as swipe-down dismiss commits in Light Mode): `Back` button + title/counter pill, bottom floating pill with
      placeholder/disabled `Share`, `Add to`, and `Bin` buttons (wired in 2B and Phases 4–5), and top-right overflow
      `⋮` menu.
    * Single-pointer swipe down / swipe up arbitration (`pointerCount == 1`, `zoomFraction <= 0.01f`, `|dy| > |dx|`
      after
      touch slop so horizontal/diagonal swipes go cleanly to `HorizontalPager` and pinch gestures aren't stolen): swipe
      down interactively drags/shrinks the viewer over the visible timeline and dismisses (swipe up connects to the
      Info sheet in 2B). Instead of scrolling `TimelineScreen` on every fast swipe or on frame 1 of dismiss (which
      forces synchronous `LazyColumn` composition under the drag finger), silently pre-sync `timelineListState` ~250ms
      after `settledPage` pauses if `activeMediaId` is offscreen (falling back to scale-to-center + fade + settle scroll
      sync if dismissed mid-fling before settle).
* [✅ Done] Verify `assembleRelease` / `assembleBenchmark` with R8 `optimization { enable = true }`, update
  `app/src/main/keepRules/rules.keep` if `kotlinx-serialization`, `Telephoto`, or `Media3` require additional keep
  rules, benchmark `TimelineScrollBenchmark.kt` with `SharedTransitionLayout` attached, and extend
  `BaselineProfileGenerator.kt` to cover viewer open, fast swiping, zoom, and swipe-down dismiss.

### 2B: Media Details, Full-Res Photo Cache, Info Sheet, Share & Copy

* Add Retrofit `converter-kotlinx-serialization` (configured with `Json { ignoreUnknownKeys = true }`),
  `androidx.exifinterface:exifinterface`, `ACCESS_MEDIA_LOCATION` in `AndroidManifest.xml` +
  `MediaPermissions.requestedPermissions`, and `FileProvider` (`AndroidManifest.xml` + `res/xml/file_paths.xml`
  including `<cache-path name="share_cache" path="share_cache/" />`). For existing Phase 1 upgraders who already granted
  `READ_MEDIA_IMAGES` / `READ_MEDIA_VIDEO` without `ACCESS_MEDIA_LOCATION`, request `ACCESS_MEDIA_LOCATION` once when
  needed or rely cleanly on the `try/catch` fallback around `MediaStore.setRequireOriginal(uri)`.
* Implement `GET /api/photos/{cloudId}/item` (`MediaItemWithAlbums` `snake_case` DTOs with nullable `panorama_config`,
  noting that every server photo endpoint takes `cloudId`) and `MediaDetailsRepository` (in-memory LRU cache, loading
  `currentPage` immediately while running `currentPage ± 1` neighbor prefetch inside the debounced/cancellable
  `settledPage` `collectLatest` alongside 2A's preview thumbnail prefetch). Hook `SyncTimelineUseCase` (via
  `MediaRefreshCoordinator`) to evict `upserted` and `deleted_ids` `cloudId`s from `MediaDetailsRepository` so edits
  made on the web client never leave stale metadata in the Info sheet.
* Add `full_res_cache/` (~500 MB LRU) and `share_cache/`, and extend `SessionManager`'s `onClearSessionData` to clear
  `TimelineProjectionStore`, `MemoryCacheKeyRegistry`, `full_res_cache/`, `share_cache/`, and `MediaDetailsRepository`
  on sign-out / 401 token revocation.
* Upgrade `ZoomableAsyncImage` for `CLOUD_ONLY` **photos** (`!isVideo`, including panoramas): wait for `/item` so
  `mime_type` and `missing_since == null` are known, then—only after `settledPage == pageIndex` settles for ~150–250ms
  (or immediately on zoom), cancelling on swipe away—fetch `/api/photos/{cloudId}/download` into `full_res_cache`
  (deduplicated per `cloudId` with temp-file atomic rename) and pass the `file://` URI (with
  `diskCachePolicy(CachePolicy.DISABLED)` and the captured `1440p` `MemoryCache.Key` placeholder) so JPEGs/PNGs/WebPs
  subsample with gainmaps while full-res AVIF/HEIC fall back to Coil without `BitmapRegionDecoder` failing; skip
  `/download` for videos, TIFF/TGA, or `missing_since != null`.
* Build the swipe-up **Media Info Bottom Sheet** (`30.dp` rounded `surfaceContainerHigh`, gated on single-pointer
  `zoomFraction <= 0.01f` on both photo and video pages and also opened via **About** in the top-right overflow menu):
    * Display capture date/time in photo's local timezone (parse naive `taken_at_local` via `LocalDateTime.parse(...)`
      online; `sortTimestamp + timezoneOffsetMinutes` offline), filename, resolution/size, user caption, weather
      summary (checking `temperature != null`, not truthy, so `0 °C` renders, and `is_daytime != null`—or defaulting
      to day—so night weather icons render when `is_daytime == false`), camera/lens/exposure pills, albums list
      (read-only chips), and location string + coordinates with **"Open in Maps"** (`geo:` intent with web fallback, no
      embedded map until MapLibre is added).
    * For `CLOUD_ONLY` items viewed offline before `/item` is cached, populate basic info from Room's `MediaEntity`
      (`sortTimestamp + timezoneOffsetMinutes`, `sizeBytes`, `durationMs`) with a subtle offline indicator for
      server-only sections.
    * Offline fallback via `MediaStore` display metadata + `ExifInterface` (photos, using
      `MediaStore.setRequireOriginal(uri)` guarded by `ACCESS_MEDIA_LOCATION` check + `try/catch` fallback) /
      `MediaMetadataRetriever` (videos) when `localContentUri != null`.
* Implement **Share** (bottom pill — always shares the actual photo/video file via `Intent.ACTION_SEND` with
  `FLAG_GRANT_READ_URI_PERMISSION`, using `localContentUri` directly when local with `share_cache` fallback on
  `SecurityException`, or staging `full_res_cache` into `share_cache/{id}/{filename}` + `FileProvider` when `CLOUD_ONLY`
  so receiving apps get the real filename and extension; fall back to `Content-Disposition` or
  `"photo_$cloudId.jpg"`/`"video_$cloudId.mp4"` if `/item` isn't loaded yet, download `TIFF`/`TGA` on demand unlike
  automatic viewer prefetch, stream large `CLOUD_ONLY` videos directly to `share_cache` without evicting the 500 MB
  photo cache, disable/guard when `missing_since != null`, and prune stale `share_cache` files periodically).
* Implement **Copy** (top-right menu, photos only — hidden on videos and `missing_since != null`; copies image URI to
  Android clipboard via `ClipData.newUri`, staging via `share_cache` + `FileProvider` for both `CLOUD_ONLY` and
  local/synced photos so third-party paste targets without `READ_MEDIA_IMAGES` or under partial access can always read
  clipboard data).

### 2C: Video Playback (`Media3` / `ExoPlayer`)

* Add `androidx.media3:media3-exoplayer`, `media3-datasource-okhttp`, and `media3-ui-compose` (plus optionally
  `media3-ui-compose-material3`).
* Configure a single shared `ExoPlayer` wired with `OkHttpDataSource.Factory(okHttpClient)` (preserving session
  cookies & 401 refresh), tuned `DefaultLoadControl` (e.g., `bufferForPlaybackMs = 1_000`, `minBufferMs = 15_000`,
  `maxBufferMs = 30_000`) for snappy remote start, `AudioAttributes` (`handleAudioFocus = true`) +
  `setHandleAudioBecomingNoisy(true)`, lifecycle pause on `ON_STOP`, and persisted **Loop** (`REPEAT_MODE_ONE` /
  `REPEAT_MODE_OFF`) + **Playback speed** preferences.
* Attach `ContentFrame` (with `SURFACE_TYPE_TEXTURE_VIEW`) only to `pagerState.settledPage` (without wrapping it in a
  mid-playback changing `key(...)`), using `rememberPresentationState(player, keepContentOnReset = isQualitySwitch)`
  and `ContentFrame`'s `shutter` slot (using `<= 720p.avif` / on-demand `size=720` for `CLOUD_ONLY` videos, or
  `MediaStoreThumbnail` for local/synced)—keeping the shutter visible on `mediaId` changes until
  `Player.Listener.onRenderedFirstFrame` fires so quality switches on the same item don't flash black while swiping
  Video A → Video B always shows Video B's thumbnail shutter instead of Video A's last frame. Call `exoPlayer.stop()`
  and `exoPlayer.clearMediaItems()` synchronously when leaving a video page / changing `activeMediaId`.
* Implement source selection + fallbacks:
    * Play `localContentUri` directly when `LOCAL_ONLY` or `SYNCED` (falling back to cloud on `SecurityException` /
      `FileNotFoundException` when `cloudId != null`).
    * For `CLOUD_ONLY`: if `hasThumbnails == false`, stream `/api/photos/{cloudId}/video` (original quality,
      Range-enabled, requires auth; show an explicit processing/unsupported/missing-source error if playback fails). If
      `hasThumbnails == true` and `/item` is already cached with `missing_since != null`, skip `Original` and start on
      `480p.webm`; otherwise don't block playback on `/item`: default to **Original** (`/api/photos/{cloudId}/video`)
      when on unmetered Wi-Fi without Data Saver and **`480p.webm`** otherwise. Offer a quality picker (`Original`,
      `480p`, `144p` using `sourceHeight = min(width, height)` from `/item`, disabling `Original` if `/item` reports
      `missing_since != null`) preserving `currentPosition` and `playWhenReady` across switches.
    * **Transient fallback rule:** When `hasThumbnails == true` and not already on `480p.webm`, fall back transiently
      (without overwriting the saved preference) to `480p.webm` if `Original` fails with either a
      codec/container/extractor `PlaybackException` (`ERROR_CODE_PARSING_*`, `ERROR_CODE_DECODER_*`,
      `ERROR_CODE_DECODING_*`, `ERROR_CODE_AUDIO_TRACK_*`) **or** `ERROR_CODE_IO_BAD_HTTP_STATUS` with HTTP
      `404`/`415`/`500` (`InvalidResponseCodeException` when the source file is missing on server disk). Never fall back
      on offline/network/timeout errors (`ERROR_CODE_IO_NETWORK_CONNECTION_*`, etc.) or `401`/`403` auth errors.
* Build floating video controls integrated with tap-to-toggle chrome (play/pause overlay, seek bar + buffered
  indicator + `tnum` duration/position driven via isolated state-read lambdas so 10–60Hz polling never recomposes
  `HorizontalPager` or `ContentFrame`, mute toggle, quality selector) and expose **Playback speed** (`0.25x`–`3.0x`
  selector) and **Loop video** toggle in the top-right overflow menu.

### 2D: Mutation Infrastructure, Rotate & Reprocess

* Define a reusable `PatchField<T>` (`Ignore` / `SetNull` / `Value`) serializer for `PUT /api/photos/{cloudId}/item`
  (mirroring the server's `UpdateField<T>` so mutations never accidentally serialize `null` for `userCaption`, `gps`, or
  untouched fields, verified by a unit test with `encodeDefaults = false` / `explicitNulls = true`; note `PUT` returns
  `camelCase` `mediaItemId` unlike `GET /item` `snake_case`), wire `POST /api/photos/{cloudId}/reprocess`
  (`Response<Unit>`), and implement `MediaRefreshCoordinator` (centralized delta sync + detail cache invalidation so
  Room `Flow` updates all active screens naturally).
* Implement `LocalFileOverwriteHelper` (`MediaStore.createWriteRequest` via a host launcher above the viewer overlay +
  download to `full_res_cache` + non-empty verification + `"wt"` overwrite + **await**
  `MediaScannerConnection.scanFile` on `DATA`). Handle post-`PUT` overwrite failure cleanly (surface a retry/error
  state and avoid displaying stale unrotated local file bytes inside rotated `ratio` bounds).
* Implement **Rotate clockwise** (`RotateMediaUseCase`, top-right menu; only for still, non-panorama, non-missing
  photos with Android-decodable MIME types `jpeg/jpg/png/webp`—excluding `tiff` on mobile since `full_res_cache` skips
  `TIFF` in 2B; require online connectivity for `SYNCED`/`CLOUD_ONLY`; wait for `orientation` from `/item` on
  cloud/synced items; disabled while uploading in Phase 3's `upload_queue`):
    * Immediate optimistic visual rotation (`0° → 90° → 180° → 270°`, using the 8-entry EXIF orientation lookup table
      as web) + 2.5s debounce + immediate flush on swipe/exit (short-circuiting if net rotation is `0°`), executing the
      commit in `applicationScope` via `MediaRefreshCoordinator` and tracking `pendingRotationIds` so exiting the viewer
      doesn't cancel an in-flight mutation and quickly re-opening or re-tapping Rotate on the same item cannot fire a
      duplicate `PUT` against `oldCloudId`. Only swap `ratio` when `(netDegrees % 180) != 0`.
    * Request `MediaStore.createWriteRequest` on the **first Rotate tap** for `SYNCED`/`LOCAL_ONLY` and gate commit on
      *both* the debounce finishing and write consent succeeding (never committing while the OS confirmation dialog is
      open, reverting visual rotation if denied).
    * Evict Coil caches by clearing matching entries in `MemoryCacheKeyRegistry` / `imageLoader.memoryCache` and
      calling `imageLoader.diskCache?.remove(uri)`.
    * `CLOUD_ONLY`: `PUT /api/photos/{oldCloudId}/item` (via `PatchField`) → as soon as `PUT` succeeds, always
      atomically
      swap `"cloud_$oldCloudId"` → `"cloud_$newCloudId"` in Room, immediately replace `"cloud_$oldCloudId"` →
      `"cloud_$newCloudId"` in `MediaViewerViewModel`'s in-memory pager list alongside updating `activeMediaId` +
      `redirectedIds` (and sync timeline scroll to `"cloud_$newCloudId"` on idle/dismiss), and evict `oldCloudId` from
      `MediaDetailsRepository`. Eagerly fetch `GET /api/photos/{newCloudId}/download` into `full_res_cache` *before*
      clearing visual rotation when online (because `try_extract_exif_thumbnail` returns raw `IFD1` bytes while
      `IngestThumbnails` is pending), still keeping the Room ID swap intact even if that eager `/download` warm-up
      fails.
    * `SYNCED`: `PUT /api/photos/{oldCloudId}/item` → in one Room `@Transaction`, immediately persist
      `cloudId = newCloudId` + adjusted `ratio` + `hasThumbnails = false` + `autoBackupSuppressed = false` on that
      Room row **and** `deleteById("cloud_$newCloudId")` (preventing duplicate visible rows if `SyncTimelineUseCase`
      raced right after `PUT` committed, while leaving any un-rotated sibling copies on `oldCloudId` to detach on next
      sync) → run `LocalFileOverwriteHelper` (`GET /api/photos/{newCloudId}/download` → `full_res_cache` → overwrite
      `localContentUri` → await `MediaScannerConnection.scanFile`), keeping local bytes, `size_bytes`, and BLAKE3
      identical to the server without setting `autoBackupSuppressed`.
    * `LOCAL_ONLY` (JPEG/PNG/WebP): open `ContentResolver.openFileDescriptor(uri, "rw")` (keeping the descriptor open
      across `ExifInterface(pfd.fileDescriptor).saveAttributes()`), query `MediaStore.MediaColumns.DATA` and await
      `MediaScannerConnection.scanFile` on that path so `MediaStore`'s `ORIENTATION` updates promptly before
      `ScanLocalMediaUseCase` runs, and update Room `ratio` & `sizeBytes`. Note the `ContentObserver` race before
      `scanFile` completes and verify on device that `MediaProvider` extracts `ORIENTATION` from PNG `eXIf` / WebP
      chunks.
* Implement **Reprocess** (top-right menu when `cloudId != null` and `missing_since == null`): call
  `POST /api/photos/{cloudId}/reprocess`, invalidate `MediaDetailsRepository` cache, and trigger
  `MediaRefreshCoordinator` delta sync. Because reprocess keeps the same `id` and isn't meant to change visual
  appearance, leaving Coil's `immutable` cached thumbnails intact is intentional.

### 2E: Datetime & Timezone Editing

* Build `DateTimeEditDialog` triggered from tapping the capture date/time or timezone row inside `MediaInfoBottomSheet`
  (matching web's `EditDateTimeCard.vue`):
    * Material 3 `DatePicker` + `TimePicker` initialized from the item's naive `taken_at_local` (or `sortTimestamp +
      timezoneOffsetMinutes` when offline `LOCAL_ONLY`), preserving the item's existing `:ss` seconds when picking
      hours/minutes, plus a `±HH:MM` timezone offset editor on server-backed items. Track date/time vs. `±HH:MM` offset
      dirtiness separately so editing only one sends `PatchField.Ignore` for the other (and no-op saves skip the API
      call).
    * Explicit Save with loading state (no optimistic file mutation before the API succeeds); surface `exiftool` errors
      cleanly (e.g. non-QuickTime video containers like `.mkv`/`.avi`/`.webm`).
* Implement `EditMediaDateTimeUseCase` (reusing Phase 2D's `PatchField<T>`, `MediaRefreshCoordinator`,
  `LocalFileOverwriteHelper`, `missing_since != null` guard, and Phase 3 `upload_queue` active-transfer guard):
    * Format `takenAtLocal` strictly as `"yyyy-MM-dd'T'HH:mm:ss"` (`Locale.US`) when edited, and leave
      `timezoneOffsetSeconds` as `PatchField.Ignore` unless the timezone offset itself was edited.
    * **`CLOUD_ONLY`:** Call `PUT /api/photos/{cloudId}/item` → evict `full_res_cache` and `share_cache` for `cloudId`
      (because `cloudId` stays unchanged while the file's EXIF bytes change) and invalidate `MediaDetailsRepository` →
      trigger `MediaRefreshCoordinator` delta sync so Room updates `sortTimestamp`, `timezoneOffsetMinutes`,
      `yearMonth`, and `sizeBytes`.
    * **`SYNCED` (requires online):**
        * *Photos (`!isVideo`):* Obtain `MediaStore.createWriteRequest` consent → call
          `PUT /api/photos/{cloudId}/item` → evict `full_res_cache`, `share_cache`, and `MediaDetailsRepository` → run
          `LocalFileOverwriteHelper` (`GET /api/photos/{cloudId}/download` → `full_res_cache` → overwrite
          `localContentUri` → await `MediaScannerConnection.scanFile`) so phone `size_bytes` and BLAKE3 stay identical
          to the server (handling any post-`PUT` overwrite failure with a clear retry/error state) → trigger
          `MediaRefreshCoordinator` delta sync.
        * *Videos (`isVideo`):* Call `PUT /api/photos/{cloudId}/item` → evict `share_cache` and `MediaDetailsRepository`
          → trigger `MediaRefreshCoordinator` delta sync, skipping local file overwrite to avoid re-downloading multi-GB
          videos (`ScanLocalMediaUseCase` preserves server metadata on `cloudId != null` rows).
    * **`LOCAL_ONLY` (JPEG/PNG/WebP only; disabled for `LOCAL_ONLY` videos & HEIC/AVIF/GIF until backed up):**
      Restrict edits to wall-clock date/time in the device's current timezone (disable timezone-offset editing for
      `LOCAL_ONLY` so subsequent incremental `MediaStore` scans using `ZoneId.systemDefault()` do not overwrite custom
      offsets). Obtain `MediaStore.createWriteRequest` → `openFileDescriptor(uri, "rw")` → write
      `TAG_DATETIME_ORIGINAL`, `TAG_DATETIME_DIGITIZED`, `TAG_DATETIME` (`"yyyy:MM:dd HH:mm:ss"`) **plus**
      `TAG_OFFSET_TIME_ORIGINAL`, `TAG_OFFSET_TIME`, and `TAG_OFFSET_TIME_DIGITIZED` (`±HH:MM` for
      `ZoneId.systemDefault()` at that instant, normalizing `"Z"` to `"+00:00"`, so any existing travel offset in EXIF
      doesn't shift `MediaStore.DATE_TAKEN`) via `ExifInterface.saveAttributes()` → await
      `MediaScannerConnection.scanFile` on `DATA` → update `sortTimestamp`, `timezoneOffsetMinutes`, `yearMonth`,
      `sizeBytes`, and reset `blake3Hash = null` in Room. Note the `ContentObserver` race before `scanFile` completes
      and
      verify on device that `MediaProvider` extracts `DATE_TAKEN` from PNG `eXIf` / WebP chunks (otherwise
      `MediaStoreDataSource` falls back to `DATE_MODIFIED * 1000L`, which becomes `now()` after saving).
* Verify live `HorizontalPager` & `TimelineScreen` reordering:
    * When `TimelineProjectionStore` emits the reordered Stage 2 list after editing a date across months,
      `MediaViewerViewModel` keeps `activeMediaId` anchored and snaps `HorizontalPager` (`scrollToPage(newIndex)`)
      without firing the deleted-item guard, prefetching the new adjacent neighbors (`newIndex ± 1`) once settled.
    * On swipe-down dismiss, verify `TimelineScreen` syncs `listState.scrollToItem(newRowIndex)` to the edited item's
      new month/position (via the settled-idle sync or dismiss settle fallback).

---

## Phase 3: Backup Engine, Device Folders & Manual Upload

* Build `DeviceFoldersScreen` ("On this device") & `BackupSettingsScreen`:
    * Browse all discovered folders (including non-timeline folders like Screenshots, Download, WhatsApp).
    * Toggle per-folder `backup_enabled` (which auto-enables `show_in_timeline`) and `show_in_timeline`.
    * Configure Wi-Fi / charging constraints and trigger manual upload (per-item action or "Back up now").
* Add Room `upload_queue` table (kept separate from `media_item` so upload progress ticks never re-trigger
  `CalculateTimelineLayoutUseCase`, and cleared in `SessionManager.onClearSessionData` on logout) to track progress,
  errors, TUS upload URL/offset, and `UPLOADED_PENDING_SYNC` state (reconciled via `POST /api/upload/check-hashes`
  before applying `SyncTimelineUseCase` snapshots, with max-age/retry `FAILED` handling). Also wire `upload_queue`
  active state into Phase 2D/2E's Rotate and Date/Time edit guards so items actively uploading cannot be mutated
  mid-transfer.
* Implement Pass 2 lazy BLAKE3 hashing + `POST /api/upload/check-hashes` pre-upload deduplication:
    * Prefer merging into an unlinked `CLOUD_ONLY` row if one matches; otherwise link to the first returned `cloud_id`
      with `is_visible_in_timeline = 0` when a canonical visible row for that `cloud_id` already exists.
* Implement `UploadWorker` with a derived `OkHttpClient` (`120s+` read/write timeouts, `404`/`410` stale upload URL
  recovery, and fresh 5-minute upload ticket renewal if `POST` TUS creation returns `401`/`403`) for 5 MB TUS chunked
  resumable uploads:
    * Run routine background auto-backups as normal constrained `WorkManager` batches (persisting TUS URL and offset in
      `upload_queue` after every chunk).
    * Reserve `setForeground(createForegroundInfo())` (guarded against `ForegroundServiceStartNotAllowedException`) or
      UIDT for explicit user-triggered "Back up now" / manual upload sessions started in the foreground (verifying
      `work-runtime-ktx:2.12.0` vs `targetSdk = 37` for `InvalidForegroundServiceTypeException`).
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