PURPOSE OF THE DOC

Purpose is to show a general idea of the breadth of the application. It's not a hard "in stone" written fact that the
things written here should be implemented as such. During implementation I'm sure we'll encounter problems that will
make another solution that what's proposed here a cleaner architecture. Just go with that, don't think these are sacred
texts.

### Part 1: The Justified Horizontal Row Grid

The gallery layout is a **Justified Horizontal Row Layout** (linear partition), identical in principle to Google Photos
and the Ruurd Photos web app (`TimelineContainer.vue`). Every row is scaled so that items fit flush from screen edge to
edge with uniform gaps.

---

#### 1. Why `LazyColumn` Beats `LazyVerticalGrid` for Justified Layouts

`LazyVerticalGrid` is designed for fixed or adaptive column counts with rigid aspect-ratio spans. It forces irregular
span math and causes measurement recalculations during scrolling.

Instead, the cleanest and most performant pattern in Jetpack Compose is **a `LazyColumn` of precomputed rows**:

* Each item in the `LazyColumn` represents either a **Month Header** or a **Row of 2–5 photos**.
* In Compose, a row item is simply a standard `Row` containing photo composables whose widths are proportioned by their
  aspect ratio using explicit DP width or `Modifier.weight(aspectRatio)`, constrained to the precalculated row height.
* **Why this is fast:** Compose treats each row as a single virtualized element. Measurement overhead is virtually zero
  during scroll because row heights and aspect ratios are precomputed on background threads.

```
LazyColumn
 ├── Item 0: MonthHeaderUiItem("October 2026")
 ├── Item 1: TimelineRowUiItem(height = 180.dp, items = [PhotoA, PhotoB, PhotoC])
 ├── Item 2: TimelineRowUiItem(height = 195.dp, items = [PhotoD, PhotoE])
 └── ...
```

---

#### 2. Layout Calculation Engine (`TimelineLayoutCalculator`)

Just like the web app's `calculateLayout` in `TimelineContainer.vue`, the math runs strictly off the UI thread (
`Dispatchers.Default`) when the window width, orientation, or density changes. Because Room acts as the Single Source of
Truth, the layout calculator receives fully resolved entities directly.

```kotlin
data class TimelineRowUiItem(
    val id: String, // Stable key, e.g. "row_${monthId}_${items.firstOrNull()?.id ?: index}"
    val heightDp: Float,
    val items: List<TimelineGridItemUiModel>,
    val monthId: String,
    val isFirstOfMonth: Boolean,
    val isLastOfMonth: Boolean,
    val estimatedOffsetTopDp: Float,
    val thumbnailTargetHeight: Int // Ladder height (e.g. 144, 240, 360, 480)
)

data class TimelineGridItemUiModel(
    val id: String,
    val localContentUri: String?,
    val cloudId: String?,
    val isVideo: Boolean,
    val durationMs: Int,
    val hasThumbnails: Boolean,
    val timestamp: Long,
    val ratio: Float,
    val syncStatus: SyncStatus
)
```

**Row Partitioning Algorithm:**

1. Target desired row height: `180.dp` on phone portrait, `240.dp` on tablet/landscape, scaled by user zoom settings.
2. Accumulate items into the current row:
   $$\text{targetWidth} = \sum (\text{aspectRatio}_i \times \text{desiredHeight}) + (N - 1) \times \text{gap}$$
3. When $\text{targetWidth} \ge \text{containerWidth}$:
   $$\text{scaleFactor} = \frac{\text{containerWidth} - \text{totalGaps}}{\sum (\text{aspectRatio}_i \times \text{desiredHeight})}$$
   $$\text{actualRowHeight} = \text{clamp}(\text{desiredHeight} \times \text{scaleFactor}, \; \text{minHeight}, \; \text{desiredHeight} \times 1.5)$$
4. Finalize the row, compute its cumulative vertical offset (`offsetTop`), and advance.
5. If the month boundary ends on an incomplete row, cap its scale factor to `1.0` so the last 1 or 2 items don't stretch
   into giant blocks.

---

#### 3. Unifying Local MediaStore + Cloud Protobuf: Dedicated Compact Sync & Room SSOT

