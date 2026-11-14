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

* Each item in the `LazyColumn` is a sealed `TimelineListItem` representing either a **Month Header** or a **Row of 2–5
  photos**, typed via `contentType` so Compose recycles row and header compositions independently without conditional
  header branches inside rows.
* In Compose, a row item is a `Row` containing photo composables with **explicit pixel-snapped `.width(item.widthDp)`
  and `.height(row.heightDp)`** (never `Modifier.weight(aspectRatio)`, which forces extra intrinsic/weighted measure
  passes).
* Measure the timeline container width **once** at the screen level (`onSizeChanged` / outer `BoxWithConstraints`),
  never inside `LazyColumn` items.
* **Why this is fast:** Compose treats each row as a single virtualized element with uniform slot structure. Measurement
  overhead is virtually zero during scroll because row heights, item widths, and pixel target sizes are precomputed on
  background threads.

```
LazyColumn (contentType = Header | PhotoRow)
├── Item 0: MonthHeaderUiItem("October 2026")
├── Item 1: TimelineRowUiItem(height = 180.dp, items = [PhotoA, PhotoB, PhotoC])
├── Item 2: TimelineRowUiItem(height = 195.dp, items = [PhotoD, PhotoE])
└── ...
```

---

#### 2. Layout Calculation Engine (`TimelineLayoutCalculator`)

Just like the web app's `calculateLayout` in `TimelineContainer.vue`, the math runs strictly off the UI thread (
`Dispatchers.Default`) when the window width, orientation, or density changes. Because Room acts as the Single Source of
Truth, the layout calculator receives slim, fully resolved projections directly—with `yearMonth` precomputed on write so
month boundary detection is a simple integer comparison with zero date parsing during layout.

To maximize Compose skipping and avoid recompositions during scroll or background syncs, mark UI models `@Immutable` and
back row item lists with `ImmutableList`:

```kotlin
sealed interface TimelineListItem {
    val id: String
    val estimatedOffsetTopDp: Float
}

@Immutable
data class MonthHeaderUiItem(
    override val id: String, // e.g. "header_202610"
    val yearMonth: Int,
    val title: String,       // Pre-formatted off main thread, e.g. "October 2026"
    override val estimatedOffsetTopDp: Float
) : TimelineListItem

@Immutable
data class TimelineRowUiItem(
    override val id: String, // Stable key from first item, e.g. "row_${items.first().id}"
    val heightDp: Float,
    val items: ImmutableList<TimelineGridItemUiModel>,
    val yearMonth: Int,      // Precomputed e.g. 202610
    val isFirstOfMonth: Boolean,
    val isLastOfMonth: Boolean,
    override val estimatedOffsetTopDp: Float,
    val thumbnailTargetHeightPx: Int // Ladder height selected in px (e.g. 240, 360, 480, 720)
) : TimelineListItem

@Immutable
data class TimelineGridItemUiModel(
    val id: String,
    val localContentUri: String?,
    val cloudId: String?,
    val isVideo: Boolean,
    val durationMs: Int,
    val hasThumbnails: Boolean,
    val widthDp: Float,      // Precomputed, pixel-snapped to prevent 1–2px right-edge gaps
    val ratio: Float,
    val syncStatus: SyncStatus
)
```

*Keep active upload progress and transient `SYNCING` / `UPLOADED_PENDING_SYNC` states (from `UploadQueueEntity`) in a
separate Flow/map observed only by visible cells—never mutate `media_item` per upload tick or start, so upload activity
never re-triggers `CalculateTimelineLayoutUseCase` across 50k items.*

**Row Partitioning Algorithm:**

1. Target desired row height: `180.dp` on phone portrait, `240.dp` on tablet/landscape, scaled by user zoom settings.
2. Accumulate items into the current row:
   $$\text{targetWidth} = \sum (\text{aspectRatio}_i \times \text{desiredHeight}) + (N - 1) \times \text{gap}$$
3. When $\text{targetWidth} \ge \text{containerWidth}$:
   $$\text{scaleFactor} = \frac{\text{containerWidth} - \text{totalGaps}}{\sum (\text{aspectRatio}_i \times \text{desiredHeight})}$$
   $$\text{actualRowHeight} = \text{clamp}(\text{desiredHeight} \times \text{scaleFactor}, \; \text{minHeight}, \; \text{desiredHeight} \times 1.5)$$
4. Compute each item's snapped `widthDp` (`roundToInt` in pixels converted back to dp); assign any remaining 1–2 px on
   a full row to the last item so rows always end flush against the right edge.
5. Select `thumbnailTargetHeightPx` from the server ladder (`[144, 240, 360, 480, 720, 1080, 1440]`) using **actual
   rendered pixels (`rowHeightDp * density`)**, not `dp` (on `3x–4x` phones a `180.dp` row is `540–720 px`; picking by
   `dp` yields blurry `240p` thumbnails). Cap grid thumbnails at `720` (or `480` on low-RAM devices) to keep AVIF
   decoding fast.
6. Finalize the row, compute its cumulative vertical offset (`estimatedOffsetTopDp`, as well as on each
   `MonthHeaderUiItem`), and advance.
7. If the `yearMonth` boundary ends on an incomplete row, cap its scale factor to `1.0` so the last 1 or 2 items don't
   stretch into giant blocks.

---

#### 3. Unifying Local MediaStore + Cloud Protobuf: Dedicated Compact Sync & Room SSOT

Instead of the web app's anonymous ratio arrays and multi-stage month hydration, the mobile client uses a **Dedicated
Compact Protobuf Sync Endpoint** (to be implemented) paired with **Room as the Single Source of Truth (SSOT)**. This
guarantees full offline capability and instant local-to-cloud reconciliation without UI placeholders.

**Tombstone Deletion Tracking & Backend Indexing:**

