use app_state::IngestSettings;
use axum::extract::{Query, State};
use axum::{Extension, Json};

use common_services::database::app_user::User;

use crate::api_state::ApiContext;
use common_services::api::app_error::AppError;
use common_services::api::theme::interfaces::{
    ColorThemeParams, RandomPhotoParams, RandomPhotoResponse,
};
use common_services::api::theme::service::random_photo_theme;
use material_color_utils::dynamic::variant::Variant;
use material_color_utils::utils::color_utils::Argb;
use material_color_utils::{MaterializedTheme, theme_from_color};

pub async fn get_random_photo_theme(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Query(params): Query<RandomPhotoParams>,
) -> Result<Json<Option<RandomPhotoResponse>>, AppError> {
    let variant: Variant = serde_json::from_str(&format!("\"{}\"", params.variant))
        .unwrap_or(context.settings.ingest.analyzer.theme_generation.variant);
    let result = random_photo_theme(&user, &context.pool, variant, params.contrast).await?;
    Ok(Json(result))
}

pub async fn get_color_theme_handler(
    State(ingestion): State<IngestSettings>,
    Query(params): Query<ColorThemeParams>,
) -> Result<Json<MaterializedTheme>, AppError> {
    let variant: Variant = serde_json::from_str(&format!("\"{}\"", params.variant))
        .unwrap_or(ingestion.analyzer.theme_generation.variant);
    let theme = theme_from_color(Argb::from_hex(&params.color)?)
        .variant(variant)
        .contrast_level(params.contrast)
        .call();
    Ok(Json(theme))
}
