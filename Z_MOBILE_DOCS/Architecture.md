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
│   DeduplicateMediaUseCase, EnqueueUploadUseCase, Rotate)     │
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

1. **Cookie-Based Session Authentication & Persistent CookieJar:**
    * Matches the web client (`web/src/scripts/services/api.ts`) by relying exclusively on HTTP cookies (`access_token`,
      `refresh_token`) set by the backend on `POST /api/auth/login`.
    * OkHttp is configured with a persistent `CookieJar` (backed by encrypted SharedPreferences or DataStore).
    * Every network call—including Retrofit API requests, Coil thumbnail image fetches, full-res downloads, Media3 video
      streaming, and TUS
      upload initiation—shares this exact `OkHttpClient` instance and automatically includes the session cookies.
    * Transparent session refresh: A mutex-guarded OkHttp `Authenticator` triggers `POST /api/auth/refresh` when an
      access cookie expires, updating the cookie store seamlessly without re-prompting the user.
2. **Room Database as Single Source of Truth (SSOT) & Refresh Coordination:**
    * Room stores complete `MediaEntity` rows (local-only items, synced items, and cloud-only items), including
      precomputed `yearMonth`, `timezoneOffsetMinutes`, `isVisibleInTimeline`, `autoBackupSuppressed`, and local folder
      metadata (`bucketId`, `bucketName`, `relativePath`), alongside `DeviceFolderEntity` for per-folder timeline
      visibility and backup rules.
    * Instead of a manual web-style `refreshStore.counter++`, **Room `Flow` is the UI refresh mechanism**: any local
      mutation in Room (rotation, download linking, deletion, album update, sync) propagates automatically to every
      observing screen.
    * A lightweight **`MediaRefreshCoordinator`** in `AppContainer` handles side-effect refreshes after async server
      mutations (like `POST /api/photos/{id}/reprocess` or `PUT /api/photos/{id}/item`): invalidating the in-memory
      `/item` details cache + Coil caches for that ID and running `SyncTimelineUseCase`.
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
    * **Permissions:** Request `READ_MEDIA_IMAGES` and `READ_MEDIA_VIDEO`, and handle Android 14+ partial access
      (`READ_MEDIA_VISUAL_USER_SELECTED`) cleanly so granting partial access does not break or loop permission prompts;
      track full vs. partial access per media type so cursor-omission deletion only runs when full access is granted.
    * **Matching Pipeline:**
        * *Fast Pass:* On startup/scan, query `CLOUD_ONLY` rows by `size_bytes` and match in Kotlin on `ratio` + exact
          or 15-minute timezone-aligned timestamp (`±2s` within `±24h`). Matched items flip to `SYNCED` with zero disk
          hashing.
        * *Lazy Crypto Verification:* For unmatched local items staged for upload (including duplicate local copies and
          rare `size_bytes` collisions with >24h timestamp drift), compute BLAKE3 lazily and verify against the backend
          prior to streaming bytes.
4. **Coil 3 Thumbnail Strategy & Separate Viewer Full-Res Cache:**
    * **Target & Min SDK:** Targeting Android 15 and up (API level 35) provides native
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
    * **Viewer Full-Res File Cache (`cacheDir/full_res_cache/`, ~250–300 MB LRU):**
        * Coil 3 does not support multiple disk-cache budgets inside one `ImageLoader`. Keeping `PhotosImageLoader`'s 1
          GB `image_cache` strictly for thumbnails prevents large 20–50 MB `/api/photos/{cloudId}/download` originals
          from evicting timeline thumbnails.
        * For `CLOUD_ONLY` photos (when MIME is natively decodable: JPEG, PNG, WebP, GIF, AVIF, HEIC/HEIF — skip
          TIFF/TGA), the viewer displays `1440p` immediately and fetches `/api/photos/{cloudId}/download` after a short
          delay (or immediately on zoom) into `full_res_cache`, passing the local `File` (
          `SubSamplingImageSource.file` / `file://` with `diskCachePolicy(CachePolicy.DISABLED)`) to
          `ZoomableAsyncImage` (preserving Ultra HDR gainmaps).
        * **Share**, **Copy**, and **Download to `Downloads`** reuse the exact same `full_res_cache` file without
          downloading twice.
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
    * **Surface ownership in `HorizontalPager`:** Because `HorizontalPager` composes adjacent offscreen pages, attach
      the single shared `ExoPlayer`'s `PlayerSurface` only to the **currently settled page**
      (`pagerState.settledPage == pageIndex`), rendering the photo thumbnail underneath until the first video frame
      renders (`onRenderedFirstFrame`) so adjacent pages don't fight over the `Surface`. Pause on `ON_STOP`.
    * **Source selection in `MediaViewerScreen`:**
        * **Local / Synced (`localContentUri != null`):** Play directly from `localContentUri` (instant, zero
          bandwidth, full quality), falling back to cloud playback if `ExoPlayer` encounters `SecurityException` or
          `FileNotFoundException`.
        * **Cloud-only (`CLOUD_ONLY`):**
            * **`hasThumbnails == false`:** Thumbnails and WebM transcodes have not been generated yet, so **only** the
              on-demand original-quality endpoint (`/api/photos/{id}/video`, HTTP `Range`-enabled, requires auth) can be
              used.
            * **`hasThumbnails == true`:** Default to **Original** (`/api/photos/{id}/video`) when on unmetered Wi-Fi
              without Data Saver (`ConnectivityManager.isActiveNetworkMetered` + `restrictBackgroundStatus`), and
              default to **`480p.webm`** otherwise. Offer a quality picker (`Original`, `480p`, `144p`, reading
              `sourceHeight` from `GET /api/photos/{id}/item` once loaded), and automatically fall back to `480p.webm`
              if `Original` fails with a codec/container `PlaybackException`. Note `/thumbnails/...` does **not**
              require authentication.
    * **Video menu preferences:** Persist **Loop video** (`REPEAT_MODE_ONE` vs `REPEAT_MODE_OFF`) and **Playback speed
      ** (`0.25x`–`3.0x`) in DataStore, controlled from the viewer's top-right overflow menu.
