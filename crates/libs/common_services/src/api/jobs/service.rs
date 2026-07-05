use crate::api::app_error::AppError;
use crate::api::jobs::interfaces::{JobInfo, JobsQuery, PaginatedJobsResponse};
use crate::api::jobs::query_helpers::{JobSort, apply_filters, parse_filter, parse_sort};
use sqlx::{PgPool, Postgres, QueryBuilder};

pub async fn get_job_overview(
    pool: &PgPool,
    query: JobsQuery,
) -> Result<PaginatedJobsResponse, AppError> {
    // 1. Parse and extract sorts
    let mut sorts = Vec::new();
    for s_str in &query.sort {
        for part in s_str.split(',') {
            let part_trimmed = part.trim();
            if !part_trimmed.is_empty() {
                sorts.push(parse_sort(part_trimmed)?);
            }
        }
    }
    if sorts.is_empty() {
        sorts.push(JobSort {
            column: "id",
            direction: "DESC",
        });
    }

    // 2. Parse and validate filters
    let mut filters = Vec::new();
    for f_str in &query.filter {
        let f_trimmed = f_str.trim();
        if !f_trimmed.is_empty() {
            filters.push(parse_filter(f_trimmed)?);
        }
    }

    // 3. Paginate calculations
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = if let Some(off) = query.offset {
        off.max(0)
    } else if let Some(pg) = query.page {
        ((pg - 1).max(0)) * limit
    } else {
        0
    };

    // 4. Build and execute total count query
    let mut count_builder = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM jobs");
    apply_filters(&mut count_builder, &filters);

    let count_query = count_builder.build_query_scalar::<i64>();
    let total = count_query.fetch_one(pool).await?;

    // 5. Build select query
    let mut select_builder = QueryBuilder::<Postgres>::new(
        "SELECT id, relative_path, user_id, job_type, payload, priority, status, attempts, \
         dependency_attempts, max_attempts, owner, started_at, finished_at, created_at, \
         scheduled_at, last_heartbeat, last_error FROM jobs",
    );
    apply_filters(&mut select_builder, &filters);

    // Apply multi-level sort
    select_builder.push(" ORDER BY ");
    for (i, sort) in sorts.iter().enumerate() {
        if i > 0 {
            select_builder.push(", ");
        }
        select_builder.push(sort.column);
        select_builder.push(" ");
        select_builder.push(sort.direction);
    }

    // Apply pagination limits
    select_builder.push(" LIMIT ");
    select_builder.push_bind(limit);
    select_builder.push(" OFFSET ");
    select_builder.push_bind(offset);

    // Execute select query
    let select_query = select_builder.build_query_as::<JobInfo>();
    let data = select_query.fetch_all(pool).await?;

    Ok(PaginatedJobsResponse {
        data,
        total,
        limit,
        offset,
    })
}

pub async fn cancel_job(pool: &PgPool, job_id: i64) -> Result<(), AppError> {
    let result = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = 'cancelled'::job_status,
            finished_at = NOW()
        WHERE id = $1 AND status IN ('queued'::job_status, 'running'::job_status)
        "#,
        job_id
    )
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest(
            "Job cannot be cancelled (it might not exist, or is already finished/cancelled)"
                .to_owned(),
        ));
    }

    Ok(())
}

pub async fn retry_job(pool: &PgPool, job_id: i64) -> Result<(), AppError> {
    let result = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = 'queued'::job_status,
            attempts = 0,
            scheduled_at = NOW(),
            finished_at = NULL,
            started_at = NULL,
            last_error = NULL,
            owner = NULL
        WHERE id = $1 AND status IN ('failed'::job_status, 'done'::job_status, 'cancelled'::job_status)
        "#,
        job_id
    )
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest(
            "Job cannot be retried (it might not exist, or is currently queued or running)"
                .to_owned(),
        ));
    }

    Ok(())
}