Instead of the web app's anonymous ratio arrays and multi-stage month hydration, the mobile client uses a **Dedicated
Compact Protobuf Sync Endpoint** (to be implemented) paired with **Room as the Single Source of Truth (SSOT)**. This
guarantees full offline capability and instant local-to-cloud reconciliation without UI placeholders.

**Tombstone Deletion Tracking:** Permanent deletions are recorded in a lightweight `deleted_media_item` (tombstone,
populated by db trigger) table with the purged IDs and their deletion timestamp. The delta sync endpoint checks
`deleted_at > since` alongside`updated_at > since` to return `deleted_ids`, allowing the mobile client to purge removed
items from Room without re-fetching the entire library.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                 Dedicated Compact Sync (Initial & Delta)                    │
│  • Remote: GET /api/mobile/timeline/sync?since=... (Compact Protobuf)       │
│    Payload: [id, sort_timestamp, ratio, is_video, has_thumbnails, size_bytes]│
│    Size: ~24 bytes/photo (~380 KB gzipped for 50,000 photos)                │
│  • Writes directly to Room `media_item` table                               │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                 Room Database: Single Source of Truth (SSOT)                │
│  • Houses local-only, cloud-only, and merged/synced items                   │
│  • Deduplication via zero-hash fast pass: (size_bytes, timestamp)           │
│  • Feeds CalculateTimelineLayoutUseCase directly on Dispatchers.Default     │
│  • 100% offline-ready: entire timeline renders instantly from SQLite       │
└─────────────────────────────────────────────────────────────────────────────┘
```

##### Sync Protocol & Data Transfer

1. **Compact Timeline Protobuf (`timeline.proto`):**
   ```protobuf
    message TimelineSyncResponse {
    // New or modified media items to be upserted into Room
    repeated CompactTimelineItem upserted = 1;
    // IDs of items moved to Bin/Trash or permanently purged via tombstones
    repeated string deleted_ids = 2;
    // Server timestamp (epoch ms) to persist and pass as `since` in the next sync request
    int64 sync_timestamp = 3;
    }
    
    message CompactTimelineItem {
    string id = 1;                  // 10-char alphanumeric ID
    int64 sort_timestamp = 2;       // Epoch millis for deterministic chronological sorting
    float ratio = 3;                // width / height aspect ratio
    bool is_video = 4;
    bool has_thumbnails = 5;        // True if server pre-generated AVIF thumbnails are ready
    int64 size_bytes = 6;           // Used for instant, zero-hash MediaStore deduplication
    optional int32 duration_ms = 7; // Video duration in milliseconds (if video)
    }
   ```
2. **Delta Syncing (`since` parameter):**
    * **Initial Launch:** Query `GET /api/mobile/timeline/sync` without `since`. Backend streams all non-deleted items.
      Room bulk-inserts them in a single transaction and stores `latestSyncTimestamp`.
    * **Subsequent Launches:** Query `GET /api/mobile/timeline/sync?since={latestSyncTimestamp}`. Only items created,
      updated, or soft-deleted since the last sync are returned (typically a few KB).
    * To handle possible out of sync situations, do a full sync (without ?size) periodically (weekly?), and on pull to
      refresh

##### Two-Pass Local/Cloud Deduplication Strategy

To prevent cold-start battery drain and disk thrashing, **the mobile app never hashes local photos on startup**:

1. **Pass 1: Instant Fast-Pass (Zero Hashing):**
    * Query `MediaStore` for `_ID`, `WIDTH`, `HEIGHT`, `ORIENTATION`, `SIZE`, and `DATE_TAKEN`.
    * Cross-reference against Room using:
      ```sql
      SELECT * FROM media_item 
      WHERE size_bytes = :localFileSize 
        AND ABS(sort_timestamp - :localDateTaken) < 2000
      LIMIT 1;
      ```
    * A match instantly links `localContentUri` to `cloudId` and marks `syncStatus = SYNCED`. This resolves >99.9% of
      items with zero disk I/O.
2. **Pass 2: On-Demand Crypto Verification (Deferred to Backup/Upload):**
    * For local items that did *not* match in Pass 1 (new or modified local files), SHA-256 is calculated **only when
      the item is staged for backup** by `UploadWorker` using ARMv8 hardware acceleration.
    * Before streaming bytes, a quick pre-upload check (`POST /api/upload/check-hashes`) queries the backend with the
      hash. If the backend already possesses it, the upload is skipped and the item is marked `SYNCED`.

##### App Launch Flow:

* **T = 0ms:** Read existing records from Room. Compute justified layout on `Dispatchers.Default`. The timeline renders
  immediately with full metadata, even in airplane mode.
* **T + 50ms:** `MediaStore` scan reconciles new or deleted local photos via the zero-hash fast pass.
* **T + 150ms:** Background delta sync calls `GET /api/mobile/timeline/sync?since=...`. Any cloud changes are committed
  to Room, triggering an incremental UI layout update.

---

#### 4. Instant Fast-Scrubbing with Variable Row Heights

Because each row has a slightly different height, native scroll thumbs can jitter. Solve this with a **Layout Index**:

```kotlin
class TimelineLayoutIndex(
    val rows: List<TimelineRowUiItem>,
    val totalHeightDp: Float,
    val monthOffsets: Map<String, Int> // monthId -> LazyColumn row index
) {
    fun findRowIndexAtPercentage(fraction: Float): Int {
        val targetOffset = fraction * totalHeightDp
        // Binary search rows by estimatedOffsetTopDp: O(log N)
        val idx = rows.binarySearchBy(targetOffset) { it.estimatedOffsetTopDp }
        return if (idx < 0) (-idx - 1).coerceIn(0, rows.lastIndex) else idx
    }

    fun findIndexForDate(yearMonth: String): Int = monthOffsets[yearMonth] ?: 0
}
```

### Part 2: Android App Architecture

Adopt **Clean Architecture + MVI/MVVM** with **Unidirectional Data Flow (UDF)**.

```
┌────────────────────────────────────────────────────────────┐
│                        UI Layer                            │
│   Compose Screens (Timeline, Viewer, Albums, Settings)     │
│   ViewModels (StateFlow<UiState>, UiEvent channels)        │
└──────────────────────────────┬─────────────────────────────┘
                               │