6. **Rotation & Reprocess Architecture:**
    * **Server rotation side effects (`PUT /api/photos/{oldId}/item`):** `apply_orientation_update` rewrites the file
      with `exiftool`, wipes thumbnails (`has_thumbnails = false`), and calls `MediaItemStore::change_media_item_id`,
      generating a brand-new `newId` (which inserts `newId` and deletes `oldId`, emitting a `deleted_media_item`
      tombstone for `oldId`).
    * **Debounce & flush:** Rotating taps increment visual rotation immediately (`0° → 90° → 180° → 270°`) with a 2.5s
      debounce before committing; if the user swipes to another item or exits the viewer before the timer fires, flush
      the pending rotation immediately.
    * **Upload guard:** Block/disable Rotate while an upload is active for that item in Phase 3's `upload_queue`.
    * **Handling per `SyncStatus`:**
        * **`CLOUD_ONLY`:** Call `PUT /api/photos/{oldId}/item` → in one Room `@Transaction`, delete `"cloud_$oldId"`
          and upsert `"cloud_$newId"` (`cloudId = newId`, swapped `ratio`, `hasThumbnails = false`), update
          `MediaViewerViewModel`'s active `mediaId` (`oldId → newId`), evict `oldId` from `MediaDetailsRepository` +
          caches, and fetch `GET /api/photos/{newId}/download` into `full_res_cache`.
        * **`SYNCED`:** Request OS write permission via `MediaStore.createWriteRequest` **before** calling
          `PUT /api/photos/{oldId}/item` (aborting cleanly if denied). When `PUT` returns `newId`, immediately update
          the Room row (`cloudId = newId`, swapped `ratio`, `hasThumbnails = false`) so any concurrent
          `SyncTimelineUseCase` sees `oldId` gone and ignores its tombstone. Then download
          `GET /api/photos/{newId}/download` into `full_res_cache` and overwrite `localContentUri` via
          `ContentResolver.openFileDescriptor(uri, "wt")`, updating `sizeBytes` in Room—keeping the phone's file
          byte-for-byte identical to the server without setting `autoBackupSuppressed`.
        * **`LOCAL_ONLY` (JPEG, PNG, WebP):** Request `MediaStore.createWriteRequest`, update
          `ExifInterface.TAG_ORIENTATION` + `saveAttributes()`, and update `ratio` & `sizeBytes` in Room.
    * **Coil cache eviction:** Because `MediaStoreThumbnailFetcher.ThumbnailKeyer` keys on `"mediastore:$uri"` (and
      Coil's default `UriKeyer` keys on `"$uri"`), explicitly remove both keys from `imageLoader.memoryCache` after
      rotating a local/synced file so the grid updates immediately.
    * **Reprocess (`POST /api/photos/{id}/reprocess`):** Enqueues full worker reprocessing (
      `thumbnails_status = 'pending'`, `updated_at = now()`) without changing `id`. Invalidate `MediaDetailsRepository`
      cache and trigger `MediaRefreshCoordinator` delta sync.
7. **Resumable Uploads via WorkManager:**
    * `UploadWorker` executes TUS resumable uploads in 5 MB chunks. The JWT upload ticket (`upload_token_expiry_minutes:
      5`) is only verified at upload creation, after which TUS uses a persistent upload UUID for `HEAD`/`PATCH` chunks.
      Only `LOCAL_ONLY` items where `auto_backup_suppressed = false` and `device_folder.backup_enabled = 1` are
      auto-enqueued.
    * **Timeout & stale-URL recovery:** Derive a dedicated upload `OkHttpClient` (via `okHttpClient.newBuilder()`,
      sharing `CookieJar` and `Authenticator`) with longer read/write timeouts (e.g., `120s+`) so TUS part concatenation
      on finalize (configured up to `600s` in `nginx.conf`) doesn't hit OkHttp's default 10s timeout. If resuming via
      `HEAD`/`PATCH` returns `404`/`410` (expired/cleaned-up server upload), clear `tusUploadUrl` and restart from
      upload ticket creation.
    * **Background vs. Foreground Execution (Android 15–17 limits):** Because `dataSync` Foreground Services enforce a
      **6-hour cumulative cap per 24 hours** and block background starts (`ForegroundServiceStartNotAllowedException`),
      let routine background auto-backups run as **normal constrained `WorkManager` batches**—persisting the TUS upload
      URL and byte offset in Room (`upload_queue`) after every chunk means OS stops after ~10 minutes resume seamlessly
      on the next run without losing progress. Reserve `setForeground(createForegroundInfo())` (guarded with
      `try/catch (ForegroundServiceStartNotAllowedException)`) or Android's User-Initiated Data Transfer (`UIDT`) API
      for explicit user-triggered "Back up now" / manual upload sessions started in the foreground.
    * Because `IngestMetadata` runs asynchronously on the server worker after upload completion, mark the item in
      `upload_queue` as `UPLOADED_PENDING_SYNC` until `SyncTimelineUseCase` reconciles its `blake3Hash` via
      `POST /api/upload/check-hashes` (and links its `cloudId`), or marks it `FAILED` after a max-age/retry threshold
      if server ingest failed—preventing `EnqueueBackupUseCase` from re-uploading already transferred files.

---
---

### Part 4: Screens & Views Breakdown

| Screen                                   | Core UI Components                                                                                                                                                                                                                                                                                                                                                                                                                  | Key Gestures & Interactions                                                                                                                                                                                                                             |
|:-----------------------------------------|:------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **`TimelineScreen`**                     | • `LazyColumn` of typed headers + justified rows (cloud items + visible camera folders)<br>• Right-edge `TimelineScrubber` (with `systemGestureExclusion`)<br>• Floating `DateOverlay`<br>• Subtle connectivity/sync indicator<br>• Contextual multi-selection top bar                                                                                                                                                              | • Fast vertical dragging on scrubber via `scrollToItem` (with `isScrubbing` image-load gating)<br>• Long-press + drag down to multi-select items<br>• Tap item → navigate into `MediaViewer`<br>• Sync scroll position to active item on viewer dismiss |
| **`MediaViewerScreen`**                  | • `HorizontalPager` anchored by `mediaId` (`userScrollEnabled` locked while zoomed)<br>• `Telephoto` `ZoomableAsyncImage` (`1440p` → `full_res_cache` original)<br>• Reused single `ExoPlayer` (`OkHttpDataSource`) on `settledPage`<br>• Floating bottom pill (`Share`, `Add to`, `Bin`)<br>• Top-right `⋮` menu (`Download` for `CLOUD_ONLY`, `About`, `Copy` for photos, `Rotate`, `Reprocess`; video `Playback speed` + `Loop`) | • Double-tap / pinch to zoom (`Telephoto`)<br>• Tap to toggle chrome<br>• Swipe down (unzoomed) to dismiss back to timeline<br>• Swipe up (unzoomed) to open **Media Info Bottom Sheet** (no map yet; resolved location + coordinates + "Open in Maps") |
| **`AlbumsScreen` & `AlbumDetailScreen`** | • `LazyVerticalGrid` (square cards) for album list<br>• Justified row grid for album contents                                                                                                                                                                                                                                                                                                                                       | • Tap album card → open album timeline<br>• Add/remove photos to/from albums via selection flow or viewer `Add to`                                                                                                                                      |
| **`DeviceFoldersScreen`**                | • "On this device" folder grid (`Screenshots`, `Download`, 3rd-party cameras, etc.)<br>• Per-folder **Back up** & **Show in timeline** toggles<br>• Justified row grid for folder contents                                                                                                                                                                                                                                          | • Browse non-camera folders without cluttering main timeline<br>• Toggle backup on for a folder (auto-enables timeline visibility) or manually upload individual items                                                                                  |
| **`BackupSettingsScreen`**               | • Discovered device folder checklist (defaults to `DCIM/*` excluding Screenshots)<br>• Constraint switches (Wi-Fi only, while charging)<br>• Live upload queue progress bar                                                                                                                                                                                                                                                         | • Toggle auto-backup globally or per folder<br>• "Sync Now" manual trigger                                                                                                                                                                              |
| **`AuthScreen`**                         | • Server URL input + LAN discovery scanner<br>• Email & password inputs                                                                                                                                                                                                                                                                                                                                                             | • Validates connection via `GET /api/health`<br>• Stores session cookies in persistent `CookieJar` across app launches                                                                                                                                  |


