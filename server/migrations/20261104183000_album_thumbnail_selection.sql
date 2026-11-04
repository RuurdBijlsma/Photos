ALTER TABLE album
    ADD COLUMN thumbnail_is_user_selected BOOLEAN NOT NULL DEFAULT false;

CREATE OR REPLACE FUNCTION clear_album_thumbnail_user_selection_on_null()
    RETURNS TRIGGER AS
$$
BEGIN
    IF NEW.thumbnail_id IS NULL THEN
        NEW.thumbnail_is_user_selected := false;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_clear_album_thumbnail_user_selection_on_null
    BEFORE UPDATE OF thumbnail_id
    ON album
    FOR EACH ROW
EXECUTE FUNCTION clear_album_thumbnail_user_selection_on_null();
