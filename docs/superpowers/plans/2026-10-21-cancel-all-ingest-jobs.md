# Cancel All Ingest Jobs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:
> executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give a user one button that cancels all of their queued and running per-file ingest jobs, and make that
cancellation stick when a worker finishes an in-flight job.

**Architecture:** A protected `POST /api/jobs/ingest/cancel-all` performs one user-scoped `UPDATE jobs` for
`ingest_metadata`, `ingest_thumbnails`, and `ingest_analysis`, returning the affected count. Worker completion/requeue
writes get a `status <> 'cancelled'` guard so a job already running at cancel time cannot overwrite the cancellation.
The Activity dashboard gets a confirmed "Cancel all" action. "Scan Folder" also enqueues `SyncThumbnails` so the
existing scan button re-ingests items with missing thumbnails after an accidental cancel.

**Tech Stack:** Rust, Axum, SQLx, Postgres, Vue 3, Pinia, Vuetify.

**Spec:** Design agreed in conversation on 2026-10-21; this plan is the source of truth.

## Global Constraints

- No database migration: `job_status` already contains `cancelled`.
- Scope is exactly the three per-file ingest types. Do **not** cancel `remove`, `scan`, `sync_thumbnails`,
  `ingest_takeout_batch`, `handle_web_upload`, cluster jobs, or stats jobs.
- Every cancel query must filter `user_id = $1`. Never reuse the admin-only `cancel_job`.
- Cancel is point-in-time. Jobs created after the call (by a running scan, an upload finalization, or a takeout batch)
  are allowed and may need a second click.
- Cancelled jobs are not deleted and do not delete files or `media_item` rows. A later scan marks deleted files missing.
- Use SQLx compile-time-checked queries like the surrounding code, then regenerate `server/.sqlx` with
  `cargo sqlx prepare --workspace`.
- Match the existing Vuetify card/dialog styling and the existing Pinia store/service split.

## Out Of Scope / Known Limitations

- Takeout batches and web-upload finalization are not cancelled. A running takeout batch can still enqueue child jobs
  after the click.
- Cancelled jobs do not appear in the queue tabs, so the user cannot retry one individually from this screen.
- The admin-only `POST /jobs/retry-cancelled` can still resurrect a cancelled batch.
- Scan + `SyncThumbnails` restores items missing thumbnails. It does not restore an item whose thumbnails finished but
  whose analysis was cancelled.

## Review Focus

- Cancel-all from one user must not touch another user's jobs. Covered in Task 1.
- A running job that returns `Done` after the cancel must not overwrite `cancelled`. Covered in Task 2.
- A running job whose file vanished must not requeue itself after the cancel (`FileNotFound`, `DependencyReschedule`,
  storage unavailable, retry). Covered in Task 2.
- A second click must return `0`, not an error. Covered in Task 1.
- Scan Folder must enqueue both `Scan` and `SyncThumbnails`, otherwise the stated undo path only discovers brand-new
  files. Covered in Task 3.

---

### Task 1: User-scoped cancel-all endpoint

**Files:**

- Modify: `server/crates/libs/common_services/src/api/jobs/interfaces.rs:91`
- Modify: `server/crates/libs/common_services/src/api/jobs/user_level.rs` (append after `retry_user_job`)
- Modify: `server/crates/apps/api/src/routes/jobs/handlers.rs`
- Modify: `server/crates/apps/api/src/routes/jobs/router.rs:21-29`
- Create: `server/crates/test_integration/src/tests/test_jobs.rs`
- Modify: `server/crates/test_integration/src/tests/mod.rs`
- Modify: `server/crates/test_integration/src/lib.rs:48-84`
- Regenerate: `server/.sqlx/query-*.json`
- Test: `server/crates/test_integration/src/tests/test_jobs.rs`

**Interfaces:**