┌──────────────────────────────▼─────────────────────────────┐
│                      Domain Layer                          │
│   UseCases (CalculateLayout, SyncTimelineUseCase,          │
│   DeduplicateMediaUseCase, EnqueueUploadUseCase)           │
│   Domain Models (MediaItem, SyncStatus, TimelineRow)       │
└──────────────────────────────┬─────────────────────────────┘
                               │
┌──────────────────────────────▼─────────────────────────────┐
│                       Data Layer                           │
│  Repositories (MediaRepository, BackupRepository)          │
│  Local: Room DB, MediaStore DataSource, Layout Index       │
│  Remote: OkHttp + Persistent CookieJar, Compact Sync API   │
│  Background: WorkManager (UploadWorker, LocalScanWorker)   │
└────────────────────────────────────────────────────────────┘
```

#### Key Architecture Principles for this App:

1. **Cookie-Based Session Authentication & Persistent CookieJar:**
    * Matches the web client (`web/src/scripts/services/api.ts`) by relying exclusively on HTTP cookies (`access_token`,
      `refresh_token`) set by the backend on `POST /api/auth/login`.
    * OkHttp is configured with a persistent `CookieJar` (backed by encrypted SharedPreferences or DataStore).
    * Every network call—including Retrofit API requests, Coil thumbnail image fetches, and TUS file uploads—shares this
      exact OkHttp client instance and automatically includes the session cookies.
    * Transparent session refresh: An OkHttp `Interceptor` or 401 response handler triggers `POST /api/auth/refresh`
      when an access cookie expires, updating the cookie store seamlessly without re-prompting the user.
2. **Room Database as Single Source of Truth (SSOT):**
    * Room stores complete `MediaEntity` rows (local-only items, synced items, and cloud-only items).
    * The dedicated compact sync endpoint populates cloud metadata, while `MediaStore` queries populate local metadata
      into the same table.
    * Layout calculations run directly over Room data, enabling a 100% offline-first architecture with zero UI
      placeholder jumping.
3. **Local vs Cloud Identity & Two-Pass Deduplication:**
    * Media items in Room contain:
        * `localContentUri`: `content://media/external/images/media/1234` (nullable)
        * `cloudId`: Backend 10-char alphanumeric string `a1B2c3D4e5` (nullable)
        * `sizeBytes`: File size in bytes (from `CompactTimelineItem` or `MediaStore`)
        * `sha256`: SHA-256 file hash (nullable, computed lazily)
        * `syncStatus`: `LOCAL_ONLY`, `SYNCING`, `SYNCED`, `CLOUD_ONLY`
    * **Matching Pipeline:**
        * *Fast Pass:* On startup/scan, instantly match local items against Room cloud records using
          `(date_taken, size_bytes)`. Matched items flip to `SYNCED` with zero disk hashing.
        * *Lazy Crypto Verification:* For unmatched local items queued for upload, calculate SHA-256 via
          hardware-accelerated `MessageDigest` and verify against the backend prior to streaming bytes.
