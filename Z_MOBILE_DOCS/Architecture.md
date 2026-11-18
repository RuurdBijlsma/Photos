PURPOSE OF THE DOC

Purpose is to show a general idea of the breadth of the application. It's not a hard "in stone" written fact that the
things written here should be implemented as such. During implementation I'm sure we'll encounter problems that will
make another solution that what's proposed here a cleaner architecture. Just go with that, don't think these are sacred
texts.

### Part 2: Android App Architecture

Adopt **Clean Architecture + MVI/MVVM** with **Unidirectional Data Flow (UDF)**.

```
┌──────────────────────────────────────────────────────────────┐
│                        UI Layer                              │
│   Compose Screens (Timeline, Viewer, Albums, Settings)       │
│   ViewModels (StateFlow<UiState>, UiEvent channels)          │
└──────────────────────────────┬───────────────────────────────┘
│
┌──────────────────────────────▼───────────────────────────────┐
│                      Domain Layer                            │
│   UseCases (CalculateLayout, SyncTimelineUseCase,            │
│   DeduplicateMediaUseCase, EnqueueUploadUseCase,             │
│   RotateMediaUseCase, EditMediaDateTimeUseCase)              │
│   Domain Models (MediaItem, DeviceFolder, SyncStatus, Row)   │
└──────────────────────────────┬───────────────────────────────┘
│
┌──────────────────────────────▼───────────────────────────────┐
│                       Data Layer                             │
│  Repositories (MediaRepository, MediaDetails, Backup, Album) │
│  Local: Room DB, MediaStore DataSource, full_res_cache       │
│  Remote: OkHttp + Persistent CookieJar, Protobuf + JSON API  │
│  Background: WorkManager (UploadWorker, LocalScanWorker)     │
└──────────────────────────────────────────────────────────────┘
```

#### Key Architecture Principles for this App:

1. **Cookie-Based Session Authentication, Persistent CookieJar & Session Cleanup:**
    * Matches the web client (`web/src/scripts/services/api.ts`) by relying exclusively on HTTP cookies (`access_token`,
      `refresh_token`) set by the backend on `POST /api/auth/login`.
    * OkHttp is configured with a persistent `CookieJar` (backed by SharedPreferences).
    * Every network call—including Retrofit API requests, Coil thumbnail image fetches, full-res downloads, Media3 video
      streaming, and TUS upload initiation—shares this exact `OkHttpClient` instance and automatically includes the
      session cookies.
    * Transparent session refresh: A mutex-guarded OkHttp `Authenticator` triggers `POST /api/auth/refresh` when an
      access cookie expires, updating the cookie store seamlessly without re-prompting the user.
    * **Complete sign-out cleanup:** `SessionManager`'s `onClearSessionData` callback (triggered on explicit logout and
      401 refresh failure) must clear all user-scoped singletons and caches—not just `media_item` and Coil's
      `image_cache`, but also `MediaDetailsRepository`'s in-memory LRU cache, `full_res_cache/`, `share_cache/`, and
      Phase 3–5 tables (`upload_queue`, `album*`)—so switching accounts never leaks cached media or queues uploads for
      the wrong user.
2. **Room Database as Single Source of Truth (SSOT) & Refresh Coordination:**
    * Room stores complete `MediaEntity` rows (local-only items, synced items, and cloud-only items), including
      precomputed `yearMonth`, `timezoneOffsetMinutes`, `isVisibleInTimeline`, `autoBackupSuppressed`, and local folder
      metadata (`bucketId`, `bucketName`, `relativePath`), alongside `DeviceFolderEntity` for per-folder timeline
      visibility and backup rules.
    * **Surrogate `id` vs. `cloudId`:** Room's primary key (`"local_<mediaStoreId>"` or `"cloud_<cloudId>"`) is strictly
      for local/navigation identity. Every server photo endpoint (`/api/photos/{cloudId}/item`, `/download`, `/video`,
      `/thumbnail`, `/reprocess`, `PUT /api/photos/{cloudId}/item`) must always receive `cloudId`, never Room's
      surrogate `id`.
    * Instead of a manual web-style `refreshStore.counter++`, **Room `Flow` is the UI refresh mechanism**: any local
      mutation in Room (rotation, datetime edit, deletion, album update, sync) propagates automatically to every
      observing screen.
    * A lightweight **`MediaRefreshCoordinator`** in `AppContainer` handles side-effect refreshes after async or
      server-side mutations (like `POST /api/photos/{cloudId}/reprocess` or `PUT /api/photos/{cloudId}/item`):
      invalidating the in-memory `/item` details cache (plus Coil caches or `full_res_cache`/`share_cache` when the
      underlying asset or EXIF bytes changed) and running `SyncTimelineUseCase`. Also hook `SyncTimelineUseCase` to
      evict any `upserted` or `deleted_ids` `cloudId`s from `MediaDetailsRepository`'s in-memory LRU cache so edits
      made on the web client never leave stale `/item` metadata in the Info sheet.