- Consumes: `ApiUser { id: i32 }` from the protected router; `PgPool`.
- Produces:
    - `BulkCancelResponse { pub cancelled: i64 }` in `common_services::api::jobs::interfaces`.
    - `cancel_user_ingest_jobs(pool: &PgPool, user_id: i32) -> Result<i64, AppError>`.
    - `POST /api/jobs/ingest/cancel-all` → `200 {"cancelled": <n>}`. Repeated calls return `0`.

- [ ] **Step 1: Write the failing integration test**

Create `server/crates/test_integration/src/tests/test_jobs.rs`:

```rust
use crate::runner::context::test_context::TestContext;
use crate::test_helpers::login;
use color_eyre::Result;
use common_services::api::jobs::interfaces::BulkCancelResponse;
use common_types::dev_constants::EMAIL;
use reqwest::StatusCode;
use sqlx::PgPool;

async fn insert_job(pool: &PgPool, user_id: i32, job_type: &str, running: bool) -> Result<i64> {
    let id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO jobs (relative_path, user_id, job_type, scope, phase, priority, status,
                          owner, started_at, last_heartbeat, scheduled_at)
        VALUES ($1, $2, $3::job_type, 'path'::job_scope, 20, 50,
                CASE WHEN $4 THEN 'running'::job_status ELSE 'queued'::job_status END,
                CASE WHEN $4 THEN 'integration-test' ELSE NULL END,
                CASE WHEN $4 THEN now() ELSE NULL END,
                CASE WHEN $4 THEN now() + interval '1 hour' ELSE now() END,
                now() + interval '1 hour')
        RETURNING id
        "#,
    )
        .bind(format!("cancel-all/{user_id}/{job_type}.jpg"))
        .bind(user_id)
        .bind(job_type)
        .bind(running)
        .fetch_one(pool)
        .await?;
    Ok(id)
}

async fn status_of(pool: &PgPool, job_id: i64) -> Result<String> {
    Ok(sqlx::query_scalar("SELECT status::text FROM jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(pool)
        .await?)
}

pub async fn test_cancel_all_ingest_jobs(context: &TestContext) -> Result<()> {
    login(context).await?;
    let url = format!(
        "{}/api/jobs/ingest/cancel-all",
        context.settings.api.public_url
    );
    let user_id: i32 = sqlx::query_scalar("SELECT id FROM app_user WHERE email = $1")
        .bind(EMAIL)
        .fetch_one(&context.pool)
        .await?;

    // Clear leftovers from earlier tests so the counts below are deterministic.
    sqlx::query(
        "DELETE FROM jobs WHERE user_id = $1 \
         AND job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis') \
         AND status IN ('queued', 'running')",
    )
        .bind(user_id)
        .execute(&context.pool)
        .await?;

    let other_user_id: i32 = sqlx::query_scalar(
        "INSERT INTO app_user (email, password, name) VALUES ('cancel-all-other@test.local', 'x', 'Other') RETURNING id",
    )
        .fetch_one(&context.pool)
        .await?;

    let metadata_id = insert_job(&context.pool, user_id, "ingest_metadata", false).await?;
    let thumbnails_id = insert_job(&context.pool, user_id, "ingest_thumbnails", true).await?;
    let analysis_id = insert_job(&context.pool, user_id, "ingest_analysis", false).await?;
    let remove_id = insert_job(&context.pool, user_id, "remove", false).await?;
    let takeout_id = insert_job(&context.pool, user_id, "ingest_takeout_batch", false).await?;
    let other_user_job_id = insert_job(&context.pool, other_user_id, "ingest_metadata", false).await?;

    let response = context.http_client.post(&url).send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body: BulkCancelResponse = response.json().await?;
    assert_eq!(body.cancelled, 3);

    assert_eq!(status_of(&context.pool, metadata_id).await?, "cancelled");
    assert_eq!(status_of(&context.pool, thumbnails_id).await?, "cancelled");
    assert_eq!(status_of(&context.pool, analysis_id).await?, "cancelled");
    assert_eq!(status_of(&context.pool, remove_id).await?, "queued");
    assert_eq!(status_of(&context.pool, takeout_id).await?, "queued");
    assert_eq!(status_of(&context.pool, other_user_job_id).await?, "queued");

    // Idempotent: a second click cancels nothing.
    let response = context.http_client.post(&url).send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body: BulkCancelResponse = response.json().await?;
    assert_eq!(body.cancelled, 0);

    let ids = [metadata_id, thumbnails_id, analysis_id, remove_id, takeout_id];
    sqlx::query("DELETE FROM jobs WHERE id = ANY($1)")
        .bind(ids.as_slice())
        .execute(&context.pool)
        .await?;
    sqlx::query("DELETE FROM app_user WHERE id = $1")
        .bind(other_user_id)
        .execute(&context.pool)
        .await?;

    Ok(())
}
```

