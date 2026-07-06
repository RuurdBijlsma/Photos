-- Ensure the vector extension is available.
CREATE
    EXTENSION IF NOT EXISTS vector;

-- Represents a person, which is a collection of one or more face clusters.
CREATE TABLE person
(
    id            VARCHAR(10) PRIMARY KEY,
    user_id       INT         NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    name          TEXT,        -- The name assigned by the user, e.g., "Jane Doe"
    face_thumb_id VARCHAR(10), -- Will be linked to a face_cluster ID later
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_person_user_id ON person (user_id);
CREATE INDEX idx_person_public_thumb_lookup ON person (id) INCLUDE (face_thumb_id);

-- Represents a cluster of similar faces. Many clusters can belong to one person.
CREATE TABLE face_cluster
(
    id                  VARCHAR(10) PRIMARY KEY,
    user_id             INT         NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    centroid            VECTOR(512), -- The average face embedding for this cluster
    person_id           VARCHAR(10) NOT NULL REFERENCES person (id) ON DELETE CASCADE,
    thumb_media_item_id VARCHAR(10) REFERENCES media_item (id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_face_cluster_user_id ON face_cluster (user_id);
CREATE INDEX idx_face_cluster_person_id ON face_cluster (person_id);
ALTER TABLE person
    ADD CONSTRAINT fk_person_face_thumb
        FOREIGN KEY (face_thumb_id) REFERENCES face_cluster (id) ON DELETE SET NULL;

-- Represents a cluster of visually similar photos.
CREATE TABLE photo_cluster
(
    id                      VARCHAR(10) PRIMARY KEY,
    user_id                 INT         NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    friendly_label          TEXT,        -- Optional auto generated title
    thumbnail_media_item_id VARCHAR(10) REFERENCES media_item (id) ON DELETE SET NULL,
    centroid                VECTOR(768), -- The average photo embedding
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Index for quickly finding all clusters belonging to a user.
CREATE INDEX idx_photo_cluster_user_id ON photo_cluster (user_id);

CREATE TABLE media_item_photo_cluster
(
    media_item_id    VARCHAR(10) NOT NULL REFERENCES media_item (id) ON DELETE CASCADE,
    photo_cluster_id VARCHAR(10) NOT NULL REFERENCES photo_cluster (id) ON DELETE CASCADE,
    PRIMARY KEY (media_item_id, photo_cluster_id)
);

-- Index for finding all media items in a specific cluster.
CREATE INDEX idx_media_item_photo_cluster_cluster_id ON media_item_photo_cluster (photo_cluster_id);

CREATE TABLE cluster_tags
(
    tag       TEXT        NOT NULL PRIMARY KEY,
    embedding VECTOR(768) NOT NULL
);

CREATE INDEX idx_cluster_tags_user_id ON photo_cluster (user_id);
CREATE INDEX idx_cluster_tags_embedding_hnsw
    ON cluster_tags
        USING hnsw (embedding vector_cosine_ops);