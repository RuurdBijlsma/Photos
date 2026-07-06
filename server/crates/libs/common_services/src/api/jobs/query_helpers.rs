use crate::api::app_error::AppError;
use sqlx::{Postgres, QueryBuilder};

#[derive(Debug, Clone)]
pub struct JobFilter {
    pub column: &'static str,
    pub operator: &'static str,
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JobSort {
    pub column: &'static str,
    pub direction: &'static str,
}

/// Maps safe API fields (either camelCase or `snake_case`) to valid DB columns.
/// Serves as an allowlist against SQL injection.
#[must_use]
pub fn map_field_to_column(field: &str) -> Option<&'static str> {
    match field {
        "id" => Some("id"),
        "relative_path" | "relativePath" => Some("relative_path"),
        "user_id" | "userId" => Some("user_id"),
        "job_type" | "jobType" => Some("job_type"),
        "payload" => Some("payload"),
        "priority" => Some("priority"),
        "status" => Some("status"),
        "attempts" => Some("attempts"),
        "dependency_attempts" | "dependencyAttempts" => Some("dependency_attempts"),
        "max_attempts" | "maxAttempts" => Some("max_attempts"),
        "owner" => Some("owner"),
        "started_at" | "startedAt" => Some("started_at"),
        "finished_at" | "finishedAt" => Some("finished_at"),
        "created_at" | "createdAt" => Some("created_at"),
        "scheduled_at" | "scheduledAt" => Some("scheduled_at"),
        "last_heartbeat" | "lastHeartbeat" => Some("last_heartbeat"),
        "last_error" | "lastError" => Some("last_error"),
        _ => None,
    }
}

/// Provides correct SQL types for Postgres type casting on parameter binds.
#[must_use]
pub fn get_column_cast_suffix(column: &str) -> &'static str {
    match column {
        "id" => "::bigint",
        "relative_path" | "owner" | "last_error" => "::text",
        "user_id" | "priority" | "attempts" | "dependency_attempts" | "max_attempts" => "::integer",
        "job_type" => "::job_type",
        "payload" => "::jsonb",
        "status" => "::job_status",
        "started_at" | "finished_at" | "created_at" | "scheduled_at" | "last_heartbeat" => {
            "::timestamp with time zone"
        }
        _ => "",
    }
}

pub fn parse_filter(filter_str: &str) -> Result<JobFilter, AppError> {
    let parts: Vec<&str> = filter_str.splitn(3, ':').collect();
    if parts.is_empty() {
        return Err(AppError::BadRequest("Filter cannot be empty".to_owned()));
    }

    let raw_field = parts[0];
    let column = map_field_to_column(raw_field)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid filter field: {raw_field}")))?;

    if parts.len() == 1 {
        return Err(AppError::BadRequest(format!(
            "Filter '{filter_str}' is missing an operator"
        )));
    }

    let raw_op = parts[1].to_lowercase();
    let (operator, needs_value) = match raw_op.as_str() {
        "eq" | "equals" | "==" => ("=", true),
        "neq" | "notequals" | "!=" => ("!=", true),
        "gt" | ">" => (">", true),
        "gte" | ">=" => (">=", true),
        "lt" | "<" => ("<", true),
        "lte" | "<=" => ("<=", true),
        "contains" | "like" => ("ILIKE", true),
        "isnull" | "is_null" => ("IS NULL", false),
        "isnotnull" | "is_not_null" => ("IS NOT NULL", false),
        _ => {
            return Err(AppError::BadRequest(format!(
                "Invalid filter operator: {raw_op}"
            )));
        }
    };

    let value = if needs_value {
        if parts.len() < 3 {
            return Err(AppError::BadRequest(format!(
                "Filter '{filter_str}' is missing a value"
            )));
        }
        let mut val = parts[2].to_string();
        if operator == "ILIKE" {
            val = format!("%{val}%");
        }
        Some(val)
    } else {
        None
    };

    Ok(JobFilter {
        column,
        operator,
        value,
    })
}

pub fn parse_sort(sort_str: &str) -> Result<JobSort, AppError> {
    let parts: Vec<&str> = sort_str.splitn(2, ':').collect();
    if parts.is_empty() {
        return Err(AppError::BadRequest("Sort cannot be empty".to_owned()));
    }

    let raw_field = parts[0];
    let column = map_field_to_column(raw_field)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid sort field: {raw_field}")))?;

    let direction = if parts.len() == 2 {
        let dir = parts[1].to_lowercase();
        if dir == "desc" || dir == "descending" {
            "DESC"
        } else if dir == "asc" || dir == "ascending" {
            "ASC"
        } else {
            return Err(AppError::BadRequest(format!(
                "Invalid sort direction '{dir}'. Must be 'asc' or 'desc'"
            )));
        }
    } else {
        "ASC"
    };

    Ok(JobSort { column, direction })
}

pub fn apply_filters(builder: &mut QueryBuilder<'_, Postgres>, filters: &[JobFilter]) {
    if !filters.is_empty() {
        builder.push(" WHERE ");
        for (i, filter) in filters.iter().enumerate() {
            if i > 0 {
                builder.push(" AND ");
            }
            builder.push(filter.column);
            builder.push(" ");
            builder.push(filter.operator);
            if let Some(ref val) = filter.value {
                builder.push(" ");
                let cast_suffix = get_column_cast_suffix(filter.column);
                builder.push_bind(val.clone());
                builder.push(cast_suffix);
            }
        }
    }
}
