use crate::api_state::ApiContext;
use axum::extract::State;
use axum::http::StatusCode;
use tracing::error;

pub async fn root() -> &'static str {
    "Hello, World!"
}

pub async fn health_check(State(context): State<ApiContext>) -> Result<&'static str, StatusCode> {
    match sqlx::query("SELECT 1").fetch_one(&context.pool).await {
        Ok(_) => Ok("OK"),
        Err(e) => {
            error!("Health check failed: database connection error: {}", e);
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}
