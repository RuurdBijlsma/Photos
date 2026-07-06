CREATE TYPE daily_card_status AS ENUM ('unopened', 'in_progress', 'complete');

CREATE TABLE daily_card
(
    id                      SERIAL PRIMARY KEY,
    user_id                 INT         NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    card_date               DATE, -- not all cards require a date. For example a cluster type card can be shown on any day. An "on this day" card has to be shown on a specific day.
    card_type               TEXT        NOT NULL,
    title                   TEXT        NOT NULL,
    subtitle                TEXT,
    thumbnail_media_item_id VARCHAR(10) REFERENCES media_item (id) ON DELETE SET NULL,
    payload                 JSONB       NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    shown                   BOOLEAN     NOT NULL DEFAULT false
);

-- Index for fast lookups by user and card date
CREATE INDEX idx_daily_card_user_date ON daily_card (user_id, card_date);

-- Index for the foreign key referencing media_item to optimize deletion lookups
CREATE INDEX idx_daily_card_thumbnail_media_item_id ON daily_card (thumbnail_media_item_id);