* Permanent deletions are recorded in a lightweight `deleted_media_item` (tombstone, populated by an `AFTER DELETE` db
  trigger, to be implemented) table with the purged IDs, `user_id`, and deletion timestamp. **Do not** add a foreign key
  with `ON DELETE CASCADE` on `deleted_media_item.user_id` referencing `app_user(id)`, because deleting a user cascades
  into `media_item` deletes whose `AFTER DELETE` trigger would try to insert rows for the deleted user (failing FK
  checks or causing unnecessary churn). Periodically prune old tombstones (e.g., >30–90 days) and fall back to a full
  sync if `since` is older than retention.
* Every backend write/update to `media_item` (including metadata re-ingest and bin/restore operations) explicitly sets
  `updated_at = now()`, so no `UPDATE` trigger is needed.
* Add a composite index on `media_item (user_id, updated_at)` on Postgres so delta sync queries (`updated_at > since`)
  remain fast at 50k+ rows.
* Note that `size_bytes` lives on `media_features` (`JOIN media_features mf ON mf.media_item_id = m.id`), and
  `POST /api/upload/check-hashes` still needs to be implemented on the backend (using `media_item.hash`, which already
  stores the full-file BLAKE3 lowercase hex digest and is indexed by `idx_media_item_user_hash`; because
  `media_item.hash` is not `UNIQUE` on the server, `check-hashes` should return all non-deleted matching `cloud_id`s
  per hash:
  `SELECT hash, array_agg(id ORDER BY created_at ASC) AS cloud_ids FROM media_item WHERE user_id = $1 AND deleted = false AND hash = ANY($2) GROUP BY hash`).

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                 Dedicated Compact Sync (Initial & Delta)                     │
│  • Remote: GET /api/timeline/sync?since=... (Compact Protobuf, gzip)  │
│    Payload: [id, sort_timestamp, tz_offset_min, ratio, is_video,             │
│              has_thumbnails, size_bytes]                                     │
│  • Computes `year_month` and writes in chunked batch inside one @Transaction │
└─────────────────────────────────────┬────────────────────────────────────────┘
                                      │
                                      ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                 Room Database: Single Source of Truth (SSOT)                 │
│  • Houses local-only, cloud-only, and merged/synced items                    │
│  • Denormalized `is_visible_in_timeline` + composite index (zero temp sort)  │
│  • Stage 1 launch query: top 2 months (~20ms to LCP); Stage 2: full library  │
│  • Deduplication via zero-hash fast pass: size_bytes + ratio + tz-aligned ts │
│  • 100% offline-ready: entire timeline renders instantly from SQLite        │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Concurrency Safety Overlap (`since`):**
If an `IngestMetadata` transaction starts at `T = 100` (setting `updated_at = now() = 100` in Postgres) and commits at
`T = 105`, while a mobile sync runs at `T = 102` and saves `sync_timestamp = 102`, the next delta sync
(`updated_at > 102`) would miss that item.
*Fix:* Subtract a small safety overlap from `since` (e.g., `since - 60_000L`, 60 seconds). Since upserts and
`deleted_ids` are idempotent in Room, overlapping by a minute costs almost nothing and eliminates missed concurrent
ingests.

##### Sync Protocol & Data Transfer

We'll have to refactor the `timeline.proto` into `common.proto`, `web.proto`, and `mobile.proto`.

