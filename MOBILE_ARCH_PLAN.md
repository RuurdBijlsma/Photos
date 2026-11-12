### Part 1: The Justified Horizontal Row Grid

The Google Photos layout is technically a **Justified Horizontal Row Layout** (linear partition), rather than a
Pinterest-style vertical masonry grid. Every row is scaled so that items fit flush from screen edge to edge with uniform
gaps.

#### 1. Why `LazyColumn` Beats `LazyVerticalGrid` for Justified Layouts

`LazyVerticalGrid` is designed for fixed or adaptive column counts with rigid aspect-ratio spans. It forces irregular
span math and causes measurement recalculations during scrolling.

Instead, the cleanest and most performant pattern in Jetpack Compose is **a `LazyColumn` of precomputed rows**:

- Each item in the `LazyColumn` represents either a **Month Header** or a **Row of 2–5 photos**.
- In Compose, a row item is simply a standard `Row` containing photo composables whose widths are proportioned by their
  aspect ratio using `Modifier.weight(aspectRatio)` or explicit DP width, constrained to the precalculated row height.
- **Why this is fast**: Compose treats each row as a single virtualized element. Measurement overhead is virtually zero
  during scroll because row heights and aspect ratios are precomputed on background threads.

```
LazyColumn
 ├── Item 0: MonthHeaderUiItem("October 2026")
 ├── Item 1: TimelineRowUiItem(height = 180.dp, photos = [PhotoA, PhotoB, PhotoC])
 ├── Item 2: TimelineRowUiItem(height = 195.dp, photos = [PhotoD, PhotoE])
 └── ...
```

---

#### 2. Layout Calculation Engine (`TimelineLayoutCalculator`)

Just like your web app's `calculateLayout` in `TimelineContainer.vue`, the math runs strictly off the UI thread (
`Dispatchers.Default`) when the window width or density changes.

```kotlin
data class TimelineRowUiItem(
    val id: String, // e.g. "row_2026_10_0"
    val heightDp: Float,
    val items: List<TimelineMediaUiModel>,
    val monthId: String,
    val isFirstOfMonth: Boolean,
    val isLastOfMonth: Boolean,
    val estimatedOffsetTopDp: Float
)
```

**Row Partitioning Algorithm:**

1. Target desired row height: e.g., `180.dp` on phone portrait, `240.dp` on tablet/landscape.
2. Accumulate items into the current row:
   $$\text{targetWidth} = \sum (\text{aspectRatio}_i \times \text{desiredHeight}) + (N - 1) \times \text{gap}$$
3. When $\text{targetWidth} \ge \text{containerWidth}$:
   $$\text{scaleFactor} = \frac{\text{containerWidth} - \text{totalGaps}}{\sum (\text{aspectRatio}_i \times \text{desiredHeight})}$$
   $$\text{actualRowHeight} = \text{clamp}(\text{desiredHeight} \times \text{scaleFactor}, \; \text{minHeight}, \; \text{desiredHeight} \times 1.5)$$
4. Finalize the row, compute its cumulative vertical offset (`offsetTop`), and advance.
5. If the month boundary ends on an incomplete row, cap its scale factor to `1.0` so the last 1 or 2 items don't stretch
   into giant blocks.

---

#### 3. Unifying Local MediaStore + Cloud Protobuf Ratios

To layout the grid instantly without loading bitmaps, **aspect ratios must be known beforehand**:

1. **Cloud Items:** Your backend's `GET /api/timeline/ratios` provides `TimelineRatiosResponse` with float aspect ratios
   per month. This payload is tiny and downloads in milliseconds. We may have to add another backend endpoint for a
   protobuf response with ratios and some more info, like media item ids if needed. That or we use the monthRatios
   endpoint for on demand media item info, same as web.
2. **Local Device Items:** `MediaStore.Images.Media` provides `WIDTH`, `HEIGHT`, and `ORIENTATION`.

* *Critical EXIF quirk:* If `orientation == 90 || orientation == 270`, swap `width` and `height` before
  calculating $\text{ratio} = \frac{\text{width}}{\text{height}}$.

3. **Room Database Cache:** Cache all items (both local and synced) with their pre-calculated aspect ratios.
4. **App Launch flow:**

- **T = 0ms:** Load cached rows from Room or memory cache. The grid renders immediately.
- **T + 100ms:** MediaStore local scan runs via ContentObserver Flow. If new photos were taken, insert them into Room.
- **T + 300ms:** Remote sync fetches `GET /api/timeline/ratios` (Protobuf), detects diffs, and updates Room.
- Any diff triggers a re-partitioning pass on `Dispatchers.Default` and emits a new list to the `LazyColumn`.

---

#### 4. Instant Fast-Scrubbing with Variable Row Heights

