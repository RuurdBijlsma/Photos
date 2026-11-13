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
Truth, the layout calculator receives fully resolved entities directly—with `yearMonth` precomputed on write so month
boundary detection is a simple integer comparison with zero date parsing during layout.

```kotlin
data class TimelineRowUiItem(
    val id: String, // Stable key, e.g. "row_${yearMonth}_${items.firstOrNull()?.id ?: index}"
    val heightDp: Float,
    val items: List<TimelineGridItemUiModel>,
    val yearMonth: Int, // Precomputed e.g. 202610
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
    val sortTimestamp: Long,          // UTC epoch millis for ordering within month
    val timezoneOffsetMinutes: Int,   // Local capture offset from UTC
    val yearMonth: Int,               // Precomputed local month (e.g. 202610)
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
5. If the `yearMonth` boundary ends on an incomplete row, cap its scale factor to `1.0` so the last 1 or 2 items don't
   stretch into giant blocks.

---

#### 3. Unifying Local MediaStore + Cloud Protobuf: Dedicated Compact Sync & Room SSOT

Instead of the web app's anonymous ratio arrays and multi-stage month hydration, the mobile client uses a **Dedicated
Compact Protobuf Sync Endpoint** (to be implemented) paired with **Room as the Single Source of Truth (SSOT)**. This
guarantees full offline capability and instant local-to-cloud reconciliation without UI placeholders.

**Tombstone Deletion Tracking:** Permanent deletions are recorded in a lightweight `deleted_media_item` (tombstone,
populated by db trigger, to be implemented) table with the purged IDs and their deletion timestamp. The delta sync
endpoint checks `deleted_at > since` alongside `updated_at > since` to return `deleted_ids`, allowing the mobile client
to purge removed items from Room without re-fetching the entire library.

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                 Dedicated Compact Sync (Initial & Delta)                     │
│  • Remote: GET /api/mobile/timeline/sync?since=... (Compact Protobuf)        │
│    Payload: [id, sort_timestamp, tz_offset_min, ratio, is_video,             │
│              has_thumbnails, size_bytes]                                     │
│    Size: ~26–27 bytes/photo (~400 KB gzipped for 50,000 photos)              │
│  • Computes `year_month` and writes directly to Room `media_item` table      │
└──────────────────────────────────────┬───────────────────────────────────────┘
                                       │
                                       ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                 Room Database: Single Source of Truth (SSOT)                 │
│  • Houses local-only, cloud-only, and merged/synced items                    │
│  • Deduplication via zero-hash fast pass: size_bytes + ratio + tz-aligned ts │
│  • Feeds CalculateTimelineLayoutUseCase directly on Dispatchers.Default      │
│  • 100% offline-ready: entire timeline renders instantly from SQLite        │
└──────────────────────────────────────────────────────────────────────────────┘
```

If an IngestMetadata transaction starts at T = 100 (setting updated_at = now() = 100 in Postgres) and commits at T =
105, while a mobile sync runs at T = 102 and saves sync_timestamp = 102, the next delta sync (updated_at > 102) will
permanently miss that item.

