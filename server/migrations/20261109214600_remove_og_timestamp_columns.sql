-- Remove original timestamp tracking columns now that media files are updated directly in EXIF
ALTER TABLE media_item
    DROP COLUMN og_taken_at_local,
    DROP COLUMN og_timezone_offset_seconds;
