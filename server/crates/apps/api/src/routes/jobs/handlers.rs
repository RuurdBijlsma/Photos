use crate::auth::middlewares::user::ApiUser;
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
use sqlx::PgPool;
use tracing::instrument;

#[instrument(skip(pool), err(Debug))]
pub async fn job_summary_handler(
    State(pool): State<PgPool>,
    Query(query): Query<JobsQuery>,
) -> Result<Json<PaginatedJobsResponse>, AppError> {
    let overview = get_job_overview(&pool, query).await?;
    Ok(Json(overview))
}

#[instrument(skip(pool), err(Debug))]
pub async fn cancel_job_handler(
    State(pool): State<PgPool>,
    Path(job_id): Path<i64>,
) -> Result<Json<()>, AppError> {
    cancel_job(&pool, job_id).await?;
    Ok(Json(()))
}

#[instrument(skip(pool), err(Debug))]
pub async fn retry_job_handler(
    State(pool): State<PgPool>,
    Path(job_id): Path<i64>,
) -> Result<Json<()>, AppError> {
    retry_job(&pool, job_id).await?;
    Ok(Json(()))
}

// -- user level jobs handlers

#[instrument(skip(pool, user), err(Debug))]
pub async fn ingest_overview_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<IngestOverviewResponse>, AppError> {
    let overview = get_user_ingest_overview(&pool, user.id).await?;
    Ok(Json(overview))
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn get_running_ingest_jobs_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<Vec<JobInfo>>, AppError> {
    let jobs = get_running_ingest_jobs(&pool, user.id).await?;
    Ok(Json(jobs))
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn get_failed_ingest_jobs_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<Vec<JobInfo>>, AppError> {
    let jobs = get_failed_ingest_jobs(&pool, user.id).await?;
    Ok(Json(jobs))
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn scan_user_media_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<()>, AppError> {
    enqueue_scan_job(&pool, user.id).await?;
    Ok(Json(()))
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn retry_ingest_job_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
    Json(payload): Json<RetryJobPayload>,
) -> Result<Json<()>, AppError> {
    retry_user_job(&pool, payload.id, user.id).await?;
    Ok(Json(()))
}
