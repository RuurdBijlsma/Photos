-- Google Takeout import support.
--
-- takeout_id on album marks albums created by a takeout import (batch id parsed
-- from the zip filename, or the literal "takeout" as fallback), so a re-import of
-- the same export reuses its albums while a different export or a regular album
-- with the same name is never merged into it.

ALTER TYPE job_type ADD VALUE IF NOT EXISTS 'takeout_import' AFTER 'handle_web_upload';

ALTER TABLE album
    ADD COLUMN takeout_id TEXT NULL;

CREATE INDEX idx_album_takeout
    ON album (owner_id, takeout_id, name);

-- One row per (album, filename) occurrence found in a takeout export.
-- While `media_item_id IS NULL` the ingest of the (single) kept copy of that
-- filename has not happened yet; IngestMetadata consumes these rows in the
-- same transaction that inserts the media item, so album membership is
-- attached atomically no matter which zip arrived or was processed first.
CREATE TABLE takeout_album_link
(
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id       INTEGER     NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    batch_id      TEXT        NOT NULL,
    album_id      VARCHAR(10) NOT NULL REFERENCES album (id) ON DELETE CASCADE,
    filename      TEXT        NOT NULL,
    media_item_id VARCHAR(10) NULL REFERENCES media_item (id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX uq_takeout_album_link
    ON takeout_album_link (user_id, album_id, filename);

-- Hot path: ingest consumes pending links by (user_id, filename).
CREATE INDEX idx_takeout_link_pending
    ON takeout_album_link (user_id, filename)
    WHERE media_item_id IS NULL;

-- Filename-based dedup lookups during takeout registration.
CREATE INDEX idx_media_item_user_filename
    ON media_item (user_id, filename);