Fix: On the client (or server), subtract a small safety overlap from since (e.g., since - 60_000L, 60 seconds). Since
upserted and deleted_ids are idempotent in Room, overlapping by a minute costs almost nothing and eliminates missed
concurrent ingests.

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
     string id = 1;                      // 10-char alphanumeric ID
     int64 sort_timestamp = 2;           // Epoch millis (UTC) for deterministic chronological sorting
     float ratio = 3;                    // width / height aspect ratio
     bool is_video = 4;
     bool has_thumbnails = 5;            // True if server pre-generated AVIF thumbnails are ready
     int64 size_bytes = 6;               // Used for instant, zero-hash MediaStore deduplication
     optional int32 duration_ms = 7;     // Video duration in milliseconds (if video)
     sint32 timezone_offset_minutes = 8; // Offset of taken_at_local vs sort_timestamp in minutes
   }
   ```
2. **Local Timezone Month Grouping (`year_month`):**
    * The web backend groups months by `month_id = date_trunc('month', taken_at_local)` (the wall-clock time where the
      photo was taken, not the viewer's current timezone) and sorts items inside each month by
      `sort_timestamp DESC, id DESC`.
    * On the server, `timezone_offset_minutes` is derived from the two `NOT NULL` columns in `media_item` so it always
      has a value even when `timezone_offset_seconds` is `NULL`:
      ```sql
      ROUND(
          EXTRACT(EPOCH FROM (taken_at_local - (sort_timestamp AT TIME ZONE 'UTC'))) / 60.0
      )::INT AS timezone_offset_minutes
      ```
      *(Sending `sint32 timezone_offset_minutes` adds only 2–3 bytes uncompressed and compresses to near zero under gzip
      since most photos in a library share 1–2 offsets; it also lets the viewer info sheet display true local capture
      time offline.)*
    * When writing to Room, precompute `year_month` (e.g., `202610` as an `Int`) and store it on `MediaEntity`:
        * **Cloud items (`CLOUD_ONLY` / `SYNCED`):** Derive `year_month` from
          `Instant.ofEpochMilli(sort_timestamp + timezone_offset_minutes * 60_000L)` in UTC.
        * **Local-only items (`LOCAL_ONLY`):** Derive `year_month` from `MediaStore.DATE_TAKEN` using
          `ZoneId.systemDefault()`. Once backed up and synced, the server's authoritative `year_month` and
          `timezone_offset_minutes` take over.
    * **Timeline Room query ordering & local folder filtering:**
      ```sql
      SELECT m.* FROM media_item m
      LEFT JOIN device_folder f ON m.bucket_id = f.bucket_id
      WHERE m.sync_status IN ('CLOUD_ONLY', 'SYNCED')
         OR (m.sync_status = 'LOCAL_ONLY' AND f.show_in_timeline = 1)
      ORDER BY m.year_month DESC, m.sort_timestamp DESC, m.id DESC
      ```
      Sorting by `year_month DESC` before `sort_timestamp DESC` guarantees contiguous month groups matching the
      backend—so cross-timezone travel around midnight on the 1st of a month never splits a month header into two
      sections.
3. **Local Folder Scope & Google Photos-Style "On This Device" Filtering:**
    * **Scan everything, filter at query time:** `LocalMediaScanner` queries **all** non-pending `MediaStore` images and
      videos (`IS_PENDING = 0 AND SIZE > 0`) and records `bucketId`, `bucketName`, and `relativePath` on `MediaEntity`.
      A lightweight `device_folder` Room table (`bucket_id`, `display_name`, `relative_path`, `show_in_timeline`,
      `backup_enabled`) stores folder preferences.
    * **Default folder rules (on first discovery of a `bucketId`):**
        * **Camera folders (`showInTimeline = true`, `backupEnabled = true`):** `relativePath.startsWith("DCIM/")`
          **excluding** screenshot/screen-recording folders (e.g.,
          `!relativePath.startsWith("DCIM/Screenshots", ignoreCase = true)`
          and `!relativePath.startsWith("DCIM/Screen", ignoreCase = true)`). This automatically includes `DCIM/Camera/`
          and 3rd-party cameras under `DCIM/` (e.g. `DCIM/OpenCamera/`) while avoiding Google Photos' infamous
          Samsung/Xiaomi bug where `DCIM/Screenshots/` pollutes the camera roll.
        * **Other device folders (`showInTimeline = false`, `backupEnabled = false`):** Everything else (
          `Pictures/Screenshots/`,
          `DCIM/Screenshots/`, `Download/`, `Pictures/WhatsApp/`, or 3rd-party cameras saving outside `DCIM/`). These
          stay out of the main timeline and are never uploaded unless the user opts in.
    * **Web vs. Mobile parity:** Only backed-up/uploaded items ever reach the server, so non-backed-up screenshots and
      downloads never clutter the web timeline.
    * **Device Folders ("On this device") & Deduplication:** All discovered folders remain browsable in a Device Folders
      view / Backup Settings (`GROUP BY bucket_id`), where users can toggle **Back up** (which automatically enables
      `showInTimeline`) or **Show in timeline** per folder. Crucially, **deduplication still runs across all scanned
      folders**, so if a user manually uploads a single screenshot via Share or Device Folders, it links cleanly as
      `SYNCED` instead of duplicating.
4. **Delta Syncing (`since` parameter):**
    * **Initial Launch:** Query `GET /api/mobile/timeline/sync` without `since`. Backend streams all non-deleted items.
      Room bulk-inserts them in a single transaction and stores `latestSyncTimestamp`.
    * **Subsequent Launches:** Query `GET /api/mobile/timeline/sync?since={latestSyncTimestamp}`. Only items created,
      updated, or soft-deleted since the last sync are returned (typically a few KB).
    * To handle possible out-of-sync situations, do a full sync (without `?since`) periodically (e.g., weekly) and on
      pull-to-refresh.
5. **Room Invalidation Note (Deferred):**
    * Always wrap sync and `MediaStore` batch updates in a single Room `@Transaction` (`withTransaction { ... }`) so
      Room emits one `Flow` invalidation per batch rather than per row.
    * If rapid concurrent updates ever cause "Room invalidation storms" (multiple layout recalculations per second), add
      a small `.debounce(150)` on the timeline `Flow` (or split queries by month). Don't implement extra complexity
      unless actually encountered.

##### Two-Pass Local/Cloud Deduplication Strategy

To prevent cold-start battery drain and disk thrashing, **the mobile app never hashes local photos on startup**:

1. **Pass 1: Instant Fast-Pass (Zero Hashing, Timezone-Safe):**
    * Android `MediaStore.DATE_TAKEN` often differs from the server's `sort_timestamp` by exact timezone offsets (e.g.,
      naive EXIF `DateTimeOriginal` interpreted in the phone's current timezone vs. server GPS/fallback timezone, OEM
      QuickTime `creation_time` written in local time, or DST shifts).
    * Query `MediaStore` (filtering `IS_PENDING = 0` and `SIZE > 0`) for `_ID`, `WIDTH`, `HEIGHT`, `ORIENTATION`,
      `SIZE`,
      `DATE_TAKEN`, `DATE_MODIFIED`, `BUCKET_ID`, `BUCKET_DISPLAY_NAME`, and `RELATIVE_PATH` (compute `localRatio` from
      oriented dimensions, guarding against `0`).
    * Index `size_bytes` in Room (`@Index(value = ["sizeBytes"])`) and query `CLOUD_ONLY` candidates by exact byte size:
      ```sql
      SELECT * FROM media_item
      WHERE size_bytes = :localFileSize
        AND sync_status = 'CLOUD_ONLY'
      ```
    * Match in Kotlin if **aspect ratio** matches (`abs(cloud.ratio - localRatio) < 0.02f`) AND **timestamp** satisfies
      either:
        * **Exact UTC match:** `diff < 2_000L` (covers sub-second truncation/rounding), OR
        * **Timezone-aligned match:** `diff <= 24 * 3_600_000L` and `diff` falls within `±2_000L` of a 15-minute
          (`900_000L`) boundary (covers all full-hour offsets plus `:30` / `:45` timezones):
          ```kotlin
          val diff = abs(cloud.sortTimestamp - localDateTakenMs)
          val stepMs = 900_000L
          val remainder = diff % stepMs
          val timestampMatches = diff < 2_000L || (
              diff <= 24 * 3_600_000L &&
                  (remainder < 2_000L || stepMs - remainder < 2_000L)
          )
          ```
    * Filtering on `sync_status = 'CLOUD_ONLY'` prevents two identical local copies from claiming the same `cloudId`. A
      match immediately links `localContentUri` to `cloudId` and marks `syncStatus = SYNCED` with zero disk I/O.
2. **Pass 2: On-Demand Crypto Verification (Deferred to Backup/Upload):**
    * For local items that did *not* match in Pass 1 (including rare cases where `size_bytes` + `ratio` matched a
      `CLOUD_ONLY` item whose timestamp differed by >24h due to a web date edit or missing `DATE_TAKEN`), the file hash
      is calculated **only when staged for backup** (or lazily for those rare size-collision candidates) using the
      sha256 stored by the backend (backend currently uses BLAKE3, sha256 hash to be implemented and added to
      `media_item`
      table).
    * Before streaming bytes, a pre-upload check (`POST /api/upload/check-hashes`) queries the backend with the hash. If
      the backend already possesses it, the upload is skipped and the item is marked `SYNCED`.

##### TIPS

`DATE_TAKEN` can be `NULL` or `0`: Screenshots, WhatsApp media, and downloaded images often lack `DATE_TAKEN` in
`MediaStore`. Fall back to `DATE_MODIFIED * 1000L` (note: `DATE_ADDED` and `DATE_MODIFIED` are in **seconds**, whereas
`DATE_TAKEN` is in **milliseconds**!).

##### How `MediaEntity` handles primary keys and state transitions between `LOCAL_ONLY`, `CLOUD_ONLY`, and `SYNCED`

If `MediaEntity` uses `cloudId` for cloud items and a generated/local ID for `LOCAL_ONLY` items, or vice versa, consider
all four transitions carefully inside `@Transaction`:

1. **Local item gets backed up → Cloud sync arrives:**
   `LOCAL_ONLY` (`local_1234`) is uploaded via TUS. `IngestMetadata` runs asynchronously on the worker and creates a new
   `cloudId` (`a1B2c3D4e5`). When `SyncTimelineUseCase` receives `a1B2c3D4e5`, it **must also run deduplication
   against `LOCAL_ONLY` / `SYNCING` rows** (not just `ScanLocalMediaUseCase` running against `CLOUD_ONLY` rows!),
   otherwise you'll momentarily show duplicate photos in the grid.
2. **Cloud item deleted (`deleted_ids` contains `a1B2c3D4e5`) while `SYNCED`:**
   Do **not** blindly `DELETE FROM media_item WHERE cloud_id IN (:deletedIds)`! If `localContentUri != null` (the photo
   still exists on the phone), flip `syncStatus = LOCAL_ONLY` and `cloudId = null` (otherwise it vanishes from the
   gallery and reappears on the next `MediaStore` scan). Only delete the Room row if `localContentUri == null`.
3. **Local file deleted from device while `SYNCED`:**
   When `ScanLocalMediaUseCase` sees `localContentUri` is gone, flip `syncStatus = CLOUD_ONLY` and
   `localContentUri = null`. Only delete the Room row if `cloudId == null`.
4. **Recommendation:** Make `localId` (or a deterministic surrogate key, e.g., `UUID` or `cloudId` with a helper merge)
   clean by indexing `cloudId` (`UNIQUE`) and `localContentUri` (`UNIQUE`) separately, or explicitly performing an
   atomic delete-old + insert/update in one Room `@Transaction` when merging `LOCAL_ONLY` + `CLOUD_ONLY` into `SYNCED`.

##### Auth & Network Tips

* **Concurrent 401 refresh lock**
  `server/config/settings.yaml` sets `access_token_expiry_minutes: 30` and `20251004204735_init.sql` stores hashed
  `refresh_token` selectors/verifiers (token rotation). If 4 parallel requests (e.g., timeline sync + 3 on-demand Coil
  thumbnail fetches) hit `401` when the 30-minute cookie expires, an unsynchronized OkHttp `Authenticator` will send 4
  concurrent `/api/auth/refresh` calls. The first rotates the token and the other 3 fail and log the user out.
    * **Fix:** Guard `/api/auth/refresh` in your OkHttp `Authenticator` with a `Mutex` / `synchronized` block and
      double-check whether the `access_token` cookie already changed before firing a refresh request. Also cap retries
      (`responseCount(response) >= 2 -> give up`) to avoid infinite 401 loops.
    * Keep `api.rate_limiting` (`req_per_second: 6, burst_size: 10`) in mind for `auth_public_router`—don't spam
      `/api/auth/login` or `/api/auth/refresh`.
* **Dynamic Server URL:**
  Since `AuthScreen` lets the user input a server URL, don't hardcode Retrofit's `baseUrl` at DI graph creation time.
  Either swap/recreate the Retrofit instance on URL change or rewrite `request.url` in an OkHttp interceptor from the
  persisted server URL. Also add a `network_security_config.xml` (or debug manifest override) if you plan to test
  against `http://10.0.2.2:5272` or LAN HTTP.

