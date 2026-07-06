use crate::api_state::ApiContext;
use app_state::constants::FACE_CLUSTERS_FOLDER;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Redirect};
use axum::{Extension, Json};
use axum_extra::protobuf::Protobuf;
use common_services::api::app_error::AppError;
use common_services::api::people::interfaces::{MergePersonRequest, UpdatePersonRequest};
use common_services::api::people::service::{
    get_all_people, get_person_photos, merge_person, unmerge_person, update_person,
};
use common_services::database::app_user::User;
use common_types::pb::api::{FullPersonMediaResponse, ListPeopleResponse};
use http::header::CACHE_CONTROL;
use tracing::instrument;

#[instrument(skip(context, user), err(Debug))]
pub async fn list_people_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
) -> Result<Protobuf<ListPeopleResponse>, AppError> {
    let result = get_all_people(&context.pool, user.id).await?;
    Ok(Protobuf(result))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn update_person_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(person_id): Path<String>,
    Json(payload): Json<UpdatePersonRequest>,
) -> Result<(), AppError> {
    update_person(&context.pool, &person_id, user.id, &payload).await?;
    Ok(())
}

#[instrument(skip(context, user), err(Debug))]
pub async fn merge_person_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(person_id): Path<String>,
    Json(payload): Json<MergePersonRequest>,
) -> Result<(), AppError> {
    merge_person(&context.pool, &person_id, user.id, &payload).await?;
    Ok(())
}

#[instrument(skip(context, user), err(Debug))]
pub async fn unmerge_person_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(person_id): Path<String>,
) -> Result<(), AppError> {
    unmerge_person(&context.pool, &person_id, user.id).await?;
    Ok(())
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_person_photos_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(person_id): Path<String>,
) -> Result<Protobuf<FullPersonMediaResponse>, AppError> {
    let result = get_person_photos(&context.pool, &person_id, user.id).await?;
    Ok(Protobuf(result))
}

pub async fn get_person_thumbnail_redirect_handler(
    State(context): State<ApiContext>,
    Path(person_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let cluster_id = if let Some(db_face) =
        sqlx::query_scalar!("SELECT face_thumb_id FROM person WHERE id = $1", person_id)
            .fetch_one(&context.pool)
            .await?
    {
        db_face
    } else {
        sqlx::query_scalar!(
            "SELECT id FROM face_cluster WHERE person_id = $1",
            person_id
        )
        .fetch_one(&context.pool)
        .await?
    };
    let target_url = format!("/{FACE_CLUSTERS_FOLDER}/{cluster_id}.webp");
    let headers = [(CACHE_CONTROL, "public, max-age=300")];
    Ok((headers, Redirect::temporary(&target_url)))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_person_media_item_id(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(person_id): Path<String>,
) -> Result<Json<String>, AppError> {
    let Some(result) = sqlx::query_scalar!(
        "SELECT thumb_media_item_id FROM face_cluster WHERE person_id = $1 AND user_id = $2 AND thumb_media_item_id IS NOT NULL",
        person_id, user.id
    )
        .fetch_one(&context.pool)
        .await? else {
        return Err(AppError::NotFound(person_id));
    };
    Ok(Json(result))
}