Because each row has a slightly different height, native scroll thumbs can jitter. Solve this by maintaining a
lightweight **Layout Index**:

```kotlin
class TimelineLayoutIndex(
    val rows: List<TimelineRowUiItem>,
    val totalHeightDp: Float,
    val monthOffsets: Map<String, Int> // monthId -> LazyColumn index
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

* **Dragging the scrubber:** Translate scrubber touch Y percentage ($0.0 \to 1.0$) into `targetOffsetDp` $\to$ binary
  search the row index $\to$ call `lazyListState.scrollToItem(index)`.
* **Date Bubble Overlay:** Read `rows[index].monthId` or the first photo's `timestamp` to display `"October 2026"` in
  real time as the thumb moves.

---

### Part 2: Android App Architecture

Adopt the standard **Clean Architecture + MVI/MVVM** with **Unidirectional Data Flow (UDF)**.

```
┌────────────────────────────────────────────────────────┐
│                        UI Layer                        │
│   Compose Screens (Timeline, Viewer, Albums, Settings) │
│   ViewModels (StateFlow<UiState>, UiEvent channels)    │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│                      Domain Layer                      │
│   UseCases (SyncTimelineUseCase, CalculateLayout,      │
│   MergeLocalAndCloudMedia, EnqueueUploadUseCase)       │
│   Domain Models (MediaItem, SyncStatus, Album)         │
└───────────────────────────┬────────────────────────────┘
                            │
┌───────────────────────────▼────────────────────────────┐
│                       Data Layer                       │
│  Repositories (MediaRepository, BackupRepository)      │
│  Local: Room DB, MediaStore DataSource, Preferences    │
│  Remote: OkHttp + Retrofit (Protobuf Lite), TUS Client │
│  Background: WorkManager (UploadWorker, SyncWorker)    │
└────────────────────────────────────────────────────────┘
```

#### Key Architecture Principles for this App:

1. **Single Source of Truth (SSOT):** The UI reads the timeline **only from Room** via a Reactive Flow. Network and
   MediaStore scans write *into* Room; they never push directly to the UI.
2. **Local vs Cloud Identity (Deduplication):**

- Media items in Room have:
    - `localContentUri`: `content://media/external/images/media/1234` (nullable)
    - `cloudId`: Backend 10-char alphanumeric string `a1B2c3D4e5` (nullable)
    - `sha256Hash`: Fast hash used to link a local photo with its uploaded backend equivalent
    - `syncStatus`: `LOCAL_ONLY`, `SYNCING`, `SYNCED`, `CLOUD_ONLY`

3. **Coil 3 Thumbnail Strategy:**

- If `syncStatus == LOCAL_ONLY`: Image source is `localContentUri` (instant system thumbnail).
- If `syncStatus == CLOUD_ONLY`: Image source is `"${BASE_URL}/thumbnails/${cloudId}/${targetHeight}p.avif"` cached in
  Coil's disk LRU cache.
- If `syncStatus == SYNCED`: Prefer `localContentUri` if still on device storage (saving bandwidth/cache space),
  fallback to cloud thumbnail URL if deleted from device.

---

### Part 3: Recommended Package Structure

A clean, feature-driven package structure (`package-by-feature` within layers):

