use crate::api_state::ApiContext;
use crate::jobs::handlers::{
    cancel_job_handler, get_failed_ingest_jobs_handler, get_running_ingest_jobs_handler,
    ingest_overview_handler, job_summary_handler, retry_ingest_job_handler, retry_job_handler,
    scan_user_media_handler,
};
use axum::{
    Router,
    routing::{get, post},
};

pub fn jobs_admin_router() -> Router<ApiContext> {
    Router::new()
        .route("/jobs", get(job_summary_handler))
        .route("/jobs/{id}/cancel", post(cancel_job_handler))
        .route("/jobs/{id}/retry", post(retry_job_handler))
}

pub fn jobs_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/jobs/ingest/overview", get(ingest_overview_handler))
        .route("/jobs/ingest/running", get(get_running_ingest_jobs_handler))
        .route("/jobs/ingest/failed", get(get_failed_ingest_jobs_handler))
        .route("/jobs/ingest/scan", post(scan_user_media_handler))
        .route("/jobs/ingest/retry", post(retry_ingest_job_handler))
}
