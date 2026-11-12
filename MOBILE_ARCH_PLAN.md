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
`Dispatchers.Default`) when the window width, orientation, or density changes.

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

sealed interface TimelineGridItemUiModel {
    val ratio: Float

    // Phase 1: Known geometry from ratios endpoint, metadata still loading
    data class Placeholder(
        val monthId: String,
        val indexInMonth: Int,
        override val ratio: Float
    ) : TimelineGridItemUiModel

    // Phase 2: Metadata loaded (local or cloud)
    data class Loaded(
        val id: String,
        val localContentUri: String?,
        val cloudId: String?,
        val isVideo: Boolean,
        val durationMs: Int,
        val hasThumbnails: Boolean,
        val timestamp: Long,
        override val ratio: Float,
        val syncStatus: SyncStatus
    ) : TimelineGridItemUiModel
}
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

#### 3. Unifying Local MediaStore + Cloud Protobuf: The Two-Tier Strategy

To render the timeline immediately without waiting for thousands of metadata rows or downloading bitmaps, data is split
into two distinct tiers (mirroring the web app's architecture):

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Tier 1: Structure & Ratios                            │
│  • Remote: GET /api/timeline/ratios (Protobuf, ~50ms, anonymous floats)     │
│  • Local: MediaStore width/height/orientation converted to aspect ratios    │
│  • Cache: Lightweight disk/DataStore cache                                  │
│  • Result: Total scroll height, row count, & layout boxes ready at T = 0ms  │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Tier 2: Item Metadata & Media Details                    │
│  • Remote: GET /api/timeline/by-month (Protobuf, lazy per visible month)     │
│  • Local: MediaStore local scan cached in Room                              │
│  • Result: Hydrates Placeholder items into Loaded items as user scrolls     │
└─────────────────────────────────────────────────────────────────────────────┘
```

##### Tier 1: Structure & Layout Partitioning

1. **Remote Cloud Items:** `GET /api/timeline/ratios` provides `TimelineRatiosResponse` with float aspect ratios grouped
   by month. This payload is tiny and downloads in tens of milliseconds.
2. **Local Device Items:** `MediaStore.Images.Media` and `MediaStore.Video.Media` provide `WIDTH`, `HEIGHT`, and
   `ORIENTATION`.
    * *Critical EXIF quirk:* If `orientation == 90 || orientation == 270`, swap `width` and `height` before
      calculating $\text{ratio} = \frac{\text{width}}{\text{height}}$.
3. **Structure Cache:** Cache month IDs and their float ratio lists locally.
4. **App Launch Flow:**
    * **T = 0ms:** Load cached ratio structure from disk. Compute row layout on `Dispatchers.Default`. The grid renders
      immediately with accurate row heights and thumbnail placeholder boxes.
    * **T + 100ms:** Local `MediaStore` scan checks for newly snapped photos. Any new items are inserted into the local
      ratio list.
    * **T + 300ms:** Remote sync fetches `GET /api/timeline/ratios` (Protobuf). If counts or ratio hashes changed,
      update the structure cache and trigger a re-partitioning pass.

##### Tier 2: Item Hydration via Lazy Loading

* When a row enters the viewport:
    * For local photos: Room already holds full metadata from the MediaStore scan.
    * For cloud photos: The ViewModel checks if the month's items are in Room. If missing, it dispatches
      `GET /api/timeline/by-month?months=YYYY-MM-01` for the visible month plus 1–2 overscan months (exact equivalent of
      `preLoadAllMonths` in `timelineStore.ts`).
    * Once metadata arrives, `Placeholder` entries in that month transition to `Loaded` items, binding thumbnail URLs
      without shifting row geometry.

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

##### Throttling During Fast Scrubbing (`isScrollingFast`)

When dragging the scrubber across years of photos, composing thousands of incoming images would flood Coil's request
queues and cause GC stutters.

* Adopt the web app's `isScrollingFast` pattern:
    * While the user is actively dragging the scrubber thumb, set `isScrollingFast = true`.
    * While `isScrollingFast == true`, row items render only the background shape box, completely bypassing image
      loading.
    * Once the scrubber stops or touch lifts (150ms debounce), set `isScrollingFast = false` to begin loading thumbnails
      for the landed viewport.

---

### Part 2: Android App Architecture

Adopt **Clean Architecture + MVI/MVVM** with **Unidirectional Data Flow (UDF)**.

```
┌────────────────────────────────────────────────────────────┐
│                        UI Layer                            │
│   Compose Screens (Timeline, Viewer, Albums, Settings)     │
│   ViewModels (StateFlow<UiState>, UiEvent channels)        │
└─────────────────────────────┬──────────────────────────────┘
                              │
┌─────────────────────────────▼──────────────────────────────┐
│                      Domain Layer                          │
│   UseCases (CalculateLayout, SyncTimelineUseCase,          │
│   DeduplicateMediaUseCase, EnqueueUploadUseCase)           │
│   Domain Models (MediaItem, SyncStatus, TimelineRow)       │
└─────────────────────────────┬──────────────────────────────┘
                              │
┌─────────────────────────────▼──────────────────────────────┐
│                       Data Layer                           │
│  Repositories (MediaRepository, BackupRepository)          │
│  Local: Room DB, MediaStore DataSource, Layout Cache       │
│  Remote: OkHttp + Retrofit (Protobuf Lite), TUS Client     │
│  Background: WorkManager (UploadWorker, LocalScanWorker)   │
└────────────────────────────────────────────────────────────┘
```

#### Key Architecture Principles for this App:

1. **Two-Tier Layout vs Item Storage:**
    * **Structure Cache:** Stores `month_id -> List<Float>` ratios to construct the visual grid partition immediately.
    * **Room Database:** Stores complete `MediaEntity` rows (local items and synced cloud items). The UI combines both
      tiers into `TimelineRowUiItem` models.
2. **Local vs Cloud Identity (Deduplication via SHA-256):**
    * Backend adds a `sha256` column to `media_item`, indexed for fast lookups.
    * Media items in Room contain:
        * `localContentUri`: `content://media/external/images/media/1234` (nullable)
        * `cloudId`: Backend 10-char alphanumeric string `a1B2c3D4e5` (nullable)
        * `sha256`: SHA-256 file hash
        * `syncStatus`: `LOCAL_ONLY`, `SYNCING`, `SYNCED`, `CLOUD_ONLY`
    * **Two-Step Deduplication Pipeline:**
        * *Fast Pass:* When scanning local storage, match against cloud items using `(date_taken, size_bytes)` as an
          initial instant filter.
        * *Crypto Verification:* For candidate matches or when staging an upload, compute SHA-256 via
          hardware-accelerated `MessageDigest.getInstance("SHA-256")`. If it matches a cloud item, mark it `SYNCED` and
          link `cloudId` without uploading.
3. **Coil 3 Thumbnail Strategy:**
    * **Target SDK:** Android 12+ (API 31+) provides native hardware-accelerated AVIF decoding via `ImageDecoder`.
    * If `syncStatus == LOCAL_ONLY`: Image source is `localContentUri` (instant system thumbnail).
    * If `syncStatus == CLOUD_ONLY`:
        * If `hasThumbnails == true`: Image source is `"${BASE_URL}/thumbnails/${cloudId}/${targetHeight}p.avif"` cached
          in Coil's disk LRU cache.
        * If `hasThumbnails == false` (ingest in progress): Image source is
          `"${BASE_URL}/api/photos/${cloudId}/thumbnail?size=${targetHeight}"`.
    * If `syncStatus == SYNCED`: Prefer `localContentUri` while on device storage; fall back to cloud thumbnail URL if
      deleted locally.
4. **Resumable Uploads via Foreground WorkManager:**
    * `UploadWorker` executes TUS resumable uploads in 5 MB chunks.
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
│   ├── network/            # OkHttp, Retrofit, AuthInterceptor, TokenAuthenticator
│   │   ├── api/            # PhotosApiService (Retrofit Protobuf Lite)
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
│       ├── local/          # MediaStoreContentObserver, LocalMediaScanner, LayoutCache
│       └── remote/         # RemoteMediaDataSource (Protobuf parsing)
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
    │   ├── components/     # JustifiedRow.kt, TimelineScrubber.kt, SelectionActionBar.kt
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
| **`AuthScreen`**                         | • Server URL input (defaults to self-hosted domain)<br>• Email & password inputs                                                                                                           | • Validates connection via `GET /api/health`<br>• Stores access/refresh tokens in encrypted DataStore                                                                                  |

---

### Part 5: Recommended Third-Party Libraries

* **Target SDK:** Min SDK **31** (Android 12), Target SDK **35+** (Android 15).
* **Image Loading:** `io.coil-kt.coil3:coil-compose:3.1.0` (Native Compose integration, supports coroutines, handles
  `content://`, `http://`, and AVIF decoding via `ImageDecoder`).
* **Zoom Gestures in Viewer:** `me.saket.telephoto:zoomable-image-coil:0.14.0` (Compose zoomable image with fling,
  inertia, and nested scrolling cooperation with `HorizontalPager`).
* **Video Playback:** `androidx.media3:media3-exoplayer:1.5.1` & `androidx.media3:media3-ui:1.5.1` (Shared player pool
  across pager items).
* **Protobuf Compilation:** `com.google.protobuf:protobuf-kotlin-lite:4.30.0` with the Gradle Protobuf plugin to compile
  `timeline.proto`.
* **Navigation:** `androidx.navigation:navigation-compose:2.8.8` (Type-safe Compose navigation routes via
  `@Serializable` classes).
* **Local DB:** `androidx.room:room-ktx:2.7.0` (Configured with SQLite WAL mode for non-blocking concurrent
  reads/writes).
* **Background Tasks:** `androidx.work:work-runtime-ktx:2.10.0` (WorkManager with Foreground Service support).