```text
app/src/main/java/com/ruurd/photos/
├── core/
│   ├── common/             # Dispatchers, Result wrapper, Extensions
│   ├── database/           # Room Database, TypeConverters
│   │   ├── dao/            # MediaDao, AlbumDao, UploadQueueDao
│   │   └── entity/         # MediaEntity, AlbumEntity, UploadQueueEntity
│   ├── network/            # OkHttp, Retrofit, CookieJar (Session Auth), Protobuf converters
│   │   ├── api/            # PhotosApiService (Retrofit interfaces)
│   │   └── tus/            # Resumable TUS upload client
│   └── ui/                 # Shared UI components, Theme (M3 Dynamic Color), Dimens
│       └── components/     # ThumbnailImage (Coil wrapper), FastScroller, DateOverlay
│
├── data/                   # Repositories & Data Sources
│   ├── repository/
│   │   ├── MediaRepositoryImpl.kt
│   │   ├── AlbumRepositoryImpl.kt
│   │   ├── BackupRepositoryImpl.kt
│   │   └── AuthRepositoryImpl.kt
│   └── datasource/
│       ├── local/          # MediaStoreContentObserver, LocalMediaScanner
│       └── remote/         # RemoteMediaDataSource (Protobuf parsing)
│
├── domain/                 # Pure Kotlin business logic & UseCases
│   ├── model/              # MediaItem, SyncStatus, TimelineRow, Album
│   ├── repository/         # Repository Interfaces
│   └── usecase/
│       ├── CalculateTimelineLayoutUseCase.kt
│       ├── SyncTimelineUseCase.kt
│       ├── ScanLocalMediaUseCase.kt
│       └── EnqueueBackupUseCase.kt
│
├── work/                   # WorkManager background workers
│   ├── UploadWorker.kt     # TUS auto-upload execution
│   └── LocalScanWorker.kt  # Periodic/Event-driven MediaStore reconciliation
│
└── features/               # Presentation layer (Compose UI + ViewModels)
    ├── auth/               # Login screen, Server URL configuration
    │   ├── AuthScreen.kt
    │   └── AuthViewModel.kt
    ├── timeline/           # The core gallery view
    │   ├── TimelineScreen.kt
    │   ├── TimelineViewModel.kt
    │   ├── TimelineUiState.kt
    │   └── components/
    │       ├── JustifiedRow.kt
    │       ├── TimelineScrubber.kt
    │       └── SelectionActionBar.kt
    ├── viewer/             # Media viewer (Pinch-to-zoom, Swipe dismiss)
    │   ├── MediaViewerScreen.kt
    │   ├── MediaViewerViewModel.kt
    │   └── components/
    │       ├── ZoomableImage.kt (Telephoto/Zoomable)
    │       ├── VideoPlayer.kt   (Media3 / ExoPlayer)
    │       └── MediaInfoBottomSheet.kt
    ├── albums/             # Albums listing & album detail views
    │   ├── AlbumsScreen.kt
    │   └── AlbumDetailScreen.kt
    └── backup/             # Backup settings & status
        ├── BackupSettingsScreen.kt
        └── BackupSettingsViewModel.kt
```

---

### Part 4: Screens & Views Breakdown

| Screen                                   | Core UI Components                                                                                                                                                                 | Key Gestures & Interactions                                                                                                                          |
|:-----------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------|
| **`TimelineScreen`**                     | • `LazyColumn` of justified rows<br>• Right-edge `TimelineScrubber`<br>• Floating `DateOverlay`<br>• Contextual multi-selection top bar                                            | • Fast vertical dragging on scrubber to jump dates<br>• Long-press + drag down to multi-select items<br>• Tap item $\to$ navigate into `MediaViewer` |
| **`MediaViewerScreen`**                  | • `HorizontalPager` for smooth swiping<br>• `Telephoto` zoomable photo viewport<br>• Media3 `AndroidView(ExoPlayer)` for videos<br>• Bottom action bar (Share, Edit, Delete, Info) | • Double-tap to zoom, pinch-to-zoom<br>• Swipe down to dismiss back to timeline<br>• Swipe up to pull up metadata / EXIF / weather sheet             |
| **`AlbumsScreen` & `AlbumDetailScreen`** | • `LazyVerticalGrid` (square cards) for album list<br>• Justified row grid for album contents                                                                                      | • Tap album card $\to$ open album timeline<br>• Add photos to album via selection flow                                                               |
| **`BackupSettingsScreen`**               | • Device folder checklist (DCIM, WhatsApp, Downloads)<br>• Constraints switches (Wi-Fi only, while charging)<br>• Live upload queue progress bar                                   | • Toggle auto-backup<br>• "Sync Now" manual trigger                                                                                                  |
| **`AuthScreen`**                         | • Server URL input (defaults to self-hosted domain)<br>• Email & password inputs                                                                                                   | • Validates connection via `GET /api/health`<br>• Stores session cookie in persistent `SharedPreferencesCookieJar`                                   |

---

### Part 5: Recommended Third-Party Libraries

* **Image Loading:** `io.coil-kt.coil3:coil-compose:3.1.0` (Native Compose integration, supports coroutines, handles
  `content://`, `http://`, and AVIF decoding via `coil-network-okhttp`).
* **Zoom Gestures in Viewer:** `me.saket.telephoto:zoomable-image-coil:0.14.0` (Industry-standard Compose zoomable image
  with smooth fling, inertia, and nested scrolling cooperation with `HorizontalPager`).
* **Video Playback:** `androidx.media3:media3-exoplayer:1.5.1` & `androidx.media3:media3-ui:1.5.1`.
* **Protobuf Compilation:** `com.google.protobuf:protobuf-kotlin-lite:4.30.0` with the Gradle Protobuf plugin to compile
  your `timeline.proto`.
* **Navigation:** `androidx.navigation:navigation-compose:2.8.8` (Type-safe Compose navigation routes via
  `@Serializable` classes).
* **Local DB:** `androidx.room:room-ktx:2.7.0` (Configured with SQLite WAL mode for non-blocking concurrent
  reads/writes).
* **Background Tasks:** `androidx.work:work-runtime-ktx:2.10.0` (WorkManager).