- [ ] **Step 2: Register the test**

In `server/crates/test_integration/src/tests/mod.rs`, add `pub mod test_jobs;`.

In `server/crates/test_integration/src/lib.rs`, add `use crate::tests::test_jobs::test_cancel_all_ingest_jobs;` and add
`test_cancel_all_ingest_jobs,` at the end of the `execute_suite!` list.

- [ ] **Step 3: Run the test to verify it fails**

Run from `server`:

```powershell
cargo test -p test_integration integration_suite -- --nocapture
```

Expected: FAIL because `/api/jobs/ingest/cancel-all` returns 404.

- [ ] **Step 4: Add the response type**

In `server/crates/libs/common_services/src/api/jobs/interfaces.rs`, after `BulkRetryResponse`:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BulkCancelResponse {
    pub cancelled: i64,
}
```

- [ ] **Step 5: Add the service function**

In `server/crates/libs/common_services/src/api/jobs/user_level.rs`, append:

```rust
/// Cancels all queued and running per-file ingest jobs owned by one user.
pub async fn cancel_user_ingest_jobs(pool: &PgPool, user_id: i32) -> Result<i64, AppError> {
    let result = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = 'cancelled'::job_status,
            finished_at = NOW()
        WHERE user_id = $1
          AND status IN ('queued'::job_status, 'running'::job_status)
          AND job_type IN ('ingest_metadata'::job_type, 'ingest_thumbnails'::job_type, 'ingest_analysis'::job_type)
        "#,
        user_id
    )
        .execute(pool)
        .await?;

    Ok(result.rows_affected() as i64)
}
```

- [ ] **Step 6: Add the handler and route**

In `server/crates/apps/api/src/routes/jobs/handlers.rs`, import `BulkCancelResponse` and `cancel_user_ingest_jobs`, then
add:

```rust
#[instrument(skip(pool, user), err(Debug))]
pub async fn cancel_all_ingest_jobs_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<BulkCancelResponse>, AppError> {
    let cancelled = cancel_user_ingest_jobs(&pool, user.id).await?;
    Ok(Json(BulkCancelResponse { cancelled }))
}
```

In `server/crates/apps/api/src/routes/jobs/router.rs`, add to `jobs_protected_router()`:

```rust
.route("/jobs/ingest/cancel-all", post(cancel_all_ingest_jobs_handler))
```

- [ ] **Step 7: Regenerate SQLx offline metadata**

Run from `server` with the dev database running:

```powershell
cargo sqlx prepare --workspace
```

Expected: `server/.sqlx` gains a `query-*.json` entry for the new `UPDATE`.

- [ ] **Step 8: Run the test to verify it passes**

Run from `server`:

```powershell
cargo test -p test_integration integration_suite -- --nocapture
```

Expected: PASS, including `test_cancel_all_ingest_jobs`.

- [ ] **Step 9: Commit**

```powershell
git add server/crates/libs/common_services/src/api/jobs/interfaces.rs server/crates/libs/common_services/src/api/jobs/user_level.rs server/crates/apps/api/src/routes/jobs/handlers.rs server/crates/apps/api/src/routes/jobs/router.rs server/crates/test_integration/src/tests/test_jobs.rs server/crates/test_integration/src/tests/mod.rs server/crates/test_integration/src/lib.rs server/.sqlx
git commit -m "feat: add user-scoped cancel-all ingest endpoint"
```

---

### Task 2: Make cancellation stick in the worker

**Files:**

- Modify: `server/crates/apps/worker/src/jobs/management.rs:162-261`
- Modify: `server/crates/test_integration/Cargo.toml` (add dev-dependency)
- Modify: `server/crates/test_integration/src/tests/test_jobs.rs`
- Modify: `server/crates/test_integration/src/lib.rs`
- Test: `server/crates/test_integration/src/tests/test_jobs.rs`

**Interfaces:**

- Consumes: `worker::jobs::management::update_job_on_completion`, `update_job_on_failure`;
  `worker::handlers::JobResult`; `common_services::database::jobs::{Job, JobScope, JobType}`.
- Produces: completion, failure, and every requeue path leave a job `cancelled` instead of overwriting it.

- [ ] **Step 1: Add the worker dev-dependency**

In `server/crates/test_integration/Cargo.toml`:

```toml
[dev-dependencies]
worker = { path = "../apps/worker" }
```

- [ ] **Step 2: Write the failing test**

Append to `server/crates/test_integration/src/tests/test_jobs.rs`:

```rust
use common_services::database::jobs::{Job, JobScope, JobType};
use worker::handlers::JobResult;
use worker::jobs::management::{update_job_on_completion, update_job_on_failure};