1. **Compact Timeline Protobuf (`mobile.proto` / `common.proto`):**
   ```protobuf
   message TimelineSyncResponse {
     // New or modified media items to be upserted into Room
     repeated CompactTimelineItem upserted = 1;
     // IDs of items moved to Bin/Trash or permanently purged via tombstones
     repeated string deleted_ids = 2;
     // Server timestamp (epoch ms) to persist and pass as `since` in the next sync request
     int64 sync_timestamp = 3;
     // True when `since` was omitted or older than tombstone retention (client must diff snapshot)
     bool is_full_snapshot = 4;
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
3. **Fast Timeline Room Query (Denormalized Visibility + Slim Projection + Composite Index):**
    * Avoid `OR` across `media_item LEFT JOIN device_folder` in the hot timeline query—at 50k rows, SQLite falls back to
      a full-table scan and a temporary B-tree sort (`USE TEMP B-TREE FOR ORDER BY`), and `SELECT m.*` pulls unused
      strings (`relative_path`, `bucket_name`, `file_hash`, etc.) across the `CursorWindow`.
    * **Fix:**
        1. Store `is_visible_in_timeline: Boolean` (0/1) directly on `media_item`: `1` for all `CLOUD_ONLY` items and
           the **canonical** `SYNCED` row per `cloudId` (`0` for secondary local duplicates of the same `cloudId`);
           copied from `device_folder.show_in_timeline` for `LOCAL_ONLY` items and updated in a single SQL `UPDATE` when
           a folder toggle changes.
        2. Create a composite index on `media_item`:
           `(is_visible_in_timeline, year_month DESC, sort_timestamp DESC, id DESC)` so SQLite reads in exact timeline
           order with zero temp B-tree sort step (note: SQLite does not support PostgreSQL's `INCLUDE (...)` syntax;
           avoid bloating the index key with wide string columns like `local_content_uri`).
        3. Query a slim `TimelineGridProjection` (`id`, `cloud_id`, `local_content_uri`, `year_month`, `ratio`,
           `is_video`, `duration_ms`, `has_thumbnails`, `sync_status`):
           ```sql
           -- Stage 1 (Viewport fast path on cold start: top 2 months)
           SELECT id, cloud_id, local_content_uri, year_month, ratio, is_video, duration_ms, has_thumbnails, sync_status
           FROM media_item
           WHERE is_visible_in_timeline = 1
             AND year_month IN (
               SELECT DISTINCT year_month FROM media_item
               WHERE is_visible_in_timeline = 1
               ORDER BY year_month DESC
               LIMIT 2
             )
           ORDER BY year_month DESC, sort_timestamp DESC, id DESC;
   
           -- Stage 2 (Full library layout)
           SELECT id, cloud_id, local_content_uri, year_month, ratio, is_video, duration_ms, has_thumbnails, sync_status
           FROM media_item
           WHERE is_visible_in_timeline = 1
           ORDER BY year_month DESC, sort_timestamp DESC, id DESC;
           ```
        4. Sorting by `year_month DESC` before `sort_timestamp DESC` guarantees contiguous month groups matching the
           backend—so cross-timezone travel around midnight on the 1st of a month never splits a month header into two
           sections.
4. **Local Folder Scope & Google Photos-Style "On This Device" Filtering:**
    * **Scan everything, filter via `is_visible_in_timeline`:** `LocalMediaScanner` queries **all** non-pending,
      non-trashed `MediaStore` images and videos (`IS_PENDING = 0 AND IS_TRASHED = 0 AND SIZE > 0`) and records
      `bucketId`, `bucketName`, and `relativePath` on `MediaEntity`. A lightweight `device_folder` Room table
      (`bucket_id`, `display_name`, `relative_path`, `show_in_timeline`, `backup_enabled`) stores folder preferences.
    * **Default folder rules (on first discovery of a `bucketId`):**
        * **Camera folders (`showInTimeline = true`, `backupEnabled = true`):**
          `relativePath.startsWith("DCIM/", ignoreCase = true)` **excluding** screenshot/screen-recording folders
          (e.g., `!relativePath.startsWith("DCIM/Screenshots", ignoreCase = true)` and
          `!relativePath.startsWith("DCIM/Screen", ignoreCase = true)`). This automatically includes `DCIM/Camera/`
          and 3rd-party cameras under `DCIM/` (e.g. `DCIM/OpenCamera/`) while avoiding the Samsung/Xiaomi bug where
          `DCIM/Screenshots/` pollutes the camera roll.
        * **Other device folders (`showInTimeline = false`, `backupEnabled = false`):** Everything else (
          `Pictures/Screenshots/`, `DCIM/Screenshots/`, `Download/`, `Pictures/WhatsApp/`, or 3rd-party cameras saving
          outside `DCIM/`). These stay out of the main timeline and are never uploaded unless the user opts in.
    * **Web vs. Mobile parity:** Only backed-up/uploaded items ever reach the server, so non-backed-up screenshots and
      downloads never clutter the web timeline.
    * **Device Folders ("On this device") & Deduplication:** All discovered folders remain browsable in a Device Folders
      view / Backup Settings (`GROUP BY bucket_id`), where users can toggle **Back up** (which automatically enables
      `showInTimeline` and flips `is_visible_in_timeline = 1` for `LOCAL_ONLY` items in that bucket) or **Show in
      timeline** per folder. Crucially, **deduplication still runs across all scanned folders**, so if a user manually
      uploads a single screenshot via Share or Device Folders, it links cleanly as `SYNCED` instead of duplicating.
5. **Delta Syncing (`since` parameter):**
    * **Initial / Full Snapshot (`is_full_snapshot = true`):** Query `GET /api/timeline/sync` without `since`
      (or receive `is_full_snapshot = true` if `since` predates tombstone retention). Backend streams all non-deleted
      items (`deleted = false`, no tombstones). Room inserts them in chunked batches (to stay below SQLite's
      bind-parameter limit) inside a single `@Transaction` and stores `latestSyncTimestamp`. To clean up any missed
      server deletions when `is_full_snapshot == true`, diff `SELECT DISTINCT cloud_id FROM media_item WHERE cloud_id IS
      NOT NULL` (excluding any `cloud_id` just linked via `check-hashes` in this pass) against `syncedCloudIds` in
      Kotlin—avoiding `NOT IN (:50kIds)` bind limits—and pass `staleCloudIds` (chunked in batches of ~500) through the
      same cloud-deletion transition used for `deleted_ids`.
    * **Subsequent Launches:** Query `GET /api/timeline/sync?since={latestSyncTimestamp - 60_000}`. Backend
      returns `upserted` (`deleted = false AND updated_at > since`) and `deleted_ids` unioning soft-deletes
      (`media_item WHERE deleted = true AND updated_at > since`) and hard-deletes (
      `deleted_media_item WHERE deleted_at > since`), or falls back to `is_full_snapshot = true` if `since` is older
      than tombstone retention.
    * To handle possible out-of-sync situations, do a full sync (without `?since`) periodically (e.g., weekly) and on
      pull-to-refresh.
6. **Room Invalidation & Coalescing:**
    * Always wrap sync and `MediaStore` batch updates in a single Room `@Transaction` (`withTransaction { ... }`) so
      Room emits one `Flow` invalidation per batch rather than per row.
    * Apply `.debounce(150)` on background Room timeline `Flow` emissions (after the initial Stage 1/Stage 2 emission)
      so overlapping `MediaStore` scans and sync batches coalesce into a single `CalculateTimelineLayoutUseCase` pass.

##### Two-Pass Local/Cloud Deduplication Strategy

To prevent cold-start battery drain and disk thrashing, **the mobile app never hashes local photos on startup**:

1. **Pass 1: Instant Fast-Pass (Zero Hashing, Timezone-Safe):**
    * Android `MediaStore.DATE_TAKEN` often differs from the server's `sort_timestamp` by exact timezone offsets (e.g.,
      naive EXIF `DateTimeOriginal` interpreted in the phone's current timezone vs. server GPS/fallback timezone, OEM
      QuickTime `creation_time` written in local time, or DST shifts).
    * Query `MediaStore` (filtering `IS_PENDING = 0 AND IS_TRASHED = 0 AND SIZE > 0`) for `_ID`, `WIDTH`, `HEIGHT`,
      `ORIENTATION`, `SIZE`, `DATE_TAKEN`, `DATE_MODIFIED`, `BUCKET_ID`, `BUCKET_DISPLAY_NAME`, and `RELATIVE_PATH`.
      **Swap `WIDTH` and `HEIGHT` when `ORIENTATION` is `90` or `270`** (backend `media_item` dimensions are already
      orientation-adjusted; `MediaStore` often reports raw sensor dimensions), and guard against `width <= 0` or
      `height <= 0` (falling back to `1f` so malformed metadata never produces `NaN`/`Infinity` aspect ratios).
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
    * Filtering on `sync_status = 'CLOUD_ONLY'` prevents two identical local copies from matching the same `cloudId`
      without a hash check. A match immediately links `localContentUri` to `cloudId` and marks `syncStatus = SYNCED`
      with zero disk I/O.
2. **Pass 2: On-Demand Crypto Verification (Deferred to Backup/Upload):**
    * For local items that did *not* match in Pass 1 (including duplicate local copies of an already-`SYNCED` file, or
      rare cases where `size_bytes` + `ratio` matched a `CLOUD_ONLY` item whose timestamp differed by >24h due to a web
      date edit or missing `DATE_TAKEN`), the BLAKE3 hash (lowercase hex) is calculated **only when staged for backup**
      (or lazily for those rare size-collision candidates), matching `media_item.hash` on the backend.
    * Before streaming bytes, a pre-upload check (`POST /api/upload/check-hashes`, to be implemented on the backend)
      queries the backend with the BLAKE3 hash and receives all non-deleted matching `cloud_id`s (`cloud_ids`). If
      non-empty, the upload is skipped, and Room prefers merging into an unlinked `CLOUD_ONLY` row matching one of the
      returned `cloud_ids` or otherwise links the local row to the first `cloud_id` as `SYNCED` (respecting the
      canonical visibility rule below).

##### TIPS

`DATE_TAKEN` can be `NULL` or `0`: Screenshots, WhatsApp media, and downloaded images often lack `DATE_TAKEN` in
`MediaStore`. Fall back to `DATE_MODIFIED * 1000L` (note: `DATE_ADDED` and `DATE_MODIFIED` are in **seconds**, whereas
`DATE_TAKEN` is in **milliseconds**!).

##### How `MediaEntity` handles primary keys, duplicate local copies, and state transitions

Keep a **single `media_item` table** (preserving the fast composite-index query with zero joins/unions) with:

* An immutable surrogate primary key `id` (e.g., `"local_<mediaStoreId>"` for local rows, `"cloud_<cloudId>"` for
  `CLOUD_ONLY` rows) so Compose keys and `ORDER BY year_month DESC, sort_timestamp DESC, id DESC` tie-breakers never
  jump during status transitions.
* `@Index(value = ["localContentUri"], unique = true)` — one row per local file URI.
* `@Index(value = ["cloudId"], unique = false)` — **non-unique** so secondary local copies (e.g., the same photo in
  `DCIM/Camera/` and `Download/`) resolved via Pass 2 `check-hashes` can also store `cloudId` and `syncStatus = SYNCED`
  without throwing `SQLiteConstraintException`, preventing infinite re-hash/re-upload loops while letting
  `DeviceFoldersScreen` and `"Free up space"` recognize all local copies as backed up.
* `autoBackupSuppressed: Boolean` (default `false`) — prevents items deleted from the server from being immediately
  re-uploaded when auto-backup is active for their folder.

> **Canonical Timeline Visibility Invariant:** For any `cloudId != null`, **at most one row** in `media_item` has
> `is_visible_in_timeline = 1`.

Handle all state transitions inside Room `@Transaction`s:

1. **Local item gets backed up → Cloud sync arrives (`SyncTimelineUseCase`):**
   `LOCAL_ONLY` (`local_1234`) is uploaded via TUS (with its `blake3Hash` already computed) and marked
   `UPLOADED_PENDING_SYNC` in `upload_queue`. Before applying `TimelineSyncResponse`, if any `UPLOADED_PENDING_SYNC`
   rows exist, `SyncTimelineUseCase` queries `POST /api/upload/check-hashes` with their BLAKE3 hashes (outside the
   Room transaction) and reconciles any returned `cloud_id`s inside the `@Transaction` using rule #2—ensuring files
   without EXIF dates (e.g. screenshots/WhatsApp where `MediaStore.DATE_MODIFIED` differs from server `sort_timestamp`
   by `> 24h`) link cleanly instead of spawning duplicate `CLOUD_ONLY` rows. Mark `UPLOADED_PENDING_SYNC` rows as
   `FAILED` after a max-age/retry threshold if server `IngestMetadata` permanently failed. For any remaining unlinked
   `LOCAL_ONLY` rows, `SyncTimelineUseCase` still runs Pass 1 deduplication when a new `cloudId` arrives.
2. **Pass 2 `check-hashes` returns matching `cloud_ids` (`[c1, c2, ...]`):**
    * **First, check for an unlinked `CLOUD_ONLY` row:** Query
      `SELECT * FROM media_item WHERE cloud_id IN (:cloudIds) AND sync_status = 'CLOUD_ONLY' LIMIT 1`. If found, merge
      the `LOCAL_ONLY` row into that `CLOUD_ONLY` row (copy `localContentUri`, `bucketId`, `bucketName`, `relativePath`,
      `blake3Hash` onto it, flip `syncStatus = SYNCED`, clear `autoBackupSuppressed = false`, and `DELETE` the old
      `LOCAL_ONLY` row) so an unlinked cloud copy is never left stranded in the timeline.
    * **Otherwise (all matching `cloudIds` are already `SYNCED`, or not yet synced into Room):** Pick
      `cloudId = cloudIds.first()`
      and check `SELECT EXISTS(SELECT 1 FROM media_item WHERE cloud_id = :cloudId AND is_visible_in_timeline = 1)`:
        * If `true`, link `cloudId` and set `syncStatus = SYNCED` with `is_visible_in_timeline = 0` (secondary duplicate
          hidden from main timeline, still visible in its `DeviceFolder` and eligible for `"Free up space"`).
        * If `false`, link `cloudId`, set `syncStatus = SYNCED`, and set `is_visible_in_timeline = 1`.
3. **Server updates a cloud item (`SyncTimelineUseCase`):**
   `UPDATE media_item SET has_thumbnails = ..., sort_timestamp = ..., timezone_offset_minutes = ..., year_month = ... WHERE cloud_id = :cloudId`
   (updates the canonical row and any linked secondary copies).
4. **Cloud item deleted (`deleted_ids` or full-snapshot `staleCloudIds` contains `a1B2c3D4e5`, or user selects "Delete
   from Server"):**
   Do **not** blindly `DELETE FROM media_item WHERE cloud_id IN (:deletedIds)`! For every row
   `WHERE cloud_id = :cloudId`:
    * If `local_content_uri == null` → `DELETE` the row.
    * If `local_content_uri != null` → flip `syncStatus = LOCAL_ONLY`, `cloudId = null`, **`autoBackupSuppressed = true`
      **,
      and recompute `is_visible_in_timeline` from `device_folder.show_in_timeline`. Without `autoBackupSuppressed`,
      `EnqueueBackupUseCase` would immediately treat the item as an unbacked-up `LOCAL_ONLY` file and re-upload it on
      the next pass. Clear `autoBackupSuppressed = false` only if the user explicitly triggers a manual upload/back up
      for that item.
5. **Local file deleted from device (`ScanLocalMediaUseCase` or "Free up space"):**
   **Permission & empty-cursor guard:** `ScanLocalMediaUseCase` must only treat omission from the `MediaStore` cursor as
   device deletion for a media type when full access is granted (`READ_MEDIA_IMAGES` for images, `READ_MEDIA_VIDEO` for
   videos), and must abort deletion reconciliation if `MediaStore` returns 0 rows while Room has local rows. Under
   `READ_MEDIA_VISUAL_USER_SELECTED` (partial access), skip cursor-omission cleanup—only run this transition on
   explicit app deletion (`MediaStore.createDeleteRequest`) or when probing a URI throws `FileNotFoundException` (never
   `SecurityException`, which only indicates temporary grant revocation). For each confirmed removed local row:
    * If `cloud_id == null` → `DELETE` the row.
    * If `cloud_id != null` and **another row** with the same `cloud_id` exists → `DELETE` this row; if it was the
      canonical visible row (`is_visible_in_timeline == 1`), promote one remaining sibling row to
      `is_visible_in_timeline = 1`.
    * If `cloud_id != null` and **no other row** with the same `cloud_id` exists → flip to `syncStatus = CLOUD_ONLY`,
      `localContentUri = null`, and `is_visible_in_timeline = 1`.

##### Auth, Network & Connectivity Tips

* **Concurrent 401 refresh lock:**
  `server/config/settings.yaml` sets `access_token_expiry_minutes: 30` and `20251004204735_init.sql` stores hashed
  `refresh_token` selectors/verifiers (token rotation). If parallel requests (e.g., timeline sync + Coil thumbnail
  fetches) hit `401` when the 30-minute cookie expires, an unsynchronized OkHttp `Authenticator` will send concurrent
  `/api/auth/refresh` calls—the first rotates the token and the rest fail and log the user out.
    * **Fix:**
        1. Guard `/api/auth/refresh` in your OkHttp `Authenticator` with a `synchronized` / `Mutex` block.
        2. Inside the lock, compare the `access_token` cookie on `response.request` against the latest `access_token`
           currently in `CookieJar`; if it already changed, skip calling refresh and retry the request with the updated
           cookie.
        3. Return `null` immediately if the failing path is `/api/auth/refresh` or `/api/auth/login` (and cap retries
           via `responseCount(response) >= 2`) to prevent infinite 401 loops.
    * Keep `api.rate_limiting` (`req_per_second: 6, burst_size: 10`) in mind for `auth_public_router`—never spam
      `/api/auth/login` or `/api/auth/refresh`.
* **Cookie `Secure` flag & custom URLs:**
  `server/config/settings.yaml` uses `cookie_secure: false` in local dev and `true` behind production HTTPS. Ensure
  `PersistentCookieJar` respects `Secure`, `Domain`, and `Path` attributes properly when the user configures a custom
  server URL (including ports).
* **Thumbnail concurrency:**
  Raise OkHttp's `dispatcher.maxRequestsPerHost` from the default `5` to `12–16` so visible cloud thumbnails load in
  parallel rather than queuing in batches of 5 (`/thumbnails` is static and not rate-limited by `auth_public_router`).
* **Dynamic Server URL & API 37 Cleartext Policy:**
  Since `AuthScreen` lets the user input a server URL, don't hardcode Retrofit's `baseUrl` at DI graph creation time.
  Either swap/recreate the Retrofit instance on URL change or rewrite `request.url` in an OkHttp interceptor from the
  persisted server URL. Because Android 17 (API 37) deprecates `android:usesCleartextTraffic`, add an explicit
  `network_security_config.xml` (or debug-only config) if testing against `http://10.0.2.2:5272` or LAN HTTP.
