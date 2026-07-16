use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
// Change: Remove Query from axum::extract, and import Query from axum_extra::extract
use axum::extract::State;
use axum_extra::extract::Query;
use axum::{Extension, Json};
use common_services::api::app_error::AppError;
use common_services::api::explore::interfaces::{ExploreTableQuery, PaginatedExploreTableResponse};
use common_services::api::explore::service::get_explore_table;
use tracing::instrument;

#[instrument(skip(context, user), err(Debug))]
pub async fn get_explore_table_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Query(query): Query<ExploreTableQuery>,
) -> Result<Json<PaginatedExploreTableResponse>, AppError> {
    let result = get_explore_table(&context.pool, user.id, query).await?;
    Ok(Json(result))
}