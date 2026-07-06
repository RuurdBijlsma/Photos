use crate::api_state::ApiContext;
use axum::extract::{Path, State};
use axum::{Extension, Json};
use axum_extra::extract::Query;
use common_services::api::app_error::AppError;
use common_services::api::jobs::interfaces::{
    IngestOverviewResponse, JobInfo, JobsQuery, PaginatedJobsResponse, RetryJobPayload,
};
use common_services::api::jobs::service::{cancel_job, get_job_overview, retry_job};
use common_services::api::jobs::user_level::{
    enqueue_scan_job, get_failed_ingest_jobs, get_running_ingest_jobs, get_user_ingest_overview,
    retry_user_job,
};
use common_services::database::app_user::User;
use tracing::instrument;

#[instrument(skip(context), err(Debug))]
pub async fn job_summary_handler(
    State(context): State<ApiContext>,
    Query(query): Query<JobsQuery>,
) -> Result<Json<PaginatedJobsResponse>, AppError> {
    let overview = get_job_overview(&context.pool, query).await?;
    Ok(Json(overview))
}

#[instrument(skip(context), err(Debug))]
pub async fn cancel_job_handler(
    State(context): State<ApiContext>,
    Path(job_id): Path<i64>,
) -> Result<Json<()>, AppError> {
    cancel_job(&context.pool, job_id).await?;
    Ok(Json(()))
}

#[instrument(skip(context), err(Debug))]
pub async fn retry_job_handler(
    State(context): State<ApiContext>,
    Path(job_id): Path<i64>,
) -> Result<Json<()>, AppError> {
    retry_job(&context.pool, job_id).await?;
    Ok(Json(()))
}

// -- user level jobs handlers

#[instrument(skip(context, user), err(Debug))]
pub async fn ingest_overview_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
) -> Result<Json<IngestOverviewResponse>, AppError> {
    let overview = get_user_ingest_overview(&context.pool, user.id).await?;
    Ok(Json(overview))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_running_ingest_jobs_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<JobInfo>>, AppError> {
    let jobs = get_running_ingest_jobs(&context.pool, user.id).await?;
    Ok(Json(jobs))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_failed_ingest_jobs_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<JobInfo>>, AppError> {
    let jobs = get_failed_ingest_jobs(&context.pool, user.id).await?;
    Ok(Json(jobs))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn scan_user_media_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
) -> Result<Json<()>, AppError> {
    enqueue_scan_job(&context.pool, user.id).await?;
    Ok(Json(()))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn retry_ingest_job_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Json(payload): Json<RetryJobPayload>,
) -> Result<Json<()>, AppError> {
    retry_user_job(&context.pool, payload.id, user.id).await?;
    Ok(Json(()))
}