3. **Local vs Cloud Identity, Permissions & Two-Pass Deduplication:**
    * Media items in Room contain:
        * `localContentUri`: `content://media/external/images/media/1234` (nullable, `UNIQUE`)
        * `cloudId`: Backend 10-char alphanumeric string `a1B2c3D4e5` (nullable, non-unique index)
        * `bucketId`: `MediaStore` bucket ID for local items (nullable, indexed)
        * `bucketName`: Display folder name, e.g., `"Camera"`, `"Screenshots"` (nullable)
        * `relativePath`: Relative storage path, e.g., `"DCIM/Camera/"`, `"Pictures/Screenshots/"` (nullable)
        * `isVisibleInTimeline`: Denormalized boolean/int (`1` for `CLOUD_ONLY` and the canonical `SYNCED` row per
          `cloudId`; `0` for secondary duplicate local copies sharing an already-visible `cloudId`; synced from
          `device_folder.show_in_timeline` for `LOCAL_ONLY`)
        * `autoBackupSuppressed`: Boolean (default `false`; set to `true` if a `SYNCED` item is deleted from the server
          so auto-backup does not immediately re-upload it)
        * `sizeBytes`: File size in bytes (indexed, from `CompactTimelineItem` or `MediaStore`)
        * `ratio`: Orientation-corrected aspect ratio `width / height`
        * `sortTimestamp`: UTC epoch millis
        * `timezoneOffsetMinutes`: Offset of local capture time from `sortTimestamp`
        * `yearMonth`: Precomputed `YYYYMM` integer for contiguous local-month grouping
        * `blake3Hash`: File BLAKE3 hash (lowercase hex) matching backend `media_item.hash` (nullable, computed lazily)
        * `syncStatus`: `LOCAL_ONLY`, `SYNCED`, `CLOUD_ONLY` (transient `SYNCING` / `UPLOADED_PENDING_SYNC` lives in
          `upload_queue`)
    * **Permissions:** Request `READ_MEDIA_IMAGES`, `READ_MEDIA_VIDEO`, and `ACCESS_MEDIA_LOCATION`, and handle Android
      14+ partial access (`READ_MEDIA_VISUAL_USER_SELECTED`) cleanly so granting partial access does not break or loop
      permission prompts; track full vs. partial access per media type so cursor-omission deletion only runs when full
      access is granted. For users upgrading from Phase 1 who already granted media permissions without
      `ACCESS_MEDIA_LOCATION`, either prompt once if missing or rely on the silent `try/catch` fallback around
      `MediaStore.setRequireOriginal(uri)` so offline `ExifInterface` reads retain GPS coordinates when granted without
      failing when unavailable.
    * **Matching Pipeline:**
        * *Fast Pass:* On startup/scan, query `CLOUD_ONLY` rows by `size_bytes` and match in Kotlin on `ratio` + exact
          or 15-minute timezone-aligned timestamp (`±2s` within `±24h`). Matched items flip to `SYNCED` with zero disk
          hashing.
        * *Lazy Crypto Verification:* For unmatched local items staged for upload (including duplicate local copies and
          rare `size_bytes` collisions with >24h timestamp drift), compute BLAKE3 lazily and verify against the backend
          prior to streaming bytes.
