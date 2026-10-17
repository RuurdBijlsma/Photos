# Implementation plan: make `rank` a manual-order override, not a maintained replica

Target branch: `develop`.

## Problem

`album_media_item.rank` is treated as the live source of truth for album display
order and is kept in sync with `sort_mode` on **every insert**:

- `ingest_metadata.rs` → `AlbumStore::sort_media_items` (full-album rewrite per ingested file)
- `service.rs::add_media_to_album` → `AlbumStore::sort_media_items`
- `service.rs::create_album` → `AlbumStore::sort_media_items`

Cost: O(album) per ingested file, and concurrent ingests into the same album can
deadlock on the rank rewrites (the root of the deadlock that forced `defer_sort`
+ `TakeoutFinalize` on the takeout branch — killed there, and a 10k+ takeout
album is O(10k × album) row rewrites without it).

## Key insight from the current code

- `get_album_media` (album detail hydration, `service.rs:543`) hardcodes
  `AlbumSort::None`, so the UI **always renders rank order**. That is the only
  reason rank must stay date-ordered for date albums.
- The reorder UX (`AlbumView.vue`) already materializes the full list on save
  (drag → send all IDs → `reorder_media_items` renumbers 1..N). So manual order
  already exists as a "user action materializes rank" flow.

Therefore: make display order mode-aware at query time, and let `rank` exist
**only when the album has a manual ordering** (`sort_mode = none`).

Invariant after the change:

> **`rank` is populated ⇔ `album.sort_mode = 'none'`. Everything else derives its
> order from data columns (`sort_timestamp`, `added_at`) at read time.**

`sort_timestamp` is `NOT NULL` (init migration) — no NULL handling needed in the
ORDER BY, only for `rank`.

## 1. Schema migration

```sql
-- rank becomes optional (NULL = no manual position)
ALTER TABLE album_media_item ALTER COLUMN rank DROP NOT NULL;

-- Backfill: albums not in manual mode lose their maintained ranks
UPDATE album_media_item ami
SET rank = NULL
FROM album a
WHERE a.id = ami.album_id
  AND a.sort_mode <> 'none';
```

Notes:

- The existing `UNIQUE (album_id, rank) DEFERRABLE INITIALLY DEFERRED` can stay.
  Postgres unique constraints treat NULLs as distinct by default, so many NULL
  ranks are fine while duplicate *manual* positions are still rejected.
- `idx_album_media_item_rank (album_id, rank)` stays; apps that filter on rank
  keep working.
- Triggers (`media_count`, earliest/latest timestamps) don't read rank — untouched.

## 2. Store changes (`album_store.rs`)

### 2a. `add_media_items` → pure append, NULL rank for non-manual albums

Single statement, O(new items), never rewrites existing rows:

```sql
INSERT INTO album_media_item (album_id, media_item_id, rank, added_by_user)
SELECT
    $1::TEXT,
    items.id,
    CASE
        WHEN (SELECT sort_mode = 'none' FROM album WHERE id = $1) THEN
            COALESCE((SELECT MAX(rank) FROM album_media_item WHERE album_id = $1), 0.0)
            + (ROW_NUMBER() OVER (ORDER BY mi.sort_timestamp ASC) * 1000.0)
        ELSE NULL
    END,
    $2
FROM UNNEST($3::TEXT[]) AS items(id)
JOIN media_item mi ON mi.id = items.id
ON CONFLICT (album_id, media_item_id) DO NOTHING
```

Manual-mode albums: new items append after `MAX(rank)` (matches expectations —
"new photo lands at the end, I can drag it"). The `ORDER BY sort_timestamp`
inside the append keeps bulk appends date-consistent.

### 2b. `list_sorted_media_items` — mode-aware, rank optional

Already mode-aware; harden the manual branch for NULL ranks:

```sql
ORDER BY
    CASE WHEN $2 = 'date_asc'  THEN mi.sort_timestamp END ASC,
    CASE WHEN $2 = 'date_desc' THEN mi.sort_timestamp END DESC,
    CASE WHEN $2 = 'added_asc' THEN ami.added_at      END ASC,
    CASE WHEN $2 = 'added_desc' THEN ami.added_at     END DESC,
    CASE WHEN $2 = 'none'      THEN ami.rank          END ASC,  -- NULLs sort last
    ami.added_at ASC,
    ami.media_item_id ASC
```

`CASE ... END ASC` naturally puts NULL ranks last, and the new deterministic
tiebreaks (`added_at`, id) govern items without a manual position.

### 2c. `list_media_items` — accept and respect the album's sort mode