pub async fn test_cancelled_job_is_not_resurrected(context: &TestContext) -> Result<()> {
    let user_id: i32 = sqlx::query_scalar("SELECT id FROM app_user WHERE email = $1")
        .bind(EMAIL)
        .fetch_one(&context.pool)
        .await?;
    let job_id = insert_job(&context.pool, user_id, "ingest_metadata", false).await?;

    sqlx::query(
        "UPDATE jobs SET status = 'cancelled'::job_status, finished_at = now() WHERE id = $1",
    )
        .bind(job_id)
        .execute(&context.pool)
        .await?;

    let job = Job {
        id: job_id,
        payload: None,
        relative_path: Some("cancel-all/guard.jpg".to_owned()),
        user_id: Some(user_id),
        job_type: JobType::IngestMetadata,
        scope: JobScope::Path,
        phase: 20,
        priority: 50,
        attempts: 0,
        max_attempts: 5,
        dependency_attempts: 0,
    };

    update_job_on_completion(&context.pool, &job, JobResult::DependencyReschedule).await?;
    update_job_on_completion(&context.pool, &job, JobResult::FileNotFound).await?;
    update_job_on_completion(
        &context.pool,
        &job,
        JobResult::StorageUnavailable("offline".to_owned()),
    )
        .await?;
    update_job_on_completion(&context.pool, &job, JobResult::Done).await?;
    update_job_on_failure(
        &context.pool,
        &job,
        &color_eyre::eyre::eyre!("simulated failure"),
    )
        .await?;

    let row = sqlx::query(
        "SELECT status::text AS status, dependency_attempts, attempts, last_error, finished_at \
         FROM jobs WHERE id = $1",
    )
        .bind(job_id)
        .fetch_one(&context.pool)
        .await?;

    use sqlx::Row;
    assert_eq!(row.try_get::<String, _>("status")?, "cancelled");
    assert_eq!(row.try_get::<i32, _>("dependency_attempts")?, 0);
    assert_eq!(row.try_get::<i32, _>("attempts")?, 0);
    assert!(row.try_get::<Option<String>, _>("last_error")?.is_none());
    assert!(row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("finished_at")?.is_some());

    sqlx::query("DELETE FROM jobs WHERE id = $1")
        .bind(job_id)
        .execute(&context.pool)
        .await?;

    Ok(())
}
```

- [ ] **Step 3: Register the test**

In `server/crates/test_integration/src/lib.rs`, import `test_cancelled_job_is_not_resurrected` and add it to
`execute_suite!` after `test_cancel_all_ingest_jobs`.

- [ ] **Step 4: Run the test to verify it fails**

Run from `server`:

```powershell
cargo test -p test_integration integration_suite -- --nocapture
```

Expected: FAIL. The final status is `queued` (or `done`), not `cancelled`, because the completion writes ignore the
cancelled state.

- [ ] **Step 5: Guard every completion/requeue update**

In `server/crates/apps/worker/src/jobs/management.rs`, add `AND status <> 'cancelled'` to the `WHERE` clause of each of
these functions:

- `mark_job_done`
- `mark_job_failed`
- `reschedule_for_retry`
- `reschedule_storage_unavailable`
- `reschedule_file_not_found`
- `dependency_reschedule_job`

Example for `mark_job_done`:

```rust
sqlx::query!(
    "UPDATE jobs SET status = 'done', finished_at = now() WHERE id = $1 AND status <> 'cancelled'",
    job_id
)
```

Do not add the guard to `mark_job_cancelled`.

- [ ] **Step 6: Run the test to verify it passes**

Run from `server`:

```powershell
cargo test -p test_integration integration_suite -- --nocapture
```

Expected: PASS, including `test_cancelled_job_is_not_resurrected`.

- [ ] **Step 7: Commit**

```powershell
git add server/crates/apps/worker/src/jobs/management.rs server/crates/test_integration/Cargo.toml server/crates/test_integration/src/tests/test_jobs.rs server/crates/test_integration/src/lib.rs
git commit -m "fix: do not overwrite cancelled jobs on completion"
```

---

### Task 3: Make Scan Folder a real undo for missing thumbnails

**Files:**

- Modify: `server/crates/apps/api/src/routes/jobs/handlers.rs:89-99`
- Modify: `server/crates/test_integration/src/tests/test_jobs.rs`
- Modify: `server/crates/test_integration/src/lib.rs`
- Test: `server/crates/test_integration/src/tests/test_jobs.rs`

**Interfaces:**

- Consumes:
  `common_services::job_queue::enqueue_full_scan(pool: &PgPool, settings: &IngestSettings, target_user_id: i32) -> Result<()>`.
- Produces: `POST /api/jobs/ingest/scan` enqueues one `scan` and one `sync_thumbnails` job for the user.

- [ ] **Step 1: Write the failing test**

Append to `server/crates/test_integration/src/tests/test_jobs.rs`:

```rust
pub async fn test_scan_enqueues_thumbnail_sync(context: &TestContext) -> Result<()> {
    login(context).await?;
    let url = format!(
        "{}/api/jobs/ingest/scan",
        context.settings.api.public_url
    );

    // Clear active scan/sync rows so dedupe cannot hide the new jobs.
    sqlx::query(
        "DELETE FROM jobs WHERE job_type IN ('scan', 'sync_thumbnails') AND status IN ('queued', 'running')",
    )
        .execute(&context.pool)
        .await?;

    let max_id: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id), 0) FROM jobs")
        .fetch_one(&context.pool)
        .await?;

    let response = context.http_client.post(&url).send().await?;
    assert_eq!(response.status(), StatusCode::OK);

    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT job_type::text, COUNT(*)::bigint FROM jobs \
         WHERE id > $1 AND job_type IN ('scan', 'sync_thumbnails') GROUP BY job_type",
    )
        .bind(max_id)
        .fetch_all(&context.pool)
        .await?;

    assert!(rows.iter().any(|(job_type, count)| job_type == "scan" && *count == 1));
    assert!(
        rows.iter()
            .any(|(job_type, count)| job_type == "sync_thumbnails" && *count == 1)
    );

    sqlx::query("DELETE FROM jobs WHERE id > $1 AND job_type IN ('scan', 'sync_thumbnails')")
        .bind(max_id)
        .execute(&context.pool)
        .await?;

    Ok(())
}
```

- [ ] **Step 2: Register the test**

In `server/crates/test_integration/src/lib.rs`, import `test_scan_enqueues_thumbnail_sync` and add it to
`execute_suite!` after `test_cancelled_job_is_not_resurrected`.

- [ ] **Step 3: Run the test to verify it fails**

Run from `server`:

```powershell
cargo test -p test_integration integration_suite -- --nocapture
```

Expected: FAIL because only the `scan` job is enqueued.

- [ ] **Step 4: Use the full-scan helper**

In `server/crates/apps/api/src/routes/jobs/handlers.rs`, replace the body of `scan_user_media_handler`:

```rust
enqueue_full_scan( & context.pool, & context.settings.ingest, user.id).await?;
Ok(Json(()))
```

Update imports: add `enqueue_full_scan` from `common_services::job_queue`; remove `enqueue_job` and `JobType` if they
are now unused.

- [ ] **Step 5: Run the test to verify it passes**

Run from `server`:

```powershell
cargo test -p test_integration integration_suite -- --nocapture
```

Expected: PASS, including `test_scan_enqueues_thumbnail_sync`.

- [ ] **Step 6: Commit**

```powershell
git add server/crates/apps/api/src/routes/jobs/handlers.rs server/crates/test_integration/src/tests/test_jobs.rs server/crates/test_integration/src/lib.rs
git commit -m "feat: run thumbnail sync with the scan button"
```

---

### Task 4: Add the Cancel all button and confirmation

**Files:**

- Modify: `web/src/scripts/types/api/ingestJobs.ts`
- Modify: `web/src/scripts/services/ingestJobsService.ts`
- Modify: `web/src/scripts/stores/ingestJobsStore.ts`
- Modify: `web/src/vues/components/activity/cards/IngestQueueTable.vue`

**Interfaces:**

- Consumes: `POST /api/jobs/ingest/cancel-all` → `{ "cancelled": number }`.
- Produces: `BulkCancelResponse { cancelled: number }`, `ingestJobsService.cancelAll()`, `ingestStore.cancelAllJobs()`,
  `ingestStore.pendingIngestCount`.

- [ ] **Step 1: Add the response type**

In `web/src/scripts/types/api/ingestJobs.ts`:

```ts
export interface BulkCancelResponse {
    cancelled: number
}
```

- [ ] **Step 2: Add the service call**

In `web/src/scripts/services/ingestJobsService.ts`, import `BulkCancelResponse` and add:

```ts
/**
 * Cancel all queued and running ingest jobs for the current user.
 */
