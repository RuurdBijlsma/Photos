ALTER TABLE media_item
    ADD COLUMN thumbnail_repair_blocked BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN analysis_repair_blocked  BOOLEAN NOT NULL DEFAULT false;