Currently `ORDER BY rank ASC` unconditionally (used by the album contents
endpoint and integration tests). Add a `sort_mode: AlbumSort` parameter and use
the same ORDER BY as 2b. Callers pass `album.sort_mode` (already loaded, or
loaded in the same call).

### 2d. `find_middle_media_item_id` — same ordering rule

It scans `ORDER BY rank` to pick a representative thumbnail. For non-manual
albums that becomes stale/arbitrary. Switch it to the shared ordering rule
(e.g. reuse 2b's ORDER BY semantics with `added_at DESC` as the "representative"
choice, or pass sort_mode). Low stakes, but keep it consistent.

### 2e. `sort_media_items` — keep, becomes user-action-only

No hot-path callers remain; the function stays for a future "re-apply sort mode"
button. Consider documenting it as O(album)-by-design and take the per-album
advisory lock inside it (it rewrites every row).

## 3. Service changes (`api/album/service.rs`)

| Location | Change |
|---|---|
| `create_album` (:201) | Remove the trailing `sort_media_items`. Fresh album: ranks all NULL, renders DateAsc from data. |
| `add_media_to_album` (:229–234) | Remove the conditional full sort. Append handles both cases. Keep the thumbnail-init and one `find_by_id`. |
| `get_album_media` (:543) | Use the **album's real mode**: `list_sorted_media_items(pool, album_id, album.sort_mode)` instead of hardcoded `AlbumSort::None`. |
| `reorder_media_items` (:639) | On save: if `sort_mode != none`, additionally `UPDATE ... SET rank = NULL` for the album (enforces the new invariant: ranks cleared when leaving manual mode). If `sort_mode = none` keep the existing renumber of the provided list. |
| `move_album_item` (:603) | Dead code (no route, no caller). Delete, or leave untouched but flagged. |

## 4. Worker changes

| Location | Change |
|---|---|
| `ingest_metadata.rs` (store_media_item) | Drop the `find_by_id` + `sort_media_items` block entirely. Keep the album link `add_media_items` and the thumbnail init. Since `sort_timestamp` is NOT NULL, ingest no longer touches shared rank rows at all → ingest path becomes deadlock-free by construction. |
| `import_album_item.rs` | No change (it just calls `add_media_items` via `enqueue_full_ingest`). |

## 5. Behavior semantics after the change

- **New album (DateAsc default):** all ranks NULL; view orders by `sort_timestamp`.
- **Ingest into date album:** append, rank NULL. Display order is immediate and correct. Zero shared-row contention.
- **User drags (manual reorder, mode → none):** ranks materialized 1..N·1000 at save. New ingests append at `MAX(rank) + delta` (end of album).
- **User saves a date sort (mode ≠ none):** ranks cleared; view derives from date columns. The previous rank values are gone by design — "manual order" is durable only while the album is in manual mode, which matches the current UI (saveReorder sends the mode you chose).
- **Mode shows progress:** `getSortedAlbumMedia` preview (`fetchSortedPreview`) already orders by data columns, unaffected.

## 6. Concurrency notes

- Ingest appends no longer rewrite other rows → the per-insert deadlock class disappears; no advisory locks needed in the ingest path.
- Two concurrent appends into a **manual** album can both read the same `MAX(rank)` and produce duplicate values → the existing deferrable unique constraint fails one of the transactions at commit. Acceptable: rare, and the job system retries (labels order ambiguous between those two items only until the next drag-save). If it ever bites, serialize None-mode appends with the per-album advisory lock — cheap now that the hot path doesn't re-sort.
- Full-album reorder (user action) concurrent with an ingest append: reorder renumbers the items it knows about; a concurrently appended item may end up with a higher rank (stays at the end — correct) or a duplicate (unique constraint bites, one retries). Optionally take the same advisory lock in `reorder_media_items`'s tx.

## 7. Testing plan (extend `test_album.rs` + worker tests)

1. Ingest into a DateAsc album → `rank IS NULL` for the new row; `get_album_media` order == `sort_timestamp` order.
2. Ingest into a manual (`none`) album → rank = `MAX(rank) + 1000`, appears at end of detail view.
3. Reorder mode → save with `date_asc` → ranks NULLed, view still date order.
4. Reorder drag → save with `none` → full list order preserved exactly (regression guard for the existing flow).
5. Re-adding an existing item (ON CONFLICT) is idempotent.
6. Two items with equal sort_timestamp → deterministic order (added_at, id tiebreak).
7. Parallel workers appending to the same album → no deadlock, at most one retry on duplicate rank.