4. **Coil 3 Thumbnail Strategy & Separate Viewer Full-Res Cache:**
    * **Target & Min SDK:** Targeting Android 15 and up (`minSdk = 35`, `targetSdk = 37`) provides native
      hardware-accelerated AVIF decoding via `ImageDecoder` (plus an in-process `LibavifDecoder` via `libavif` + `dav1d`
      to bypass Binder IPC overhead) and modern adaptive windowing (note API 37 ignores fixed orientation /
      non-resizable restrictions on large screens).
    * **Custom `MediaStoreThumbnailFetcher` (Scoped to Grid Thumbnails):** For `LOCAL_ONLY` and `SYNCED` items, wrap
      the URI in a dedicated `@Immutable data class MediaStoreThumbnail(...)` and register
      `MediaStoreThumbnailFetcher.Factory` + `ThumbnailKeyer` on Coil instead of matching raw `content://` `Uri`s
      globally. Grid cells pass `MediaStoreThumbnail(localContentUri)` so Coil calls
      `ContentResolver.loadThumbnail(uri, Size(widthPx, heightPx), signal)`, while `MediaViewerScreen`
      (`ZoomableAsyncImage`) passes the raw `Uri` directly so Coil's `ContentUriFetcher` and Telephoto serve
      full-resolution tiles (falling back to cloud `1440p` on `SecurityException` or `FileNotFoundException`):
        * Wire `CancellationSignal` to the coroutine's `Job` (`invokeOnCompletion { signal.cancel() }`) so fast
          scrolling immediately aborts pending IPC thumbnail loads.
        * On `loadThumbnail` failure, fall back to subsampled `ImageDecoder` for **images** and
          `MediaMetadataRetriever` for **videos**.
    * **Cloud Thumbnails (`image_cache`, 1 GB LRU):**
        * `hasThumbnails == true`: `"${BASE_URL}/thumbnails/${cloudId}/${targetHeightPx}p.avif"` (no auth required;
          `1440p` in viewer)
        * `hasThumbnails == false`: `"${BASE_URL}/api/photos/${cloudId}/thumbnail?size=${targetHeightPx}"` (requires
          auth; `1440p` in viewer)
    * **Viewer Full-Res File Cache (`cacheDir/full_res_cache/`, ~500 MB LRU):**
        * Coil 3 does not support multiple disk-cache budgets inside one `ImageLoader`. Keeping `PhotosImageLoader`'s 1
          GB `image_cache` strictly for thumbnails prevents large 20–50 MB `/api/photos/{cloudId}/download` originals
          from evicting timeline thumbnails.
        * For `CLOUD_ONLY` **photos** (`!isVideo`), wait for `GET /api/photos/{cloudId}/item` so `mime_type` and
          `missing_since` are known. When `missing_since == null` and MIME is natively decodable (JPEG, PNG, WebP, GIF,
          AVIF, HEIC/HEIF — skip TIFF/TGA), the viewer displays `1440p` immediately and fetches
          `/api/photos/{cloudId}/download` after a short delay (or immediately on zoom) into `full_res_cache`,
          deduplicating concurrent downloads per `cloudId` via a temp-file atomic rename. Never run this automatic
          full-res prefetch for videos.
        * Pass the cached `file://` URI (with `diskCachePolicy(CachePolicy.DISABLED)`) to `ZoomableAsyncImage` (rather
          than bare `SubSamplingImageSource.file(...)`) so Telephoto subsamples JPEG/PNG/WebP with Ultra HDR gainmaps
          and automatically falls back to Coil (`LibavifDecoder` / `ImageDecoder`) for full-res AVIF/HEIC originals. Key
          the surrounding page (`HorizontalPager(key = ...)` / `key(mediaId)`) since `rememberZoomableState()` does not
          take a `key` parameter directly, and use `ZoomableAsyncImage`'s `onClick` / `onLongClick` parameters rather
          than `Modifier.clickable` / `combinedClickable` (which Telephoto consumes for double-tap zoom). Because Coil 3
          encodes `coil#size` into `MemoryCache.Key.extras` when `isSampled = true`, resolve/capture the actual
          `MemoryCache.Key` (matching `"mediastore:$uri"` or the `1440p` URL, or captured via `onSuccess`) for
          `placeholderMemoryCacheKey` rather than passing a bare `String`, so upgrading thumbnail → `1440p` → full-res
          never flickers or resets zoom.
        * **Share** and **Copy** reuse the exact same `full_res_cache` file without downloading twice (staging into
          `share_cache/{id}/{filename}` + `FileProvider` with `FLAG_GRANT_READ_URI_PERMISSION` / `ClipData.newUri` so
          receiving apps get the real filename and extension, falling back to `Content-Disposition` or
          `"photo_$cloudId.jpg"`/`"video_$cloudId.mp4"` if `/item` isn't loaded, and pruning stale `share_cache` files
          periodically). For **Copy** (`ClipData.newUri`), stage local/synced photos via `share_cache` + `FileProvider`
          as well (rather than passing a raw `MediaStore` URI directly) so target apps without `READ_MEDIA_IMAGES` (or
          under partial access) can always read pasted clipboard data. Unlike automatic viewer prefetch, manual
          **Share** still downloads `TIFF`/`TGA` originals on demand when `missing_since == null`. When sharing a
          `CLOUD_ONLY` video (or large file exceeding cache thresholds), stream directly to `share_cache` so it doesn't
          evict the 500 MB photo `full_res_cache`. Disable or hide **Share**, **Copy**, **Rotate**, and **Date/Time
          editing** when `missing_since != null` (or surface a clear message that the source file is missing from server
          disk).
    * **Grid ImageLoader tuning:**
        * Always pass exact pixel `.size(widthPx, heightPx)` on `ImageRequest`.
        * Keep hardware bitmaps enabled (`allowHardware(true)`).
        * **Disable `crossfade` in the timeline grid** to avoid simultaneous alpha-layer overdraw across 15+ cells.
        * Configure `MemoryCache` (~35% of heap), `DiskCache` (1 GB LRU), and wire the shared `OkHttpClient` (with
          `maxRequestsPerHost = 16`) into `OkHttpNetworkFetcherFactory`.
