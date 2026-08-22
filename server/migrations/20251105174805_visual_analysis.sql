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

CREATE TABLE color
(
    visual_analysis_id BIGINT PRIMARY KEY REFERENCES visual_analysis (id) ON DELETE CASCADE,
    prominent_colors   TEXT[] NOT NULL,
    average_hue        REAL   NOT NULL,
    average_saturation REAL   NOT NULL,
    average_lightness  REAL   NOT NULL,
    histogram          JSONB  NOT NULL
);

-- Search suggestions indices:
CREATE
    EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX trgm_idx_person_name ON person USING gin (name gin_trgm_ops);
CREATE INDEX trgm_idx_location_name ON location USING gin (name gin_trgm_ops);
CREATE INDEX trgm_idx_location_admin1 ON location USING gin (admin1 gin_trgm_ops);
CREATE INDEX trgm_idx_location_admin2 ON location USING gin (admin2 gin_trgm_ops);
CREATE INDEX trgm_idx_location_country_name ON location USING gin (country_name gin_trgm_ops);
CREATE INDEX trgm_idx_object_tag ON object USING gin (tag gin_trgm_ops);
CREATE INDEX idx_object_tag ON object (tag);
CREATE INDEX idx_visual_analysis_user_id ON visual_analysis (user_id);
CREATE INDEX idx_person_user_id_name ON person (user_id, name);

-- For search performance
CREATE INDEX idx_visual_analysis_search_filters ON visual_analysis (user_id, deleted, media_item_id);