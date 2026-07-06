-- For storing key-vector pairs
CREATE TABLE key_vector_store
(
    key        TEXT PRIMARY KEY,
    vector     VECTOR(768),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- Persistent key-json pairs
CREATE TABLE key_json_store
(
    id         SERIAL PRIMARY KEY,
    key        TEXT NOT NULL,
    value      JSONB,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    user_id    INT REFERENCES app_user (id) ON DELETE CASCADE, -- if user_id is NULL, then it's a global key/value pair
    CONSTRAINT uq_key_json_store UNIQUE NULLS NOT DISTINCT (user_id, key)
);

-- Text embedding cache to speed up search
CREATE TABLE text_embedding_cache
(
    id         UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    model_id   TEXT        NOT NULL,
    text       TEXT        NOT NULL,
    embedding  vector(768) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (model_id, text)
);

CREATE INDEX idx_text_embedding_cache_lookup ON text_embedding_cache (model_id, text);

-- Vision embedding cache to speed up search
CREATE TABLE vision_embedding_cache
(
    id         UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    model_id   TEXT        NOT NULL,
    uuid       UUID        NOT NULL,
    embedding  vector(768) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (model_id, uuid)
);

CREATE INDEX idx_vision_embedding_cache_lookup ON vision_embedding_cache (model_id, uuid);