* **Connectivity & Sync Status Indicator (`NetworkMonitor`):**
  Observe connectivity via `ConnectivityManager.NetworkCallback` alongside a `SyncState` (`Idle`, `Syncing`, `Offline`,
  `ServerUnreachable`). Do not surface transient network errors as fatal errors when offline; display a subtle status
  indicator in the UI instead, since local Room + `MediaStore` items remain fully browsable offline.

##### Non-Initial App Launch Flow (Optimized for LCP):

* **T = 0ms (Stage 1 — Fast Viewport LCP):** Query only the top 2 `year_month`s from Room using the composite index
  (`is_visible_in_timeline = 1`), compute their rows on `Dispatchers.Default`, and emit immediately (~15–30ms).
  `LazyColumn` renders the first screenful and Coil starts decoding visible thumbnails right away—never block launch on
  `/api/auth/me` or network sync.
* **T + ~20ms (Stage 2 — Full Timeline):** Read the slim projection for the full library from Room, partition all rows +
  build `TimelineLayoutIndex` on `Dispatchers.Default`, and emit. Because Stage 1 ended on a clean `year_month`
  boundary, the visible rows remain identical with zero layout jump.
* **T + 50ms:** `MediaStore` scan reconciles new (and, when full permissions are granted, deleted) local photos across
  all folders via the zero-hash fast pass and registers any newly seen `bucketId`s in `device_folder` with default
  visibility/backup flags.
