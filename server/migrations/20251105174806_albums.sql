-- Step 1: Define custom ENUM types for roles and statuses to ensure data integrity.

CREATE TYPE album_role AS ENUM ('owner', 'contributor', 'viewer');
CREATE TYPE album_sort AS ENUM ('date_desc', 'date_asc', 'added_desc', 'added_asc', 'none');
CREATE TYPE invitation_status AS ENUM ('pending', 'accepted', 'rejected');


-- Step 2: Create the core tables for local album management.

-- The main 'album' table.
CREATE TABLE album
(
    id                            VARCHAR(10) PRIMARY KEY,
    owner_id                      INTEGER     NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    name                          TEXT        NOT NULL,
    description                   TEXT,
    thumbnail_id                  VARCHAR(10) NULL REFERENCES media_item (id) ON DELETE SET NULL,
    -- This flag enables public, view-only link sharing without requiring a login.
    is_public                     BOOLEAN     NOT NULL DEFAULT false,
    -- sort columns: automatically updated via trigger
    latest_media_item_timestamp   TIMESTAMPTZ,
    earliest_media_item_timestamp TIMESTAMPTZ,
    sort_mode                     album_sort  NOT NULL,
    media_count                   INT         NOT NULL DEFAULT 0,
    created_at                    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Index to quickly find all albums owned by a specific user.
CREATE INDEX idx_album_owner_id ON album (owner_id);
-- Index to quickly find public albums by their ID.
CREATE INDEX idx_album_is_public ON album (id) WHERE is_public = true;


-- A many-to-many join table connecting albums and media_items.
-- Ranks are strictly doubles to allow insertion between existing items without full rebalancing.
CREATE TABLE album_media_item
(
    album_id      VARCHAR(10)      NOT NULL REFERENCES album (id) ON DELETE CASCADE,
    media_item_id VARCHAR(10)      NOT NULL REFERENCES media_item (id) ON DELETE CASCADE,
    added_by_user INT              REFERENCES app_user (id) ON DELETE SET NULL,
    added_at      TIMESTAMPTZ      NOT NULL DEFAULT now(),
    -- The user can manually sort items, so sort by rank
    rank          DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (album_id, media_item_id),
    CONSTRAINT uq_album_media_rank UNIQUE (album_id, rank) DEFERRABLE INITIALLY DEFERRED
);

-- Indices for album_media_item
CREATE INDEX idx_album_media_item_rank ON album_media_item (album_id, rank);
CREATE INDEX idx_album_media_item_album_id ON album_media_item (album_id);
CREATE INDEX idx_album_media_item_media_item_id ON album_media_item (media_item_id);


-- Step 3: Create the collaborator table, designed to handle BOTH local and remote users.

-- Manages permissions for albums, linking users (local or remote) to albums with a specific role.
CREATE TABLE album_collaborator
(
    id       BIGSERIAL PRIMARY KEY,
    album_id VARCHAR(10) NOT NULL REFERENCES album (id) ON DELETE CASCADE,
    user_id  INTEGER     NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    role     album_role  NOT NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- A user can only have one role per album.
    CONSTRAINT uq_album_local_collaborator UNIQUE (album_id, user_id)
);

CREATE INDEX idx_album_collaborator_user_album ON album_collaborator (user_id, album_id);

-- For search suggestion performance:
CREATE INDEX idx_album_name ON album (name);