##### App Launch Flow:

* **T = 0ms:** Read existing records from Room (`ORDER BY year_month DESC, sort_timestamp DESC, id DESC`, filtered by
  `device_folder.show_in_timeline = 1` for `LOCAL_ONLY` items). Compute justified layout on `Dispatchers.Default`. The
  timeline renders immediately with full metadata, even in airplane mode.
* **T + 50ms:** `MediaStore` scan reconciles new or deleted local photos across all folders via the zero-hash fast pass
  and registers any newly seen `bucketId`s in `device_folder` with default visibility/backup flags.
* **T + 150ms:** Background delta sync calls `GET /api/mobile/timeline/sync?since=...`. Any cloud changes are committed
  to Room in a single transaction, triggering an incremental UI layout update.

---

#### 4. Instant Fast-Scrubbing with Variable Row Heights

Because each row has a slightly different height, native scroll thumbs can jitter. Solve this with a **Layout Index**:

```kotlin
class TimelineLayoutIndex(
    val rows: List<TimelineRowUiItem>,
    val totalHeightDp: Float,
    val monthOffsets: Map<Int, Int> // yearMonth (e.g. 202610) -> LazyColumn row index
) {
    fun findRowIndexAtPercentage(fraction: Float): Int {
        val targetOffset = fraction * totalHeightDp
        // Binary search rows by estimatedOffsetTopDp: O(log N)
        val idx = rows.binarySearchBy(targetOffset) { it.estimatedOffsetTopDp }
        return if (idx < 0) (-idx - 1).coerceIn(0, rows.lastIndex) else idx
    }

    fun findIndexForDate(yearMonth: Int): Int = monthOffsets[yearMonth] ?: 0
}
```

