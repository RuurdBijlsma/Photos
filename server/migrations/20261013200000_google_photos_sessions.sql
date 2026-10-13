CREATE TABLE google_photos_import_session (
    id          TEXT         PRIMARY KEY,
    user_id     INTEGER      NOT NULL REFERENCES app_user(id) ON DELETE CASCADE,
    picker_session_id TEXT   NOT NULL,
    picker_uri  TEXT         NOT NULL,
    access_token TEXT        NOT NULL,
    status      TEXT         NOT NULL DEFAULT 'pending',
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ  NOT NULL
);

CREATE INDEX idx_google_photos_import_session_user_id ON google_photos_import_session(user_id);
