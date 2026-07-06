ALTER
    ROLE CURRENT_USER SET random_page_cost = 1.1;

-- A record for a single visual analysis run.
CREATE TABLE visual_analysis
(
    id            BIGSERIAL PRIMARY KEY,
    user_id       INT         NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    deleted       BOOLEAN     NOT NULL DEFAULT false, -- Trigger keeps this in sync with media_item.deleted
    media_item_id VARCHAR(10) NOT NULL REFERENCES media_item (id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    embedding     VECTOR(768) NOT NULL,
    percentage    INT         NOT NULL
);
ALTER TABLE visual_analysis
    ALTER
        COLUMN embedding SET STORAGE MAIN;
CREATE INDEX idx_visual_analysis_media_item_id ON visual_analysis (media_item_id);
CREATE INDEX idx_visual_analysis_embedding_hnsw
    ON visual_analysis
        USING hnsw (embedding vector_cosine_ops)
    WHERE (deleted = false);

CREATE TABLE face
(
    id                 BIGSERIAL PRIMARY KEY,
    visual_analysis_id BIGINT      NOT NULL REFERENCES visual_analysis (id) ON DELETE CASCADE,
    position_x         REAL        NOT NULL,
    position_y         REAL        NOT NULL,
    width              REAL        NOT NULL,
    height             REAL        NOT NULL,
    confidence         REAL        NOT NULL,
    age                INT         NOT NULL,
    sex                VARCHAR(10) NOT NULL,
    embedding          VECTOR(512) NOT NULL,
    face_cluster_id    VARCHAR(10) REFERENCES face_cluster (id) ON DELETE SET NULL
);
CREATE INDEX idx_face_visual_analysis_id ON face (visual_analysis_id);
CREATE INDEX ON face USING hnsw (embedding vector_cosine_ops);
CREATE INDEX idx_face_cluster_id ON face (face_cluster_id);


CREATE TABLE object
(
    id                 BIGSERIAL PRIMARY KEY,
    visual_analysis_id BIGINT NOT NULL REFERENCES visual_analysis (id) ON DELETE CASCADE,
    position_x         REAL   NOT NULL,
    position_y         REAL   NOT NULL,
    width              REAL   NOT NULL,
    height             REAL   NOT NULL,
    confidence         REAL   NOT NULL,
    tag                TEXT   NOT NULL
);
CREATE INDEX idx_object_visual_analysis_id ON object (visual_analysis_id);

-- Stores image quality metrics.
CREATE TABLE measured_quality
(
    visual_analysis_id BIGINT PRIMARY KEY REFERENCES visual_analysis (id) ON DELETE CASCADE,
    blurriness         DOUBLE PRECISION NOT NULL,
    noisiness          DOUBLE PRECISION NOT NULL,
    exposure           DOUBLE PRECISION NOT NULL,
    accidentalness     DOUBLE PRECISION NOT NULL,
    weighted_score     DOUBLE PRECISION NOT NULL
);

CREATE TABLE quality_judge
(
    visual_analysis_id BIGINT PRIMARY KEY REFERENCES visual_analysis (id) ON DELETE CASCADE,
    exposure           SMALLINT         NOT NULL,
    contrast           SMALLINT         NOT NULL,
    sharpness          SMALLINT         NOT NULL,
    color_accuracy     SMALLINT         NOT NULL,
    composition        SMALLINT         NOT NULL,
    subject_clarity    SMALLINT         NOT NULL,
    visual_impact      SMALLINT         NOT NULL,
    creativity         SMALLINT         NOT NULL,
    color_harmony      SMALLINT         NOT NULL,
    storytelling       SMALLINT         NOT NULL,
    style_suitability  SMALLINT         NOT NULL,
    weighted_score     DOUBLE PRECISION NOT NULL
);


-- CHANGE: themes is now an array of JSONB (JSONB[]).
CREATE TABLE color
(
    visual_analysis_id BIGINT PRIMARY KEY REFERENCES visual_analysis (id) ON DELETE CASCADE,
    prominent_colors   TEXT[] NOT NULL,
    average_hue        REAL   NOT NULL,
    average_saturation REAL   NOT NULL,
    average_lightness  REAL   NOT NULL,
    histogram          JSONB  NOT NULL
);


-- CHANGE: All boolean flags are now explicitly marked as NOT NULL.
CREATE TABLE classification
(
    visual_analysis_id   BIGINT PRIMARY KEY REFERENCES visual_analysis (id) ON DELETE CASCADE,
    caption              TEXT    NOT NULL,
    main_subject         TEXT    NOT NULL,
    search_term          TEXT    NOT NULL,
    contains_pets        BOOLEAN NOT NULL,
    contains_vehicle     BOOLEAN NOT NULL,
    contains_landmarks   BOOLEAN NOT NULL,
    contains_people      BOOLEAN NOT NULL,
    contains_animals     BOOLEAN NOT NULL,
    contains_text        BOOLEAN NOT NULL,
    is_indoor            BOOLEAN NOT NULL,
    is_food              BOOLEAN NOT NULL,
    is_drink             BOOLEAN NOT NULL,
    is_event             BOOLEAN NOT NULL,
    is_document          BOOLEAN NOT NULL,
    is_landscape         BOOLEAN NOT NULL,
    is_cityscape         BOOLEAN NOT NULL,
    is_activity          BOOLEAN NOT NULL,
    setting              TEXT    NOT NULL,
    ocr_text             TEXT,
    animal_type          TEXT,
    food_name            TEXT,
    drink_name           TEXT,
    vehicle_type         TEXT,
    event_type           TEXT,
    landmark_name        TEXT,
    document_type        TEXT,
    people_count         INT,
    people_mood          TEXT,
    photo_type           TEXT,
    activity_description TEXT
);
CREATE INDEX idx_classification_contains_people ON classification (contains_people) WHERE contains_people = true;
CREATE INDEX idx_classification_contains_pets ON classification (contains_pets) WHERE contains_pets = true;
CREATE INDEX idx_classification_is_landscape ON classification (is_landscape) WHERE is_landscape = true;
CREATE INDEX idx_classification_contains_landmarks ON classification (contains_landmarks) WHERE contains_landmarks = true;

CREATE INDEX idx_classification_landmark_name ON classification (landmark_name);
CREATE INDEX idx_classification_setting ON classification (setting);

-- Search suggestions indices:
CREATE
    EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX trgm_idx_classification_term ON classification USING gin (search_term gin_trgm_ops);
CREATE INDEX trgm_idx_person_name ON person USING gin (name gin_trgm_ops);
CREATE INDEX trgm_idx_location_name ON location USING gin (name gin_trgm_ops);
CREATE INDEX trgm_idx_location_admin1 ON location USING gin (admin1 gin_trgm_ops);
CREATE INDEX trgm_idx_location_admin2 ON location USING gin (admin2 gin_trgm_ops);
CREATE INDEX trgm_idx_location_country_name ON location USING gin (country_name gin_trgm_ops);
CREATE INDEX trgm_idx_object_tag ON object USING gin (tag gin_trgm_ops);
CREATE INDEX idx_object_tag ON object (tag);
CREATE INDEX idx_visual_analysis_user_id ON visual_analysis (user_id);
CREATE INDEX idx_classification_search_term ON classification (search_term);
CREATE INDEX idx_person_user_id_name ON person (user_id, name);

-- For search performance
CREATE INDEX idx_visual_analysis_search_filters ON visual_analysis (user_id, deleted, media_item_id);