5. **Video Playback Strategy (`Media3` / `ExoPlayer`):**
    * **Shared `OkHttpClient` DataSource:** Media3's `DefaultHttpDataSource` does **not** use OkHttp or your
      `CookieJar`. Configure `ExoPlayer` with `media3-datasource-okhttp` (`OkHttpDataSource.Factory(okHttpClient)`) so
      authenticated remote video streams automatically carry session cookies and trigger transparent 401 token refresh.
      Configure `AudioAttributes` (`USAGE_MEDIA`, `CONTENT_TYPE_MOVIE`) with `handleAudioFocus = true` and
      `setHandleAudioBecomingNoisy(true)`.
    * **Surface ownership in `HorizontalPager`:** Because `HorizontalPager` composes adjacent offscreen pages, attach
      the single shared `ExoPlayer`'s `ContentFrame` only to the **currently settled page**
      (`pagerState.settledPage == pageIndex`) using `SURFACE_TYPE_TEXTURE_VIEW` (because `SurfaceView` ignores Compose
      `graphicsLayer` transforms during swipe-down dismiss and pager drags), and keep `ContentFrame` stable without
      mid-playback `key(...)` recreation. Pair it with `rememberPresentationState(player, keepContentOnReset = true)`
      and `ContentFrame`'s `shutter` slot so quality switches don't flash black and the photo thumbnail stays rendered
      until the first video frame arrives (`coverSurface`). When leaving a video page or changing `activeMediaId`, call
      `exoPlayer.stop()` and `exoPlayer.clearMediaItems()` synchronously (distinct from quality switches on the same
      `mediaId`) so switching from one video to another never briefly shows the previous video's frame. Pause on
      `ON_STOP`.
    * **Source selection & transient fallback in `MediaViewerScreen`:**
        * **Local / Synced (`localContentUri != null`):** Play directly from `localContentUri` (instant, zero
          bandwidth, full quality), falling back to cloud playback if `ExoPlayer` encounters `SecurityException` or
          `FileNotFoundException`.
        * **Cloud-only (`CLOUD_ONLY`):**
            * **`hasThumbnails == false`:** Thumbnails and WebM transcodes have not been generated yet, so **only** the
              on-demand original-quality endpoint (`/api/photos/{cloudId}/video`, HTTP `Range`-enabled, requires auth)
              can be used. If `Original` fails here, show a clear unsupported/processing/missing-source error state.
            * **`hasThumbnails == true`:** If `/item` is already cached (e.g., prefetched for `currentPage ± 1`) and
              `missing_since != null`, start directly on `480p.webm` and disable `Original` in the quality picker.
              Otherwise, **do not block playback on `/item`**: default to **Original** (`/api/photos/{cloudId}/video`)
              when on unmetered Wi-Fi without Data Saver (`ConnectivityManager.isActiveNetworkMetered` +
              `restrictBackgroundStatus`), and default to **`480p.webm`** otherwise. Offer a quality picker
              (`Original`, `480p`, `144p`, reading `sourceHeight = min(width, height)` from
              `GET /api/photos/{cloudId}/item`
              once loaded, and disabling `Original` if `missing_since != null`), preserving `currentPosition` and
              `playWhenReady` across switches.
            * **Transient fallback rule:** When `hasThumbnails == true` and not already on `480p.webm`, fall back
              transiently to `480p.webm` (without overwriting the persisted preference) if `Original` fails with:
                1. A codec/container/extractor `PlaybackException` (`ERROR_CODE_PARSING_*`, `ERROR_CODE_DECODER_*`,
                   `ERROR_CODE_DECODING_*`, `ERROR_CODE_AUDIO_TRACK_*`), **or**
                2. `ERROR_CODE_IO_BAD_HTTP_STATUS` where the server rejected the original stream with HTTP `404`, `415`,
                   or `500` (`cause as? HttpDataSource.InvalidResponseCodeException`—which happens when
                   `stream_video_file`
                   cannot open a missing source file on disk while `480p.webm` is still served from `/thumbnails`).
                   Never fall back on offline/network/timeout errors (`ERROR_CODE_IO_NETWORK_CONNECTION_*`, etc.) or
                   auth errors (`401`/`403`). Note `/thumbnails/...` does **not** require authentication.
    * **Video menu preferences:** Persist **Loop video** (`REPEAT_MODE_ONE` vs `REPEAT_MODE_OFF`) and **Playback speed**
      (`0.25x`–`3.0x`) in DataStore, controlled from the viewer's top-right overflow menu.
