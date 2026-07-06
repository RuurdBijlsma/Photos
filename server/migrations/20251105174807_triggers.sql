-- Function to broadcast a notification when a row is inserted
CREATE OR REPLACE FUNCTION notify_new_media_item() RETURNS trigger AS
$$
BEGIN
    PERFORM pg_notify('media_item_added', row_to_json(NEW)::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_new_media_item
    AFTER INSERT
    ON media_item
    FOR EACH ROW
EXECUTE FUNCTION notify_new_media_item();


-- =========================================================================================
-- Album Timestamp Triggers (Statement Level)
-- =========================================================================================

CREATE OR REPLACE FUNCTION update_album_timestamps_stmt()
    RETURNS TRIGGER AS
$$
BEGIN
    -- Handle INSERTS
    IF (TG_OP = 'INSERT') THEN
        UPDATE album a
        SET latest_media_item_timestamp   = sub.max_ts,
            earliest_media_item_timestamp = sub.min_ts
        FROM (SELECT nt.album_id, MAX(mi.sort_timestamp) as max_ts, MIN(mi.sort_timestamp) as min_ts
              FROM (SELECT DISTINCT album_id FROM new_table) nt
                       LEFT JOIN album_media_item ami ON ami.album_id = nt.album_id
                       LEFT JOIN media_item mi ON ami.media_item_id = mi.id AND mi.deleted = false
              GROUP BY nt.album_id) sub
        WHERE a.id = sub.album_id;

        -- Handle DELETES
    ELSIF (TG_OP = 'DELETE') THEN
        UPDATE album a
        SET latest_media_item_timestamp   = sub.max_ts,
            earliest_media_item_timestamp = sub.min_ts
        FROM (SELECT ot.album_id, MAX(mi.sort_timestamp) as max_ts, MIN(mi.sort_timestamp) as min_ts
              FROM (SELECT DISTINCT album_id FROM old_table) ot
                       LEFT JOIN album_media_item ami ON ami.album_id = ot.album_id
                       LEFT JOIN media_item mi ON ami.media_item_id = mi.id AND mi.deleted = false
              GROUP BY ot.album_id) sub
        WHERE a.id = sub.album_id;
    END IF;

    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Trigger for INSERTS
CREATE TRIGGER trigger_update_album_timestamp_insert
    AFTER INSERT
    ON album_media_item
    REFERENCING NEW TABLE AS new_table
    FOR EACH STATEMENT
EXECUTE FUNCTION update_album_timestamps_stmt();

-- Trigger for DELETES
CREATE TRIGGER trigger_update_album_timestamp_delete
    AFTER DELETE
    ON album_media_item
    REFERENCING OLD TABLE AS old_table
    FOR EACH STATEMENT
EXECUTE FUNCTION update_album_timestamps_stmt();


-- =========================================================================================
-- Album Media Count Triggers (Statement Level)
-- =========================================================================================

CREATE OR REPLACE FUNCTION update_album_media_count_stmt()
    RETURNS TRIGGER AS
$$
BEGIN
    IF (TG_OP = 'INSERT') THEN
        UPDATE album a
        SET media_count = media_count + sub.cnt
        FROM (
                 -- Count only items being inserted that aren't soft-deleted
                 SELECT nt.album_id, COUNT(*) as cnt
                 FROM new_table nt
                          JOIN media_item mi ON nt.media_item_id = mi.id
                 WHERE mi.deleted = false
                 GROUP BY nt.album_id) sub
        WHERE a.id = sub.album_id;

    ELSIF (TG_OP = 'DELETE') THEN
        UPDATE album a
        SET media_count = media_count - sub.cnt
        FROM (
                 -- Count only items being removed that weren't soft-deleted
                 SELECT ot.album_id, COUNT(*) as cnt
                 FROM old_table ot
                          JOIN media_item mi ON ot.media_item_id = mi.id
                 WHERE mi.deleted = false
                 GROUP BY ot.album_id) sub
        WHERE a.id = sub.album_id;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_album_media_item_count ON album_media_item;

CREATE TRIGGER trg_album_media_item_count_insert
    AFTER INSERT
    ON album_media_item
    REFERENCING NEW TABLE AS new_table
    FOR EACH STATEMENT
EXECUTE FUNCTION update_album_media_count_stmt();

CREATE TRIGGER trg_album_media_item_count_delete
    AFTER DELETE
    ON album_media_item
    REFERENCING OLD TABLE AS old_table
    FOR EACH STATEMENT
EXECUTE FUNCTION update_album_media_count_stmt();


-- =========================================================================================
-- Hard-delete synchronization functions
-- =========================================================================================

-- Combined hard-delete synchronization function
CREATE OR REPLACE FUNCTION fn_trigger_media_item_hard_delete_sync()
    RETURNS TRIGGER AS
$$
BEGIN
    -- 1. Clear thumbnail references in the 'album' table to prevent foreign key errors during cascaded updates.
    UPDATE album
    SET thumbnail_id = NULL
    WHERE thumbnail_id = OLD.id;

    -- 2. Adjust the media counts if the deleted item was not soft-deleted.
    IF (OLD.deleted = false) THEN
        UPDATE album
        SET media_count = media_count - 1
        WHERE id IN (SELECT album_id
                     FROM album_media_item
                     WHERE media_item_id = OLD.id);
    END IF;

    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_media_item_hard_delete
    BEFORE DELETE
    ON media_item
    FOR EACH ROW
EXECUTE FUNCTION fn_trigger_media_item_hard_delete_sync();


-- =========================================================================================
-- Soft-delete synchronization functions (Row and Statement levels)
-- =========================================================================================

-- Batch update visual_analysis's deleted status
CREATE OR REPLACE FUNCTION fn_trigger_media_item_soft_delete_sync_stmt()
    RETURNS TRIGGER AS
$$
BEGIN
    UPDATE visual_analysis va
    SET deleted = nt.deleted
    FROM new_table nt
             JOIN old_table ot ON nt.id = ot.id
    WHERE va.media_item_id = nt.id
      AND nt.deleted IS DISTINCT FROM ot.deleted;

    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_media_item_soft_delete ON media_item;

-- Bind the statement-level trigger to media_item
CREATE TRIGGER trg_media_item_soft_delete
    AFTER UPDATE
    ON media_item
    REFERENCING NEW TABLE AS new_table OLD TABLE AS old_table
    FOR EACH STATEMENT
EXECUTE FUNCTION fn_trigger_media_item_soft_delete_sync_stmt();

-- Statement-level trigger function to batch update album statistics on media_item soft-delete
CREATE OR REPLACE FUNCTION update_album_stats_on_media_item_soft_delete_stmt()
    RETURNS TRIGGER AS
$$
BEGIN
    -- Identify the distinct albums containing any changed media items,
    -- and recalculate their statistics in a single batch operation.
    UPDATE album a
    SET latest_media_item_timestamp   = sub.max_ts,
        earliest_media_item_timestamp = sub.min_ts,
        media_count                   = sub.cnt
    FROM (
             SELECT
                 ami_all.album_id,
                 MAX(mi.sort_timestamp) as max_ts,
                 MIN(mi.sort_timestamp) as min_ts,
                 COUNT(mi.id) as cnt
             FROM album_media_item ami_all
                      LEFT JOIN media_item mi ON ami_all.media_item_id = mi.id AND mi.deleted = false
             WHERE ami_all.album_id IN (
                 SELECT DISTINCT ami.album_id
                 FROM album_media_item ami
                          JOIN new_table nt ON ami.media_item_id = nt.id
                          JOIN old_table ot ON nt.id = ot.id
                 WHERE nt.deleted IS DISTINCT FROM ot.deleted
             )
             GROUP BY ami_all.album_id
         ) sub
    WHERE a.id = sub.album_id;

    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Bind the statement-level trigger to media_item to ensure performance during bulk deletes
CREATE TRIGGER trg_media_item_album_sync_update
    AFTER UPDATE
    ON media_item
    REFERENCING NEW TABLE AS new_table OLD TABLE AS old_table
    FOR EACH STATEMENT
EXECUTE FUNCTION update_album_stats_on_media_item_soft_delete_stmt();