### Part 2: Android App Architecture

Adopt **Clean Architecture + MVI/MVVM** with **Unidirectional Data Flow (UDF)**.

```
┌─────────────────────────────────────────────────────────────┐
│                        UI Layer                             │
│   Compose Screens (Timeline, Viewer, Albums, Settings)      │
│   ViewModels (StateFlow<UiState>, UiEvent channels)         │
└──────────────────────────────┬──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│                      Domain Layer                           │
│   UseCases (CalculateLayout, SyncTimelineUseCase,           │
│   DeduplicateMediaUseCase, EnqueueUploadUseCase)            │
│   Domain Models (MediaItem, SyncStatus, TimelineRow)        │
└──────────────────────────────┬──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│                       Data Layer                            │
│  Repositories (MediaRepository, BackupRepository)           │
│  Local: Room DB, MediaStore DataSource, Layout Index        │
│  Remote: OkHttp + Persistent CookieJar, Compact Sync API    │
│  Background: WorkManager (UploadWorker, LocalScanWorker)    │
└─────────────────────────────────────────────────────────────┘
```

#### Key Architecture Principles for this App:

1. **Cookie-Based Session Authentication & Persistent CookieJar:**
    * Matches the web client (`web/src/scripts/services/api.ts`) by relying exclusively on HTTP cookies (`access_token`,
      `refresh_token`) set by the backend on `POST /api/auth/login`.
    * OkHttp is configured with a persistent `CookieJar` (backed by encrypted SharedPreferences or DataStore).
    * Every network call—including Retrofit API requests, Coil thumbnail image fetches, and TUS file uploads—shares this
      exact OkHttp client instance and automatically includes the session cookies.
    * Transparent session refresh: An OkHttp `Authenticator` triggers `POST /api/auth/refresh` (synchronized via Mutex)
      when an access cookie expires, updating the cookie store seamlessly without re-prompting the user.