6. **Rotation, Datetime Editing & Reprocess Architecture:**
    * **Shared `PUT /api/photos/{cloudId}/item` DTO (`PatchField<T>`):** The backend deserializes
      `UpdateMediaItemRequest` using `UpdateField<T>` where omitting a key means `Ignore`, sending `null` means
      `SetNull`
      (clearing `userCaption`, `gps`, or `timezoneOffsetSeconds`), and sending a value means `Value`. Model this in
      Kotlin with a reusable `PatchField<T>` (`Ignore` / `SetNull` / `Value`) serializer so neither rotation nor
      datetime/timezone edits ever accidentally wipe unrelated fields. Format `takenAtLocal` strictly as
      `"yyyy-MM-dd'T'HH:mm:ss"` (naive wall-clock time without zone/offset, matching `NaiveDateTime` on the server;
      `LocalDateTime.toString()` omits `:ss` when seconds are `0` and fails validation). Note `PUT` returns
      `camelCase` (`mediaItemId`) unlike `GET /item` (`snake_case`).
    * **Shared Upload Guard & Local File Overwrite Helper:** Block/disable both **Rotate** and **Date/Time editing**
      while an item is queued/active or `UPLOADED_PENDING_SYNC` in Phase 3's `upload_queue` (so `size_bytes` and
      `blake3_hash` cannot change mid-transfer), and when `missing_since != null`. Share a `LocalFileOverwriteHelper`
      for `SYNCED` photo mutations: obtain `MediaStore.createWriteRequest` consent, download
      `GET /api/photos/{cloudId}/download` into `full_res_cache`, verify the downloaded file is non-empty *before*
      truncating `localContentUri` via `ContentResolver.openFileDescriptor(uri, "wt")`, and trigger and **await**
      `MediaScannerConnection.scanFile` on its `MediaStore.MediaColumns.DATA` path (if available) so the phone's file
      stays byte-for-byte identical to the server without setting `autoBackupSuppressed` and without `ContentObserver`
      rescanning stale `MediaStore` columns. If the server `PUT` succeeded but `LocalFileOverwriteHelper` fails (e.g.,
      network drop during `/download` or `"wt"` write error), surface a clear retry/error state and avoid displaying
      stale unrotated local file bytes inside rotated `ratio` bounds.
    * **Rotation (`RotateMediaUseCase`):**
        * **Eligibility:** Only allow rotation for still, non-panorama, non-missing photos (
          `!isVideo && !usePanoramaViewer && missingSince == null`) with Android-decodable MIME types
          (`jpeg/jpg/png/webp` on both server-backed and `LOCAL_ONLY` items—excluding `tiff` on mobile since
          `full_res_cache` skips `TIFF` in 2B), require network connectivity for `SYNCED`/`CLOUD_ONLY` items, and wait
          until `orientation` is loaded from `/item` for cloud/synced items.
        * **Server side effects (`PUT /api/photos/{oldCloudId}/item`):** `apply_orientation_update` rewrites the file
          with `exiftool`, wipes thumbnails (`has_thumbnails = false`), and calls
          `MediaItemStore::change_media_item_id`,
          generating a brand-new `newCloudId` (which inserts `newCloudId` and deletes `oldCloudId`, emitting a
          `deleted_media_item` tombstone for `oldCloudId`).
        * **Optimistic UI, debounce, scope & `createWriteRequest`:** Rotating taps increment visual rotation immediately
          (`0° → 90° → 180° → 270°`, using the same 8-entry EXIF orientation lookup table as web) with a 2.5s debounce
          before committing; if the user swipes to another item or exits the viewer before the timer fires, flush the
          pending rotation immediately (skipping if net rotation is `0°`). Only swap `ratio` when
          `(netDegrees % 180) != 0`.
          Execute the commit in `applicationScope` (inside `MediaRefreshCoordinator` / `RotateMediaUseCase`) so exiting
          `MediaViewerScreen` doesn't cancel an in-flight server mutation. For `SYNCED`/`LOCAL_ONLY`, request
          `MediaStore.createWriteRequest` on the **first Rotate tap** (hosted above the viewer overlay) and gate the
          commit on *both* the 2.5s debounce finishing and write consent succeeding (never committing while the OS
          confirmation dialog is open, and reverting visual rotation if denied).
        * **`CLOUD_ONLY`:** Call `PUT /api/photos/{oldCloudId}/item` → fetch `GET /api/photos/{newCloudId}/download`
          into
          `full_res_cache` *before* clearing visual rotation (because `try_extract_exif_thumbnail` returns raw `IFD1`
          bytes without the updated `IFD0` Orientation while `IngestThumbnails` is pending) → in one Room
          `@Transaction`, delete `"cloud_$oldCloudId"` and upsert `"cloud_$newCloudId"` (`cloudId = newCloudId`,
          adjusted `ratio`, `hasThumbnails = false`). Because `MediaViewerViewModel` observes
          `mediaRepository.getTimelineStage2()` directly (bypassing `ObserveTimelineUseCase`'s Stage 1 + 150ms
          debounce), it also replaces `"cloud_$oldCloudId"` → `"cloud_$newCloudId"` immediately in its active pager list
          in lockstep with updating `activeMediaId` + `redirectedIds` so `HorizontalPager` neither jumps nor triggers
          the deleted-item guard, and `TimelineScreen` scroll-syncs to `"cloud_$newCloudId"` on dismiss. Evict
          `oldCloudId` from `MediaDetailsRepository` + caches.
        * **`SYNCED`:** After obtaining `MediaStore.createWriteRequest` consent, call
          `PUT /api/photos/{oldCloudId}/item` → in one Room `@Transaction`, immediately update that Room row
          (`cloudId = newCloudId`, adjusted `ratio`, `hasThumbnails = false`, `autoBackupSuppressed = false`; keeping
          its existing surrogate `id`, whether `"cloud_$oldCloudId"` or `"local_<mediaStoreId>"`, and leaving any
          un-rotated sibling copies on `oldCloudId` to detach on next sync) **and** `deleteById("cloud_$newCloudId")`
          (defense-in-depth in case a concurrent `SyncTimelineUseCase` raced right after `PUT` committed and created a
          duplicate `CLOUD_ONLY` row for `newCloudId`). Then run `LocalFileOverwriteHelper`
          (`GET /api/photos/{newCloudId}/download` → `full_res_cache` → overwrite `localContentUri` → await
          `MediaScannerConnection.scanFile`) and update `sizeBytes` in Room.
        * **`LOCAL_ONLY` (JPEG, PNG, WebP):** After obtaining `MediaStore.createWriteRequest`, open
          `ContentResolver.openFileDescriptor(uri, "rw")` (keeping the `ParcelFileDescriptor` open across
          `ExifInterface(pfd.fileDescriptor).saveAttributes()`), query `MediaStore.MediaColumns.DATA` and trigger/await
          `MediaScannerConnection.scanFile` on that path so `MediaStore`'s `ORIENTATION` column updates promptly
          (preventing `ScanLocalMediaUseCase` from overwriting `ratio` back to the stale orientation), and update
          `ratio` & `sizeBytes` in Room.
        * **Coil cache eviction:** Because Coil 3 `MemoryCache.Key` can carry `extras`, iterate
          `imageLoader.memoryCache?.keys` matching `it.key == "mediastore:$uri" || it.key == uri` to remove all
          variants, and also call `imageLoader.diskCache?.remove(uri)` after rotating a local/synced file so the grid
          updates immediately.
    * **Datetime & Timezone Editing (`EditMediaDateTimeUseCase`):**
        * **Eligibility & UX:** Triggered from tapping the capture date/time or timezone row in `MediaInfoBottomSheet`
          (mirroring web's `EditDateTimeCard.vue`), with explicit Save + loading state rather than optimistic debounce.
          `CLOUD_ONLY` and `SYNCED` require network connectivity (server-first) and apply to both photos and videos
          (`missingSince == null`). Note that server `exiftool` `write_media_datetime` expects QuickTime tags for
          videos and may reject non-QuickTime containers (`.mkv`, `.avi`, `.webm`), so surface HTTP errors cleanly
          without altering Room on failure. `LOCAL_ONLY` is limited to `ExifInterface`-writable photo MIME types
          (`jpeg`, `png`, `webp`—disabled/hidden for `LOCAL_ONLY` videos and HEIC/AVIF/GIF until backed up).
        * **Server side effects (`PUT /api/photos/{cloudId}/item`):** Unlike rotation, `cloudId` does **not** change and
          thumbnails are **not** wiped (`ReprocessScope::MetadataOnly`), but `exiftool` rewrites the source file on disk
          (changing `size_bytes` and BLAKE3 `hash`) and updates `taken_at_local`, `taken_at_utc`, `sort_timestamp`,
          `timezone_offset_seconds`, `weather`, and `updated_at` synchronously before returning `200 OK`.
        * **`CLOUD_ONLY`:** Call `PUT /api/photos/{cloudId}/item` → evict `full_res_cache` and `share_cache` for
          `cloudId` (because `cloudId` stayed the same, cached originals now hold stale EXIF) and invalidate
          `MediaDetailsRepository` → trigger `MediaRefreshCoordinator` delta sync (updating `sortTimestamp`,
          `timezoneOffsetMinutes`, `yearMonth`, and `sizeBytes` in Room). Coil thumbnail caches do not need eviction.
        * **`SYNCED`:**
            * *Photos (`!isVideo`):* Obtain `MediaStore.createWriteRequest` consent before calling
              `PUT /api/photos/{cloudId}/item` (or gate local overwrite on consent), evict `full_res_cache` /
              `share_cache` and `MediaDetailsRepository`, run `LocalFileOverwriteHelper` (`GET /download` →
              `full_res_cache` → overwrite `localContentUri` → await `MediaScannerConnection.scanFile`) so phone
              `size_bytes` and BLAKE3 match the server, and run `MediaRefreshCoordinator` delta sync.
            * *Videos (`isVideo`):* Call `PUT /api/photos/{cloudId}/item` and run `MediaRefreshCoordinator` delta sync +
              evict `share_cache` / `MediaDetailsRepository`, **skipping local file overwrite** to avoid re-downloading
              multi-GB videos over mobile connections. Because `ScanLocalMediaUseCase` preserves server `sortTimestamp`,
              `yearMonth`, and `sizeBytes` on rows where `cloudId != null`, Room stays consistent (with the accepted
              trade-off that a future fresh-app reinstall would not deduplicate that unmodified local video copy against
              the edited cloud hash).
        * **`LOCAL_ONLY` (JPEG, PNG, WebP):** Restrict editing to wall-clock date/time in the device's current timezone
          (disable/hide timezone-offset editing for `LOCAL_ONLY` until backed up, because `MediaStore` does not expose
          EXIF `OffsetTimeOriginal` in its cursor and `LocalMediaMapper` derives `yearMonth` / `timezoneOffsetMinutes`
          from `ZoneId.systemDefault()` on subsequent incremental scans). Obtain `MediaStore.createWriteRequest` →
          open `ContentResolver.openFileDescriptor(uri, "rw")` → write `TAG_DATETIME_ORIGINAL`,
          `TAG_DATETIME_DIGITIZED`, `TAG_DATETIME` (`"yyyy:MM:dd HH:mm:ss"`) **plus** `TAG_OFFSET_TIME_ORIGINAL`,
          `TAG_OFFSET_TIME`, and `TAG_OFFSET_TIME_DIGITIZED` (`±HH:MM` for `ZoneId.systemDefault()` at that instant,
          normalizing `"Z"` to `"+00:00"`, so any existing travel offset in EXIF doesn't cause `MediaProvider` to
          recompute a shifted `DATE_TAKEN`) via `ExifInterface.saveAttributes()` → query `DATA` and await
          `MediaScannerConnection.scanFile` → update `sortTimestamp`, `timezoneOffsetMinutes`, `yearMonth`, `sizeBytes`,
          and reset `blake3Hash = null` in Room.
        * **Live timeline reordering & pager anchoring:** Unlike rotation, changing a photo/video's date moves it to a
          new position (often across months) in Room. Keep `MediaViewerViewModel` observing
          `mediaRepository.getTimelineStage2()` directly (bypassing `ObserveTimelineUseCase`'s Stage 1 + 150ms
          debounce, and waiting for the full list before creating `PagerState` on initial open so opening an older
          item never flashes page `0` or triggers a false deletion guard on Stage 1), anchored by `activeMediaId`. When
          Room emits the reordered list, `HorizontalPager` snaps (`scrollToPage(newIndex)`) to the item's new index
          without triggering the deleted-item guard, and swipe-down dismiss scrolls `TimelineScreen` to
          `findRowIndexForMedia(activeMediaId)`'s new location.
    * **Reprocess (`POST /api/photos/{cloudId}/reprocess`):** Enqueues full worker reprocessing (
      `thumbnails_status = 'pending'`, `updated_at = now()`) without changing `id` (endpoint returns an empty `200 OK`
      body). Invalidate `MediaDetailsRepository` cache and trigger `MediaRefreshCoordinator` delta sync. Because
      reprocess isn't meant to change visual appearance, leaving Coil's `immutable` cached thumbnails intact is
      intentional.
7. **Resumable Uploads via WorkManager:**
    * `UploadWorker` executes TUS resumable uploads in 5 MB chunks. The JWT upload ticket (`upload_token_expiry_minutes:
      5`) is only verified at upload creation, after which TUS uses a persistent upload UUID for `HEAD`/`PATCH` chunks.
      Only `LOCAL_ONLY` items where `auto_backup_suppressed = false` and `device_folder.backup_enabled = 1` are
      auto-enqueued.
    * **Timeout, ticket & stale-URL recovery:** Derive a dedicated upload `OkHttpClient` (via
      `okHttpClient.newBuilder()`, sharing `CookieJar` and `Authenticator`) with longer read/write timeouts (e.g.,
      `120s+`) so TUS part concatenation on finalize (configured up to `600s` in `nginx.conf`) doesn't hit OkHttp's
      default timeout. If resuming via `HEAD`/`PATCH` returns `404`/`410` (expired/cleaned-up server upload), clear
      `tusUploadUrl` and restart from upload ticket creation; if `POST` TUS creation fails with `401`/`403` because a
      previously fetched 5-minute upload ticket expired, request a fresh upload ticket before retrying.
    * **Background vs. Foreground Execution (Android 15–17 limits):** Because `dataSync` Foreground Services enforce a
      **6-hour cumulative cap per 24 hours** and block background starts (`ForegroundServiceStartNotAllowedException`),
      let routine background auto-backups run as **normal constrained `WorkManager` batches**—persisting the TUS upload
      URL and byte offset in Room (`upload_queue`) after every chunk means OS stops after ~10 minutes resume seamlessly
      on the next run without losing progress. Reserve `setForeground(createForegroundInfo())` (guarded with
      `try/catch (ForegroundServiceStartNotAllowedException)`) or Android's User-Initiated Data Transfer (`UIDT`) API
      for explicit user-triggered "Back up now" / manual upload sessions started in the foreground (and verify
      `work-runtime-ktx:2.12.0` against `targetSdk = 37` for `InvalidForegroundServiceTypeException`).
    * Because `IngestMetadata` runs asynchronously on the server worker after upload completion, mark the item in
      `upload_queue` as `UPLOADED_PENDING_SYNC` until `SyncTimelineUseCase` reconciles its `blake3Hash` via
      `POST /api/upload/check-hashes` (and links its `cloudId`), or marks it `FAILED` after a max-age/retry threshold
      if server ingest failed—preventing `EnqueueBackupUseCase` from re-uploading already transferred files.

---
---

### Part 4: Screens & Views Breakdown

| Screen                                   | Core UI Components                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Key Gestures & Interactions                                                                                                                                                                                                                                                                                                                                                                                              |
|:-----------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **`TimelineScreen`**                     | • `LazyColumn` of typed headers + justified rows (cloud items + visible camera folders)<br>• Right-edge `TimelineScrubber` (with `systemGestureExclusion`)<br>• Floating `DateOverlay`<br>• Subtle connectivity/sync indicator<br>• Contextual multi-selection top bar                                                                                                                                                                                                                                                                                                                                                                                                       | • Fast vertical dragging on scrubber via `requestScrollToItem` (with `isScrubbing` image-load gating)<br>• Long-press + drag down to multi-select items<br>• Tap item → open `MediaViewerScreen` overlay<br>• Sync scroll position to active item on viewer dismiss                                                                                                                                                      |
| **`MediaViewerScreen`**                  | • Hosted in a `SharedTransitionLayout` / translucent overlay above `NavHost` (`MainScreen` stays mounted) using `sharedElementWithCallerManagedVisibility` so `TimelineScreen` stays visible underneath during drag-to-dismiss<br>• `HorizontalPager` anchored by `mediaId` (`userScrollEnabled` checks active page `zoomFraction` only)<br>• `Telephoto` `ZoomableAsyncImage` (`1440p` → `full_res_cache` original)<br>• Reused single `ExoPlayer` (`OkHttpDataSource` + `ContentFrame`) on `settledPage`<br>• Floating bottom pill (`Share`, `Add to`, `Bin`)<br>• Top-right `⋮` menu (`About`, `Copy` for photos, `Rotate`, `Reprocess`; video `Playback speed` + `Loop`) | • Double-tap / pinch to zoom (`Telephoto`)<br>• Tap to toggle chrome (`ZoomableAsyncImage` `onClick`)<br>• Single-pointer swipe down (unzoomed, `\|dy\| > \|dx\|`) interactively drags/shrinks over the visible timeline to dismiss<br>• Single-pointer swipe up (unzoomed) to open **Media Info Bottom Sheet** (with tappable date/time & timezone rows to edit, plus resolved location + coordinates + "Open in Maps") |
| **`AlbumsScreen` & `AlbumDetailScreen`** | • `LazyVerticalGrid` (square cards) for album list<br>• Justified row grid for album contents                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | • Tap album card → open album timeline<br>• Add/remove photos to/from albums via selection flow or viewer `Add to`                                                                                                                                                                                                                                                                                                       |
| **`DeviceFoldersScreen`**                | • "On this device" folder grid (`Screenshots`, `Download`, 3rd-party cameras, etc.)<br>• Per-folder **Back up** & **Show in timeline** toggles<br>• Justified row grid for folder contents                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | • Browse non-camera folders without cluttering main timeline<br>• Toggle backup on for a folder (auto-enables timeline visibility) or manually upload individual items                                                                                                                                                                                                                                                   |
| **`BackupSettingsScreen`**               | • Discovered device folder checklist (defaults to `DCIM/*` excluding Screenshots)<br>• Constraint switches (Wi-Fi only, while charging)<br>• Live upload queue progress bar                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | • Toggle auto-backup globally or per folder<br>• "Sync Now" manual trigger                                                                                                                                                                                                                                                                                                                                               |
| **`AuthScreen`**                         | • Server URL input + LAN discovery scanner<br>• Email & password inputs                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | • Validates connection via `GET /api/health`<br>• Stores session cookies in persistent `CookieJar` across app launches                                                                                                                                                                                                                                                                                                   |
