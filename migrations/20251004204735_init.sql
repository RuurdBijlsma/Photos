CREATE TYPE user_role AS ENUM ('admin', 'user');

CREATE TABLE location
(
    id           SERIAL PRIMARY KEY,
    name         TEXT NOT NULL,
    admin1       TEXT NOT NULL,
    admin2       TEXT NOT NULL,
    country_code TEXT NOT NULL,
    country_name TEXT NOT NULL
);
CREATE INDEX idx_location_lookup ON location (name, admin1, country_code);

CREATE TABLE app_user
(
    id           SERIAL PRIMARY KEY,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    email        TEXT        NOT NULL UNIQUE,
    password     TEXT        NOT NULL,
    name         TEXT        NOT NULL,
    media_folder TEXT,
    role         user_role   NOT NULL DEFAULT 'user'
);

CREATE TABLE user_invite
(
    token        TEXT PRIMARY KEY,
    media_folder TEXT        NOT NULL,
    expires_at   TIMESTAMPTZ NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE remote_user
(
    id         SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    identity   TEXT        NOT NULL UNIQUE,
    name       TEXT,
    user_id    INT         NOT NULL REFERENCES app_user (id) ON DELETE CASCADE
);
CREATE INDEX idx_remote_user_user_id ON remote_user (user_id);

-- Create the Refresh Token table for persistent user sessions.
CREATE TABLE refresh_token
(
    id            SERIAL PRIMARY KEY,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    user_id       INTEGER     NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    selector      TEXT        NOT NULL UNIQUE,
    verifier_hash TEXT        NOT NULL,
    expires_at    TIMESTAMPTZ NOT NULL
);

-- Create the central MediaItem table.
-- Other tables with specific metadata will link to this one.
CREATE TABLE media_item
(
    -- Ids
    id                         VARCHAR(10) PRIMARY KEY,
    user_id                    INT         NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    remote_user_id             INT         REFERENCES remote_user (id) ON DELETE SET NULL,
    hash                       TEXT        NOT NULL,
    -- File info
    relative_path              TEXT        NOT NULL UNIQUE,
    filename                   TEXT        NOT NULL,
    created_at                 TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                 TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Media info
    width                      INT         NOT NULL,
    height                     INT         NOT NULL,
    is_video                   BOOLEAN     NOT NULL,
    duration_ms                BIGINT,
    orientation                INT         NOT NULL DEFAULT 1,
    -- Time
    taken_at_local             TIMESTAMP   NOT NULL,
    og_taken_at_local          TIMESTAMP   NOT NULL,
    taken_at_utc               TIMESTAMPTZ,
    sort_timestamp             TIMESTAMPTZ NOT NULL,
    timezone_name              TEXT,
    timezone_offset_seconds    INT,
    og_timezone_offset_seconds INT,
    -- Frontend fields
    use_panorama_viewer        BOOLEAN     NOT NULL,
    has_thumbnails             BOOLEAN     NOT NULL DEFAULT false,
    month_id                   DATE GENERATED ALWAYS AS (date_trunc('month', taken_at_local)) STORED,
    user_caption               TEXT,
    -- Other
    deleted                    BOOLEAN     NOT NULL DEFAULT false,
    search_vector              TSVECTOR,
    CONSTRAINT width_positive CHECK (width > 0),
    CONSTRAINT height_positive CHECK (height > 0)
);

ALTER TABLE app_user
    ADD COLUMN avatar_id VARCHAR(10) REFERENCES media_item (id) ON DELETE SET NULL;

-- The following tables store optional, detailed metadata for a MediaItem.
-- They use a one-to-one relationship where the primary key is also a foreign key
-- referencing media_item.id. This keeps the main media_item table clean.
CREATE TABLE gps
(
    media_item_id     VARCHAR(10) PRIMARY KEY REFERENCES media_item (id) ON DELETE CASCADE,
    location_id       INT REFERENCES location (id), -- A media item can exist without a resolved location.
    latitude          DOUBLE PRECISION NOT NULL,
    longitude         DOUBLE PRECISION NOT NULL,
    altitude          DOUBLE PRECISION,
    compass_direction DOUBLE PRECISION
);

CREATE TABLE panorama_config
(
    media_item_id VARCHAR(10) PRIMARY KEY REFERENCES media_item (id) ON DELETE CASCADE,
    config        JSONB NOT NULL
);

CREATE TABLE time
(
    media_item_id     VARCHAR(10) PRIMARY KEY REFERENCES media_item (id) ON DELETE CASCADE,
    timezone_source   TEXT,
    source_details    TEXT NOT NULL,
    source_confidence TEXT NOT NULL
);

CREATE TABLE weather
(
    media_item_id     VARCHAR(10) PRIMARY KEY REFERENCES media_item (id) ON DELETE CASCADE,
    temperature       REAL,
    dew_point         REAL,
    relative_humidity INT,
    precipitation     REAL,
    snow              INT,
    wind_direction    INT,
    wind_speed        REAL,
    peak_wind_gust    REAL,
    pressure          REAL,
    sunshine_minutes  INT,
    condition         TEXT,
    sunrise           TIMESTAMPTZ,
    sunset            TIMESTAMPTZ,
    dawn              TIMESTAMPTZ,
    dusk              TIMESTAMPTZ,
    is_daytime        BOOLEAN
);

CREATE TABLE media_features
(
    media_item_id                       VARCHAR(10) PRIMARY KEY REFERENCES media_item (id) ON DELETE CASCADE,
    mime_type                           TEXT    NOT NULL,
    size_bytes                          BIGINT  NOT NULL,
    is_motion_photo                     BOOLEAN NOT NULL,
    motion_photo_presentation_timestamp BIGINT,
    is_hdr                              BOOLEAN NOT NULL,
    is_burst                            BOOLEAN NOT NULL,
    burst_id                            TEXT,
    capture_fps                         REAL,
    video_fps                           REAL,
    is_nightsight                       BOOLEAN NOT NULL,
    is_timelapse                        BOOLEAN NOT NULL,
    exif                                JSONB   NOT NULL,
    audio_format                        TEXT,
    audio_channels                      INT,
    audio_sample_rate                   INT,
    compressor_id                       TEXT
);

CREATE TABLE camera_settings
(
    media_item_id         VARCHAR(10) PRIMARY KEY REFERENCES media_item (id) ON DELETE CASCADE,
    iso                   INT,
    exposure_time         REAL,
    aperture              REAL,
    focal_length          REAL,
    focal_length_in_35mm  REAL,
    camera_make           TEXT,
    camera_model          TEXT,
    flash_fired           BOOLEAN,
    flash_mode            TEXT,
    lens_make             TEXT,
    lens_model            TEXT,
    digital_zoom_ratio    REAL,
    subject_distance      REAL,
    exposure_compensation REAL
);

-- Create indices for foreign keys and frequently queried columns for performance.

-- Full text search index:
CREATE INDEX idx_media_item_search ON media_item USING GIN (search_vector);

-- Index for the foreign key in the gps table.
CREATE INDEX idx_gps_location_id ON gps (location_id);

-- Indices for common sorting/filtering operations on media_item.
CREATE INDEX idx_media_item_created_at ON media_item (created_at);
CREATE INDEX idx_media_item_taken_at_local ON media_item (taken_at_local);
CREATE INDEX idx_media_item_user_id ON media_item (user_id);
CREATE INDEX idx_media_item_user_hash ON media_item (user_id, hash);
CREATE INDEX idx_media_item_not_deleted ON media_item (id) WHERE deleted = false;
CREATE INDEX idx_media_item_search_lookup ON media_item (user_id, deleted, id);

-- For /timeline/ids
CREATE INDEX idx_media_item_ids_timeline
    ON media_item (user_id, sort_timestamp DESC) INCLUDE (id)
    WHERE deleted = false;

-- For /timeline/ratios
CREATE INDEX idx_media_item_user_month_order_partial
    ON media_item (
                   user_id,
                   month_id,
                   sort_timestamp DESC
        ) INCLUDE (width, height)
    WHERE deleted = false;

-- Composite index for search filtering
CREATE INDEX idx_media_item_search_filters
    ON media_item (user_id, deleted, is_video) INCLUDE (id, taken_at_utc, sort_timestamp);

-- For /photos/geo (Map view)
CREATE INDEX idx_media_item_geo_lookup
    ON media_item (user_id, deleted, sort_timestamp DESC) INCLUDE (id, width, height, is_video, has_thumbnails, duration_ms);

-- For /camera
CREATE INDEX idx_camera_settings_normalized_make_model
    ON camera_settings (
                        LOWER(TRIM(camera_make)),
                        LOWER(REGEXP_REPLACE(TRIM(camera_model), '\s*\(.*\)$', '', 'i'))
        );