4. **Coil 3 Thumbnail Strategy:**
    * **Target SDK:** (Android 17 and up (API level 37)) provides native hardware-accelerated AVIF decoding via `ImageDecoder`.
    * If `syncStatus == LOCAL_ONLY`: Image source is `localContentUri` (instant system thumbnail).
    * If `syncStatus == CLOUD_ONLY`:
        * If `hasThumbnails == true`: Image source is `"${BASE_URL}/thumbnails/${cloudId}/${targetHeight}p.avif"` cached
          in Coil's disk LRU cache.
        * If `hasThumbnails == false` (ingest in progress): Image source is
          `"${BASE_URL}/api/photos/${cloudId}/thumbnail?size=${targetHeight}"`
    * If `syncStatus == SYNCED`: Prefer `localContentUri` while on device storage; fall back to cloud thumbnail URL if
      deleted locally.
5. **Resumable Uploads via Foreground WorkManager:**
    * `UploadWorker` executes TUS resumable uploads in 5 MB chunks (tus uploads work with JWT ticket).
    * To prevent the OS from killing transfers after 10 minutes, the worker calls
      `setForeground(createForegroundInfo())` to run as an Android Foreground Service with an ongoing notification.
    * The upload offset is persisted in Room after each chunk, ensuring seamless resume on network drops.

---

### Part 3: Recommended Package Structure

```text
app/src/main/java/com/ruurd/photos/
├── core/
│   ├── common/             # Dispatchers, Result wrapper, Extensions
│   ├── database/           # Room Database, TypeConverters
│   │   ├── dao/            # MediaDao, AlbumDao, UploadQueueDao
│   │   └── entity/         # MediaEntity, AlbumEntity, UploadQueueEntity
│   ├── network/            # OkHttp, Retrofit, PersistentCookieJar, SessionManager
│   │   ├── api/            # PhotosApiService (Retrofit Protobuf Lite)
│   │   ├── cookie/         # PersistentCookieJar implementation (DataStore-backed)
│   │   └── tus/            # Resumable TUS upload client
│   └── ui/                 # Shared UI components, Theme (M3 Dynamic Color), Dimens
│       └── components/     # ThumbnailImage, FastScroller, DateOverlay
│
├── data/                   # Repositories & Data Sources
│   ├── repository/
│   │   ├── MediaRepositoryImpl.kt
│   │   ├── AlbumRepositoryImpl.kt
│   │   ├── BackupRepositoryImpl.kt
│   │   └── AuthRepositoryImpl.kt
│   └── datasource/
│       ├── local/          # MediaStoreContentObserver, LocalMediaScanner
│       └── remote/         # RemoteMediaDataSource (Compact Protobuf Sync)
│
├── domain/                 # Pure Kotlin business logic & UseCases
│   ├── model/              # MediaItem, SyncStatus, TimelineRowUiItem, Album
│   ├── repository/         # Repository Interfaces
│   └── usecase/
│       ├── CalculateTimelineLayoutUseCase.kt
│       ├── SyncTimelineUseCase.kt
│       ├── ScanLocalMediaUseCase.kt
│       └── EnqueueBackupUseCase.kt
│
├── work/                   # WorkManager background workers
│   ├── UploadWorker.kt     # TUS auto-upload execution (Foreground Service)
│   └── LocalScanWorker.kt  # Periodic/Event-driven MediaStore reconciliation
│
└── features/               # Presentation layer (Compose UI + ViewModels)
    ├── auth/               # Login screen, Server URL configuration
    ├── timeline/           # Justified row gallery view
    │   └── components/     # JustifiedRow.kt, TimelineScrubber.kt, SelectionActionBar.kt
    ├── viewer/             # Media viewer (Pinch-to-zoom, Swipe dismiss)
    │   └── components/     # ZoomableImage.kt, VideoPlayer.kt, MediaInfoBottomSheet.kt
    ├── albums/             # Albums listing & album detail views
    └── backup/             # Backup settings & status
```