## 8. What this unblocks

The takeout pipeline no longer needs its compensating machinery. A fresh branch
on top of this can:

- enqueue plain `IngestMetadata` jobs without ` defer_sort`/album payloads,
- link album memberships with pure appends at processing time,
- do album creation + batch linking once after the drain, with no sort pass —
  `TakeoutFinalize`'s only real job (restoring rank order) no longer exists.

Killing the O(album)-per-file rewrite was the reason both the advisory locks and
the finalize job had to exist.

## Pros & cons vs. current implementation

**Pros**

- Ingest is O(new item) instead of O(album): a 20k-photo takeout album goes from
  ~20k full-album rewrites to 20k single-row appends.
- Ingest path can no longer deadlock on album ordering (no shared rank rows rewritten) — removes the class of bug that produced the takeout branch's `defer_sort` + `TakeoutFinalize`.
- Album detail view reflects reality immediately; no "sort catches up later".
- Simpler mental model, documented invariant: `rank IS NULL ⇔ not manually placed`.
- The reorder UX, API shape, frontend, and triggers stay exactly as today.
- Fewer DB round trips in ingest (one `find_by_id` instead of two, no full reload per album link).

**Cons / trade-offs**

- Display order becomes read-time derived for date albums: each album-view request sorts the album's rows (join + ORDER BY) instead of scanning the maintained `(album_id, rank)` index. For a 30k-item album without pagination this is a real per-request sort. Mitigations if it ever shows in profiling: denormalize a snapshot column (e.g. `ami.sort_epoch`) or cache; not needed day one.
- Rank values for date albums are no longer meaningful (NULL). Anything that assumed `rank` reflects chronology must switch (covered: `find_middle_media_item_id`, tests). Dead code `move_album_item` assumed fractional insertion — it's unused.
- Concurrent appends into a *manual* album can duplicate ranks (unique-violation retry). Pre-existing race in the current design too, so no regression — but the failure is now louder instead of silently fixed by the next full sort.
- Data migration introduces NULLs into a previously NOT NULL column: every query that orders by rank must consciously decide `NULLS LAST` (this plan covers all known ones — 2b/2c/2d).
- Two kinds of ordering coexist (data-derived vs materialized). Document the invariant at the type level if possible (e.g. a comment + store-level helper that asserts `sort_mode = 'none'` when writing ranks).

## 9. Benchmark: view-ordering cost on the real 16k album (dev DB)

Album `AqB4BXIrMI` ("Lorge"), 15,833 items, `EXPLAIN (ANALYZE, BUFFERS, TIMING OFF)`,
interleaved A/B rounds in one session so both see identical cache conditions:

| Variant | Plan | Exec time (3 rounds) |
|---|---|---|
| A — current CASE ordering (`AlbumSort::None`) | seq scans → hash join → **quicksort 15.8k rows** | 75.2 / 66.0 / 65.6 ms |
| B — proposed direct `sort_timestamp DESC/ASC` | same shape, sort key = data column | 63.5 / 67.7 / 65.8 ms |
| C — bare `ORDER BY rank` (reference) | identical: hash join + quicksort | ~14 ms (quiet-machine batch) |
| C2 — forced index-ordered nested loop on `idx_album_media_item_rank` | rank index scan + 15,833 PK probes + Memoize | **235 ms** |

Conclusions:

- **Parity confirmed**: A ≈ B within noise. The feared regression (index scan →
  query-time sort) does not exist, because the current query never had the
  index ordering: the CASE-with-parameter ORDER BY cannot use
  `idx_album_media_item_rank`, and even a bare `ORDER BY rank` (C) still yields
  hash join + quicksort at this size — the planner sorts 15.8k rows on *every
  request today*.
- **A maintained rank index would be slower anyway** (C2): 47.7k buffer
  accesses from 15,833 random PK probes cost ~4x more than the ~1.7MB
  in-memory quicksort. "Index beats sort" is false at this scale when the
  ordering column lives on the joined table.
- **Planning stability bonus**: the CASE plan showed planning spikes
  (2.2 / 0.4 / 10.7 ms) vs a stable ~0.4 ms for the direct ORDER BY.
- The measured DB time (~14–65 ms, environment-dependent) is only a slice of
  the observed ~190 ms endpoint latency; the rest is serialization of 15.8k
  protobuf items + the other queries + transfer. That overhead is orthogonal
  to this plan; the only meaningful lever there is pagination/caching.
- The snapshot-column escape hatch (§8 mitigation) is only worth revisiting
  for albums an order of magnitude larger than today's worst case.