2. **Room Database as Single Source of Truth (SSOT):**
    * Room stores complete `MediaEntity` rows (local-only items, synced items, and cloud-only items), including
      precomputed `yearMonth`, `timezoneOffsetMinutes`, and local folder metadata (`bucketId`, `bucketName`,
      `relativePath`), alongside `DeviceFolderEntity` for per-folder timeline visibility and backup rules.
    * The dedicated compact sync endpoint populates cloud metadata, while `MediaStore` queries populate local metadata
      into the same table.
    * Layout calculations run directly over Room data (`ORDER BY year_month DESC, sort_timestamp DESC, id DESC`),
      enabling a 100% offline-first architecture with zero UI placeholder jumping.
3. **Local vs Cloud Identity & Two-Pass Deduplication:**
    * Media items in Room contain:
        * `localContentUri`: `content://media/external/images/media/1234` (nullable)
        * `cloudId`: Backend 10-char alphanumeric string `a1B2c3D4e5` (nullable)
        * `bucketId`: `MediaStore` bucket ID for local items (nullable, indexed)
        * `bucketName`: Display folder name, e.g., `"Camera"`, `"Screenshots"` (nullable)
        * `relativePath`: Relative storage path, e.g., `"DCIM/Camera/"`, `"Pictures/Screenshots/"` (nullable)
        * `sizeBytes`: File size in bytes (indexed, from `CompactTimelineItem` or `MediaStore`)
        * `ratio`: Aspect ratio `width / height`
        * `sortTimestamp`: UTC epoch millis
        * `timezoneOffsetMinutes`: Offset of local capture time from `sortTimestamp`
        * `yearMonth`: Precomputed `YYYYMM` integer for contiguous local-month grouping
        * `fileHash`: File hash matching backend `media_item.hash` (nullable, computed lazily)
        * `syncStatus`: `LOCAL_ONLY`, `SYNCING`, `SYNCED`, `CLOUD_ONLY`
    * **Matching Pipeline:**
        * *Fast Pass:* On startup/scan, query `CLOUD_ONLY` rows by `size_bytes` and match in Kotlin on `ratio` + exact
          or 15-minute timezone-aligned timestamp (`±2s` within `±24h`). Matched items flip to `SYNCED` with zero disk
          hashing.
        * *Lazy Crypto Verification:* For unmatched local items staged for upload (and rare `size_bytes` collisions
          with >24h timestamp drift), compute the file hash lazily and verify against the backend prior to streaming
          bytes.