* **T + 150ms:** Background delta sync calls `GET /api/timeline/sync?since=...` (if online). Any cloud changes
  are committed to Room in a single transaction, triggering a debounced layout update.

---

#### 4. Instant Fast-Scrubbing with Variable Row Heights & Viewer Scroll Sync

Because each row has a slightly different height, native scroll thumbs can jitter. Solve this with a **Layout Index**:

```kotlin
class TimelineLayoutIndex(
    val rows: List<TimelineListItem>,
    val totalHeightDp: Float,
    val monthOffsets: Map<Int, Int>, // yearMonth (e.g. 202610) -> LazyColumn item index
    val itemOffsets: Map<String, Int> // mediaId -> LazyColumn item index
) {
    fun findRowIndexAtPercentage(fraction: Float): Int {
        val targetOffset = fraction * totalHeightDp
        // Binary search rows and headers by estimatedOffsetTopDp: O(log N)
        val idx = rows.binarySearchBy(targetOffset) { it.estimatedOffsetTopDp }
        return if (idx < 0) (-idx - 1).coerceIn(0, rows.lastIndex) else idx
    }

    fun findIndexForDate(yearMonth: Int): Int = monthOffsets[yearMonth] ?: 0

    fun findRowIndexForMedia(mediaId: String): Int? = itemOffsets[mediaId]
}
```

