-- Album media item `rank` becomes an optional manual-ordering override:
-- `rank IS NOT NULL` <=> `album.sort_mode = 'none'` (the album has a user-defined order).
-- Date/added-sorted albums derive their display order from data columns
-- (`media_item.sort_timestamp`, `album_media_item.added_at`) at read time.
--
-- Consequences:
-- * Ingest/appends into non-manual albums no longer write rank at all
--   (no more O(album) rank rewrites, no ordering deadlocks under concurrent ingest).
-- * rank is only materialized by explicit user actions (album reorder save).

ALTER TABLE album_media_item ALTER COLUMN rank DROP NOT NULL;
