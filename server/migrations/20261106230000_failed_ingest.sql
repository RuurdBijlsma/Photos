CREATE TABLE failed_ingest
(
    relative_path TEXT PRIMARY KEY,
    user_id       INT         NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    error         TEXT        NOT NULL,
    failed_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_failed_ingest_user_id ON failed_ingest (user_id);