**Fast-Scrubbing FPS, Viewer Sync & Gesture Rules:**

* **Instant jumps & smooth thumb tracking:** Always use `lazyListState.scrollToItem(index)` while dragging the
  `TimelineScrubber`—never `animateScrollToItem`. When the user scrolls the `LazyColumn` normally, compute the scrubber
  thumb fraction from `rows[lazyListState.firstVisibleItemIndex].estimatedOffsetTopDp / totalHeightDp` rather than
  `firstVisibleItemIndex / rows.size` so variable row heights don't make the thumb jitter.
* **Image request gating while scrubbing (`isScrubbing`):** Mirroring `isScrollingFast` in `TimelineContainer.vue`,
  while the user is actively dragging the scrubber rapidly, skip mounting new network/full-decode `AsyncImage` requests
  (or restrict Coil to `memoryCache`/`diskCache` reads only) and resume full loading once scrubbing pauses (~60ms) or
  lifts. This prevents transient rows from flooding OkHttp and `ImageDecoder` with dozens of cancelled requests per
  second.
* **Viewer-to-Timeline Scroll Sync:** Mirroring `TimelineContainer.vue`, if the user opens `MediaViewerScreen`, swipes
  across many photos in `HorizontalPager`, and dismisses back to `TimelineScreen`, check whether the final `mediaId` is
  outside the visible `LazyColumn` viewport and jump/scroll to `layoutIndex.findRowIndexForMedia(mediaId)` so the user
  lands on the item they were just viewing.
* **Edge-to-edge & gesture navigation safety (API 37):** Apply `Modifier.systemGestureExclusion()` on the right-edge
  scrubber handle so vertical scrubbing doesn't trigger Android's predictive back gesture, and pass
  `WindowInsets.statusBars` / `navigationBars` into `LazyColumn` `contentPadding` and scrubber vertical bounds.