cancelAll()
:
Promise < AxiosResponse < BulkCancelResponse >> {
    return apiClient.post<BulkCancelResponse>('/jobs/ingest/cancel-all')
},
```

- [ ] **Step 3: Add the store action and pending count**

In `web/src/scripts/stores/ingestJobsStore.ts`, import `computed` from `vue`, then add:

```ts
const pendingIngestCount = computed(() => {
    const counts = overview.value
    if (!counts) return 0
    return (
        counts.metadata.queued +
        counts.metadata.running +
        counts.thumbnails.queued +
        counts.thumbnails.running +
        counts.analysis.queued +
        counts.analysis.running
    )
})

async function cancelAllJobs() {
    try {
        const response = await ingestJobsService.cancelAll()
        const cancelled = response.data.cancelled
        snackbarStore.success(
            cancelled === 0 ? 'No pending ingest jobs to cancel' : `Cancelled ${cancelled} ingest jobs`,
        )
        await Promise.all([fetchOverview(), fetchRunning(), fetchUserJobs(true)])
        return cancelled
    } catch (error) {
        snackbarStore.error('Failed to cancel ingest jobs', error)
        throw error
    }
}
```

Return `pendingIngestCount` and `cancelAllJobs` from the store.

- [ ] **Step 4: Add the button and dialog**

In `web/src/vues/components/activity/cards/IngestQueueTable.vue`:

- Import `MdiCancel` from `~icons/mdi/cancel`.
- Add refs `cancelAllDialog = ref(false)` and `isCancelling = ref(false)`.
- Add `const canCancelAll = computed(() => ingestStore.pendingIngestCount > 0)`.
- Add:

```ts
async function handleCancelAll() {
    isCancelling.value = true
    try {
        await ingestStore.cancelAllJobs()
        cancelAllDialog.value = false
    } catch {
        // Handled in store
    } finally {
        isCancelling.value = false
    }
}
```

Replace the `<h2 class="card-title">` line with:

```vue