---

### Part 4: Screens & Views Breakdown

| Screen                                   | Core UI Components                                                                                                                                                                         | Key Gestures & Interactions                                                                                                                                                            |
|:-----------------------------------------|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **`TimelineScreen`**                     | • `LazyColumn` of justified rows<br>• Right-edge `TimelineScrubber`<br>• Floating `DateOverlay`<br>• Contextual multi-selection top bar                                                    | • Fast vertical dragging on scrubber to jump dates (with `isScrollingFast` blanking)<br>• Long-press + drag down to multi-select items<br>• Tap item $\to$ navigate into `MediaViewer` |
| **`MediaViewerScreen`**                  | • `HorizontalPager` for smooth swiping<br>• `Telephoto` zoomable photo viewport<br>• Reused single `ExoPlayer` instance for video items<br>• Bottom action bar (Share, Edit, Delete, Info) | • Double-tap to zoom, pinch-to-zoom<br>• Swipe down to dismiss back to timeline<br>• Swipe up to expand metadata / EXIF / weather sheet                                                |
| **`AlbumsScreen` & `AlbumDetailScreen`** | • `LazyVerticalGrid` (square cards) for album list<br>• Justified row grid for album contents                                                                                              | • Tap album card $\to$ open album timeline<br>• Add photos to album via selection flow                                                                                                 |
| **`BackupSettingsScreen`**               | • Device folder checklist (DCIM, WhatsApp, Downloads)<br>• Constraint switches (Wi-Fi only, while charging)<br>• Live upload queue progress bar                                            | • Toggle auto-backup<br>• "Sync Now" manual trigger                                                                                                                                    |
| **`AuthScreen`**                         | • Server URL input (defaults to self-hosted domain)<br>• Email & password inputs                                                                                                           | • Validates connection via `GET /api/health`<br>• Stores session cookies in persistent `CookieJar` across app launches                                                                 |

---

### Part 5: Recommended Third-Party Libraries

* **Image Loading:** `io.coil-kt.coil3:coil-compose:3.1.0` (Native Compose integration, supports coroutines, handles
  `content://`, `http://`, and AVIF decoding via `ImageDecoder`).
* **Zoom Gestures in Viewer:** `me.saket.telephoto:zoomable-image-coil:0.14.0` (Compose zoomable image with fling,
  inertia, and nested scrolling cooperation with `HorizontalPager`).
* **Video Playback:** `androidx.media3:media3-exoplayer:1.5.1` & `androidx.media3:media3-ui:1.5.1`
* **Protobuf Compilation:** `com.google.protobuf:protobuf-kotlin-lite:4.30.0` with the Gradle Protobuf plugin to compile
  `timeline.proto`.
* **Navigation:** `androidx.navigation:navigation-compose:2.8.8` (Type-safe Compose navigation routes via
  `@Serializable` classes).
* **Local DB:** `androidx.room:room-ktx:2.7.0` (Configured with SQLite WAL mode for non-blocking concurrent
  reads/writes).
* **Background Tasks:** `androidx.work:work-runtime-ktx:2.10.0` (WorkManager with Foreground Service support).