---

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
│   DeduplicateMediaUseCase, EnqueueUploadUseCase)             │
│   Domain Models (MediaItem, DeviceFolder, SyncStatus, Row)   │
└──────────────────────────────┬───────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────┐
│                       Data Layer                             │
│  Repositories (MediaRepository, BackupRepository)            │
│  Local: Room DB, MediaStore DataSource, Layout Index         │
│  Remote: OkHttp + Persistent CookieJar, Compact Sync API     │
│  Background: WorkManager (UploadWorker, LocalScanWorker)     │
└──────────────────────────────────────────────────────────────┘
```

#### Key Architecture Principles for this App:

1. **Cookie-Based Session Authentication & Persistent CookieJar:**
    * Matches the web client (`web/src/scripts/services/api.ts`) by relying exclusively on HTTP cookies (`access_token`,
      `refresh_token`) set by the backend on `POST /api/auth/login`.
    * OkHttp is configured with a persistent `CookieJar` (backed by encrypted SharedPreferences or DataStore).
    * Every network call—including Retrofit API requests, Coil thumbnail image fetches, Media3 video streaming, and TUS
      upload initiation—shares this exact `OkHttpClient` instance and automatically includes the session cookies.
    * Transparent session refresh: A mutex-guarded OkHttp `Authenticator` triggers `POST /api/auth/refresh` when an
      access cookie expires, updating the cookie store seamlessly without re-prompting the user.
2. **Room Database as Single Source of Truth (SSOT):**
    * Room stores complete `MediaEntity` rows (local-only items, synced items, and cloud-only items), including
      precomputed `yearMonth`, `timezoneOffsetMinutes`, `isVisibleInTimeline`, `autoBackupSuppressed`, and local folder
      metadata (`bucketId`, `bucketName`, `relativePath`), alongside `DeviceFolderEntity` for per-folder timeline
      visibility and backup rules.
    * The dedicated compact sync endpoint populates cloud metadata, while `MediaStore` queries populate local metadata
      into the same table.
    * Layout calculations run directly over slim Room projections (
      `ORDER BY year_month DESC, sort_timestamp DESC, id DESC`),
      enabling a 100% offline-first architecture with zero UI placeholder jumping.
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
4. **Coil 3 Thumbnail Strategy (LCP & FPS Tuned):**
    * **Target & Min SDK:** Targeting Android 17 and up (API level 37, Pixel devices) provides native
      hardware-accelerated AVIF decoding via `ImageDecoder` and modern adaptive windowing (note API 37 ignores fixed
      orientation / non-resizable restrictions on large screens).
    * **Custom `MediaStoreThumbnailFetcher` (Scoped to Grid Thumbnails):** For `LOCAL_ONLY` and `SYNCED` items, wrap
      the URI in a dedicated `@Immutable data class MediaStoreThumbnail(val uri: Uri, val cloudId: String? = null)` and
      register `MediaStoreThumbnailFetcher.Factory : Fetcher.Factory<MediaStoreThumbnail>` + a matching `Keyer` on Coil
      instead of matching raw `content://` `Uri`s globally. Grid cells pass `MediaStoreThumbnail(localContentUri)` so
      Coil calls `ContentResolver.loadThumbnail(uri, Size(widthPx, heightPx), signal)`, while `MediaViewerScreen`
      (`ZoomableAsyncImage`) passes the raw `Uri` directly so Coil's built-in `ContentUriFetcher` and Telephoto's
      `SubSamplingImageSource.contentUri(...)` serve full-resolution tiles without blurry zoom (optionally passing the
      grid cell's `placeholderMemoryCacheKey`):
        * Wire `CancellationSignal` to the coroutine's `Job` (`invokeOnCompletion { signal.cancel() }`) so fast
          scrolling immediately aborts pending IPC thumbnail loads.
        * On `SecurityException` (e.g., temporary URI grant revoked after restart under partial access), fall back to
          the cloud thumbnail URL if `cloudId != null`.
        * On `loadThumbnail` failure, fall back to subsampled `ImageDecoder` for **images** and
          `MediaMetadataRetriever` / Coil's `VideoFrameDecoder` for **videos** (`ImageDecoder` cannot decode video
          URIs).
    * **If `syncStatus == CLOUD_ONLY` (or `SYNCED` after local file deletion):**
        * If `hasThumbnails == true`: `"${BASE_URL}/thumbnails/${cloudId}/${targetHeightPx}p.avif"` (no auth required)
        * If `hasThumbnails == false`: `"${BASE_URL}/api/photos/${cloudId}/thumbnail?size=${targetHeightPx}"` (requires
          auth)
    * **Grid ImageLoader tuning:**
        * Always pass exact pixel `.size(widthPx, heightPx)` on `ImageRequest`.
        * Keep hardware bitmaps enabled (`allowHardware(true)`).
        * **Disable `crossfade` in the timeline grid** to avoid simultaneous alpha-layer overdraw across 15+ cells.
        * Configure `MemoryCache` (~25% of heap), `DiskCache` (500 MB LRU), and wire the shared `OkHttpClient` (with
          `maxRequestsPerHost = 12–16`) into `OkHttpNetworkFetcherFactory`.
5. **Video Playback Strategy (`Media3` / `ExoPlayer`):**
    * **Shared `OkHttpClient` DataSource:** Media3's `DefaultHttpDataSource` does **not** use OkHttp or your
      `CookieJar`. Configure `ExoPlayer` with `media3-datasource-okhttp` (`OkHttpDataSource.Factory(okHttpClient)`) so
      authenticated remote video streams automatically carry session cookies and trigger transparent 401 token refresh.
    * **Surface ownership in `HorizontalPager`:** Because `HorizontalPager` composes adjacent offscreen pages, attach
      the single shared `ExoPlayer`'s `PlayerSurface` / `PlayerView` only to the **currently settled page**
      (`pagerState.settledPage == pageIndex`), rendering the photo thumbnail underneath until the first video frame
      renders so adjacent pages don't fight over the `Surface`.
    * **Source selection in `MediaViewerScreen`:**
        * **Local / Synced (`localContentUri != null`):** Play directly from `localContentUri` (instant, zero
          bandwidth, full quality).
        * **Cloud-only (`CLOUD_ONLY`):**
            * **`hasThumbnails == false`:** Thumbnails and WebM transcodes have not been generated yet, so **only** the
              on-demand original-quality endpoint (`/api/photos/{id}/video`) can be used.
            * **`hasThumbnails == true`:** Check whether the original video format/codec is playable on the phone (e.g.,
              via codec capabilities / `MediaCodecList` or fallback on `PlaybackException`):
                * If **playable**: let user pick quality from [possible webm qualities + original quality] to stream
                  the original-quality (`4K` / `1080p`) video from `"${BASE_URL}/api/photos/${cloudId}/video"`. This
                  endpoint supports HTTP `Range` requests for seeking and **requires authentication** (handled via
                  `OkHttpDataSource`).
                * If **not playable** natively on the phone (e.g., unsupported container/codec): fall back to the
                  transcoded WebM preview at `"${BASE_URL}/thumbnails/${cloudId}/480p.webm"` and let the user pick
                  quality from transcoded qualities. Note that `/thumbnails/...` does **not** require authentication.
6. **Resumable Uploads via WorkManager:**
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

### Part 3: Recommended Package Structure