<div class="card-header-row">
  <h2 class="card-title">Ingestion Queue Details</h2>
  <v-btn
      color="error"
      variant="tonal"
      rounded="xl"
      :prepend-icon="MdiCancel"
      :disabled="!canCancelAll"
      @click="cancelAllDialog = true"
  >
    Cancel all
  </v-btn>
</div>
```

Add the dialog before `</v-card>` (outside `.card-body`):

```vue

<v-dialog v-model="cancelAllDialog" max-width="480">
  <v-card rounded="xl" color="surface-container-highest">
    <v-card-title>Cancel all ingest jobs?</v-card-title>
    <v-card-text>
      This cancels {{ ingestStore.pendingIngestCount }} queued and running ingest jobs.
      Running jobs stop at their next checkpoint. Photos and videos are not deleted, and new
      files added afterwards will still be ingested.
    </v-card-text>
    <v-card-actions>
      <v-spacer/>
      <v-btn variant="text" :disabled="isCancelling" @click="cancelAllDialog = false">
        Keep jobs
      </v-btn>
      <v-btn
          color="error"
          variant="tonal"
          :loading="isCancelling"
          @click="handleCancelAll"
      >
        Cancel jobs
      </v-btn>
    </v-card-actions>
  </v-card>
</v-dialog>
```

Add styles:

```css
.card-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
    flex-wrap: wrap;
}

