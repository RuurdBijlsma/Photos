CREATE TYPE job_type AS ENUM ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis', 'ingest_llm',
    'remove', 'scan', 'clean_db', 'cluster_faces', 'cluster_photos', 'import_album_item', 'update_global_centroid',
    'sync_thumbnails', 'generate_daily_cards', 'calc_system_stats', 'handle_web_upload');
CREATE TYPE job_status AS ENUM ('queued', 'running', 'failed', 'done', 'cancelled');
CREATE TYPE job_scope AS ENUM ('path', 'global');

CREATE TABLE jobs
(
    id                  BIGSERIAL PRIMARY KEY,
    relative_path       TEXT,                                  -- references files table
    user_id             INT REFERENCES app_user (id) ON DELETE CASCADE,
    job_type            job_type    NOT NULL,
    scope               job_scope   NOT NULL,
    phase               INT         NOT NULL,
    payload             JSONB,                                 -- For storing extra job parameters
    priority            INT         NOT NULL DEFAULT 100,      -- lower = higher priority
    status              job_status  NOT NULL DEFAULT 'queued', -- queued, running, failed, done, cancelled
    attempts            INT         NOT NULL DEFAULT 0,
    dependency_attempts INT         NOT NULL DEFAULT 0,
    max_attempts        INT         NOT NULL DEFAULT 5,
    owner               TEXT,                                  -- worker id that claimed it
    started_at          TIMESTAMPTZ,
    finished_at         TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    scheduled_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_heartbeat      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error          TEXT
);

-- Unique constraint for active job deduplication (used by ON CONFLICT)
CREATE UNIQUE INDEX uq_jobs_active_job
    ON jobs (
             job_type,
             coalesce(user_id, -1),
             coalesce(md5(payload::text), ''),
             coalesce(relative_path, '')
        ) WHERE status IN ('queued', 'running');

-- Fast candidate selection for workers (covers ORDER BY in claim_next_job)
CREATE INDEX idx_jobs_claim_active
    ON jobs (priority ASC, relative_path DESC, scheduled_at ASC, created_at ASC)
    WHERE status IN ('queued', 'running');

-- Fast path-level phase calculations (used by path_limits CTE in claim/demand queries)
CREATE INDEX idx_jobs_active_path_phase
    ON jobs (relative_path, phase)
    WHERE status IN ('queued', 'running');

-- Fast global/overall phase calculations (used by phase_limits CTE in claim/demand queries)
CREATE INDEX idx_jobs_active_global_phase
    ON jobs (scope, phase)
    WHERE status IN ('queued', 'running');

-- Fast path lookup for job cancellations and per-job logic
CREATE INDEX idx_jobs_active_relative_path
    ON jobs (relative_path)
    WHERE status IN ('queued', 'running');

-- User filtering (e.g., full scan enqueueing)
CREATE INDEX idx_jobs_user_id ON jobs (user_id);

-- Dashboard / Monitoring
CREATE INDEX jobs_status_priority_idx ON jobs (status, priority, scheduled_at, created_at);

-- For performance when filtering/sorting by job type or timestamp
CREATE INDEX idx_jobs_job_type ON jobs (job_type);
CREATE INDEX idx_jobs_created_at ON jobs (created_at DESC);
CREATE INDEX idx_jobs_scheduled_at ON jobs (scheduled_at DESC);