```text
app/src/main/java/com/ruurd/photos/
├── core/
│   ├── common/             # Dispatchers, Result wrapper, Extensions
│   ├── database/           # Room Database, TypeConverters
│   │   ├── dao/            # MediaDao, DeviceFolderDao, AlbumDao, UploadQueueDao
│   │   └── entity/         # MediaEntity, DeviceFolderEntity, AlbumEntity, UploadQueueEntity
│   ├── network/            # OkHttp, Retrofit, PersistentCookieJar, SessionManager, NetworkMonitor
│   │   ├── api/            # PhotosApiService (Retrofit Protobuf Lite)
│   │   ├── cookie/         # PersistentCookieJar implementation (DataStore-backed)
│   │   └── tus/            # Resumable TUS upload client
│   ├── image/              # Coil ImageLoader setup, MediaStoreThumbnailFetcher
│   └── ui/                 # Shared UI components, Theme (M3 Dynamic Color), Dimens
│       └── components/     # ThumbnailImage, FastScroller, DateOverlay, SyncStatusIndicator
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
│   ├── model/              # MediaItem, DeviceFolder, SyncStatus, TimelineListItem, Album
│   ├── repository/         # Repository Interfaces
│   └── usecase/
│       ├── CalculateTimelineLayoutUseCase.kt
│       ├── SyncTimelineUseCase.kt
│       ├── ScanLocalMediaUseCase.kt
│       └── EnqueueBackupUseCase.kt
│
├── work/                   # WorkManager background workers
│   ├── UploadWorker.kt     # TUS auto-upload execution (Constrained WorkManager / Foreground for manual)
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

| Screen                                   | Core UI Components                                                                                                                                                                                                                                                     | Key Gestures & Interactions                                                                                                                                                                                                                                  |
|:-----------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **`TimelineScreen`**                     | • `LazyColumn` of typed headers + justified rows (cloud items + visible camera folders)<br>• Right-edge `TimelineScrubber` (with `systemGestureExclusion`)<br>• Floating `DateOverlay`<br>• Subtle connectivity/sync indicator<br>• Contextual multi-selection top bar | • Fast vertical dragging on scrubber via `scrollToItem` (with `isScrubbing` image-load gating)<br>• Long-press + drag down to multi-select items<br>• Tap item → navigate into `MediaViewer`<br>• Sync scroll position back to active item on viewer dismiss |
| **`MediaViewerScreen`**                  | • `HorizontalPager` for smooth swiping<br>• `Telephoto` zoomable photo viewport<br>• Reused single `ExoPlayer` instance (`OkHttpDataSource`) attached to settled page<br>• Bottom action bar (Share, Add to Album, Delete, Info)                                       | • Double-tap to zoom, pinch-to-zoom<br>• Swipe down to dismiss back to timeline<br>• Swipe up to expand metadata / EXIF / weather sheet                                                                                                                      |
| **`AlbumsScreen` & `AlbumDetailScreen`** | • `LazyVerticalGrid` (square cards) for album list<br>• Justified row grid for album contents                                                                                                                                                                          | • Tap album card → open album timeline<br>• Add/remove photos to/from albums via selection flow or viewer                                                                                                                                                    |
| **`DeviceFoldersScreen`**                | • "On this device" folder grid (`Screenshots`, `Download`, 3rd-party cameras, etc.)<br>• Per-folder **Back up** & **Show in timeline** toggles<br>• Justified row grid for folder contents                                                                             | • Browse non-camera folders without cluttering main timeline<br>• Toggle backup on for a folder (auto-enables timeline visibility) or manually upload individual items                                                                                       |
| **`BackupSettingsScreen`**               | • Discovered device folder checklist (defaults to `DCIM/*` excluding Screenshots)<br>• Constraint switches (Wi-Fi only, while charging)<br>• Live upload queue progress bar                                                                                            | • Toggle auto-backup globally or per folder<br>• "Sync Now" manual trigger                                                                                                                                                                                   |
| **`AuthScreen`**                         | • Server URL input (defaults to self-hosted domain)<br>• Email & password inputs                                                                                                                                                                                       | • Validates connection via `GET /api/health`<br>• Stores session cookies in persistent `CookieJar` across app launches                                                                                                                                       |

---

### Part 5: Recommended Third-Party Libraries

* **Image Loading:** `io.coil-kt.coil3:coil-compose:3.6.3` & `io.coil-kt.coil3:coil-network-okhttp:3.6.3` (Native
  Compose integration, coroutines, custom `MediaStoreThumbnailFetcher`, shared `OkHttpClient`, and AVIF decoding via
  `ImageDecoder`).
* **Immutable Collections:** `org.jetbrains.kotlinx:kotlinx-collections-immutable:0.5.2` (`ImmutableList` for stable
  Compose row skipping).
* **Zoom Gestures in Viewer:** `me.saket.telephoto:zoomable-image-coil3:0.19.0` (Coil 3 integration for Compose zoomable
  images with fling, inertia, and nested scrolling cooperation with `HorizontalPager`).
* **Video Playback:** `androidx.media3:media3-exoplayer:1.11.1`,
  `androidx.media3:media3-ui-compose-material3:1.11.1` (or `media3-ui:1.11.1`), &
  `androidx.media3:media3-datasource-okhttp:1.11.1` (Required so `ExoPlayer` shares `OkHttpClient` cookies and 401
  refresh).
* **Protobuf Compilation:** `com.google.protobuf:protobuf-kotlin-lite:4.36.2` with the Gradle Protobuf plugin to compile
  the `.proto` schemas.
* **Navigation:** `androidx.navigation:navigation-compose:2.10.2` (Type-safe Compose navigation routes via
  `@Serializable` classes).
* **Local DB:** `androidx.room:room-runtime:2.8.5`, `androidx.room:room-ktx:2.8.5`, & KSP
  `androidx.room:room-compiler:2.8.5` (Configured with SQLite WAL mode for non-blocking concurrent reads/writes).
* **Background Tasks:** `androidx.work:work-runtime-ktx:2.12.0` (WorkManager for background sync and TUS uploads).