.card-header-row .card-title {
    margin-bottom: 0;
}
```

- [ ] **Step 5: Run the web checks**

Run from `web`:

```powershell
npm run lint:type-check
npm run lint:oxlint
npm run prettier:check
```

Expected: all pass.

- [ ] **Step 6: Manual verification**

Start the app, open Activity, and confirm:

1. With pending jobs, "Cancel all" opens the dialog with the correct count.
2. Confirming shows "Cancelled N ingest jobs" and the queue/overview empty out.
3. With no pending jobs, the button is disabled.
4. "Scan Folder" still works and repopulates jobs for files that still exist without thumbnails.

- [ ] **Step 7: Commit**

```powershell
git add web/src/scripts/types/api/ingestJobs.ts web/src/scripts/services/ingestJobsService.ts web/src/scripts/stores/ingestJobsStore.ts web/src/vues/components/activity/cards/IngestQueueTable.vue
git commit -m "feat: add cancel-all ingest button"
```

---

### Task 5: Final verification

- [ ] **Step 1: Run Rust formatting and clippy**

From `server`:

```powershell
cargo fmt --all -- --check
cargo clippy --no-deps --all-features --tests --benches -- -D clippy::all -D clippy::pedantic -D clippy::nursery
```

Expected: no diffs, no warnings.

- [ ] **Step 2: Run the integration suite**

From `server`:

```powershell
cargo test -p test_integration integration_suite -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run the web build**

From `web`:

```powershell
npm run build
```

Expected: PASS.

- [ ] **Step 4: Commit any formatting fixes**

```powershell
git add server/crates/apps server/crates/libs server/crates/test_integration web/src
git commit -m "chore: format cancel-all ingest changes"
```

Only commit if `cargo fmt` or prettier changed something.