4. **Coil 3 Thumbnail Strategy:**
    * **Target SDK:** (Android 17 and up (API level 37)) provides native hardware-accelerated AVIF decoding via
      `ImageDecoder`.
    * If `syncStatus == LOCAL_ONLY`: Image source is `localContentUri` (instant system thumbnail).
    * If `syncStatus == CLOUD_ONLY`:
        * If `hasThumbnails == true`: Image source is `"${BASE_URL}/thumbnails/${cloudId}/${targetHeight}p.avif"` cached
          in Coil's disk LRU cache.
        * If `hasThumbnails == false` (ingest in progress): Image source is
          `"${BASE_URL}/api/photos/${cloudId}/thumbnail?size=${targetHeight}"`
    * If `syncStatus == SYNCED`: Prefer `localContentUri` while on device storage; fall back to cloud thumbnail URL if
      deleted locally.
5. **Resumable Uploads via Foreground WorkManager:**
    * `UploadWorker` executes TUS resumable uploads in 5 MB chunks (tus uploads work with JWT ticket). Only `LOCAL_ONLY`
      items whose folder has `backup_enabled = 1` in `device_folder` are auto-enqueued.
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
│   │   ├── dao/            # MediaDao, DeviceFolderDao, AlbumDao, UploadQueueDao
│   │   └── entity/         # MediaEntity, DeviceFolderEntity, AlbumEntity, UploadQueueEntity
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
│   ├── model/              # MediaItem, DeviceFolder, SyncStatus, TimelineRowUiItem, Album
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
    ├── albums/             # Albums listing, Device Folders ("On this device"), & album detail views
    └── backup/             # Backup settings & status
```

---

### Part 4: Screens & Views Breakdown

| Screen                                   | Core UI Components                                                                                                                                                                         | Key Gestures & Interactions                                                                                                                                                        |
|:-----------------------------------------|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **`TimelineScreen`**                     | • `LazyColumn` of justified rows (cloud items + visible camera folders)<br>• Right-edge `TimelineScrubber`<br>• Floating `DateOverlay`<br>• Contextual multi-selection top bar             | • Fast vertical dragging on scrubber to jump dates (with `isScrollingFast` blanking)<br>• Long-press + drag down to multi-select items<br>• Tap item → navigate into `MediaViewer` |
| **`MediaViewerScreen`**                  | • `HorizontalPager` for smooth swiping<br>• `Telephoto` zoomable photo viewport<br>• Reused single `ExoPlayer` instance for video items<br>• Bottom action bar (Share, Edit, Delete, Info) | • Double-tap to zoom, pinch-to-zoom<br>• Swipe down to dismiss back to timeline<br>• Swipe up to expand metadata / EXIF / weather sheet                                            |
| **`AlbumsScreen` & `AlbumDetailScreen`** | • `LazyVerticalGrid` (square cards) for album list<br>• Justified row grid for album contents                                                                                              | • Tap album card → open album timeline<br>• Add photos to album via selection flow                                                                                                 |
| **`DeviceFoldersScreen`**                | • "On this device" folder grid (`Screenshots`, `Download`, 3rd-party cameras, etc.)<br>• Per-folder **Back up** & **Show in timeline** toggles<br>• Justified row grid for folder contents | • Browse non-camera folders without cluttering main timeline<br>• Toggle backup on for a folder (auto-enables timeline visibility) or manually upload individual items             |
| **`BackupSettingsScreen`**               | • Discovered device folder checklist (defaults to `DCIM/*` excluding Screenshots)<br>• Constraint switches (Wi-Fi only, while charging)<br>• Live upload queue progress bar                | • Toggle auto-backup globally or per folder<br>• "Sync Now" manual trigger                                                                                                         |
| **`AuthScreen`**                         | • Server URL input (defaults to self-hosted domain)<br>• Email & password inputs                                                                                                           | • Validates connection via `GET /api/health`<br>• Stores session cookies in persistent `CookieJar` across app launches                                                             |

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
