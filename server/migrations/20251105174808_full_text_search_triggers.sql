-- Create the function that performs the update for a specific ID
CREATE OR REPLACE FUNCTION rebuild_media_item_search_vector(target_id VARCHAR(10))
    RETURNS VOID AS
$$
BEGIN
    UPDATE media_item
    SET search_vector = (SELECT
                             -- HIGH CONFIDENCE ('A')
                             setweight(to_tsvector('english', coalesce(l.name, '')), 'A') ||
                             setweight(to_tsvector('english', coalesce(l.admin1, '')), 'A') ||
                             setweight(to_tsvector('english', coalesce(l.country_name, '')), 'A') ||
                             setweight(to_tsvector('english', coalesce(p_agg.names, '')), 'A') ||
                                 -- MEDIUM CONFIDENCE ('B')
                             setweight(to_tsvector('english', mi.filename), 'B') ||
                             setweight(to_tsvector('english', coalesce(l.admin2, '')), 'B') ||
                             setweight(to_tsvector('english', coalesce(w.condition, '')), 'B') ||
                                 -- LOWER CONFIDENCE ('C' & 'D')
                             setweight(to_tsvector('english', to_char(mi.taken_at_local, 'YYYY Month Day')), 'C') ||
                             setweight(to_tsvector('english', CASE WHEN mi.is_video THEN 'video' ELSE 'photo' END), 'D')
                         FROM media_item mi
                                  LEFT JOIN gps g ON mi.id = g.media_item_id
                                  LEFT JOIN location l ON g.location_id = l.id
                                  LEFT JOIN weather w ON mi.id = w.media_item_id

                             -- Faces aggregation
                                  LEFT JOIN (SELECT va_inner.media_item_id, string_agg(DISTINCT pers.name, ' ') as names
                                             FROM face f
                                                      JOIN face_cluster fc ON f.face_cluster_id = fc.id
                                                      JOIN person pers ON fc.person_id = pers.id
                                                      JOIN visual_analysis va_inner ON f.visual_analysis_id = va_inner.id
                                             GROUP BY va_inner.media_item_id) p_agg ON mi.id = p_agg.media_item_id
                         WHERE mi.id = target_id)
    WHERE id = target_id;
END;
$$
    LANGUAGE plpgsql;

-- Trigger function for row-level child tables (media_item, gps, weather, visual_analysis)
CREATE OR REPLACE FUNCTION tg_rebuild_search_vector()
    RETURNS TRIGGER AS
$$
BEGIN
    -- Skip rebuilding if we are only syncing the 'deleted' flag on visual_analysis
    IF TG_OP = 'UPDATE' AND TG_TABLE_NAME = 'visual_analysis' THEN
        IF OLD.deleted IS DISTINCT FROM NEW.deleted THEN
            RETURN NEW;
        END IF;
    END IF;

    IF TG_TABLE_NAME = 'media_item' THEN
        PERFORM rebuild_media_item_search_vector(NEW.id);
    ELSIF TG_TABLE_NAME = 'gps' OR TG_TABLE_NAME = 'weather' OR TG_TABLE_NAME = 'visual_analysis' THEN
        PERFORM rebuild_media_item_search_vector(NEW.media_item_id);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Row-level triggers on single-row tables
CREATE TRIGGER trg_mi_search_update
    AFTER INSERT OR UPDATE OF relative_path, taken_at_local
    ON media_item
    FOR EACH ROW
EXECUTE FUNCTION tg_rebuild_search_vector();

CREATE TRIGGER trg_gps_search_update
    AFTER INSERT OR UPDATE
    ON gps
    FOR EACH ROW
EXECUTE FUNCTION tg_rebuild_search_vector();

CREATE TRIGGER trg_weather_search_update
    AFTER INSERT OR UPDATE
    ON weather
    FOR EACH ROW
EXECUTE FUNCTION tg_rebuild_search_vector();

CREATE TRIGGER trg_va_search_update
    AFTER INSERT OR UPDATE
    ON visual_analysis
    FOR EACH ROW
EXECUTE FUNCTION tg_rebuild_search_vector();


-- =========================================================================================
-- Face Clustering Triggers (Statement Level)
-- =========================================================================================

-- Dedicated statement-level function for face_cluster updates
CREATE OR REPLACE FUNCTION tg_face_cluster_update_stmt()
    RETURNS TRIGGER AS
$$
BEGIN
    PERFORM rebuild_media_item_search_vector(va.media_item_id)
    FROM (SELECT DISTINCT nt.visual_analysis_id
          FROM new_table nt
                   JOIN old_table ot ON nt.id = ot.id
          WHERE nt.face_cluster_id IS DISTINCT FROM ot.face_cluster_id) diff
             JOIN visual_analysis va ON diff.visual_analysis_id = va.id;

    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_face_cluster_update_stmt
    AFTER UPDATE
    ON face
    REFERENCING NEW TABLE AS new_table OLD TABLE AS old_table
    FOR EACH STATEMENT
EXECUTE FUNCTION tg_face_cluster_update_stmt();


-- Dedicated statement-level function for face inserts (only queries new_table)
CREATE OR REPLACE FUNCTION tg_face_insert_stmt()
    RETURNS TRIGGER AS
$$
BEGIN
    PERFORM rebuild_media_item_search_vector(va.media_item_id)
    FROM (SELECT DISTINCT visual_analysis_id
          FROM new_table
          WHERE face_cluster_id IS NOT NULL) diff
             JOIN visual_analysis va ON diff.visual_analysis_id = va.id;

    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_face_insert_stmt
    AFTER INSERT
    ON face
    REFERENCING NEW TABLE AS new_table
    FOR EACH STATEMENT
EXECUTE FUNCTION tg_face_insert_stmt();


-- =========================================================================================
-- Person Rename Trigger
-- =========================================================================================

CREATE OR REPLACE FUNCTION tg_person_rename_search_update()
    RETURNS TRIGGER AS
$$
BEGIN
    IF OLD.name IS DISTINCT FROM NEW.name THEN
        PERFORM rebuild_media_item_search_vector(va.media_item_id)
        FROM face f
                 JOIN face_cluster fc ON f.face_cluster_id = fc.id
                 JOIN visual_analysis va ON f.visual_analysis_id = va.id
        WHERE fc.person_id = NEW.id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_person_search_update
    AFTER UPDATE OF name
    ON person
    FOR EACH ROW
EXECUTE FUNCTION tg_person_rename_search_update();