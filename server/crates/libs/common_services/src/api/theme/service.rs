use crate::api::app_error::AppError;
use crate::api::theme::interfaces::RandomPhotoResponse;
use crate::database::app_user::User;
use material_color_utils::dynamic::variant::Variant;
use material_color_utils::theme_from_color;
use material_color_utils::utils::color_utils::Argb;
use rand::RngExt;
use sqlx::PgPool;
use tracing::warn;

/// Fetches a random photo with its color theme data for a specific user.
///
/// # Errors
///
/// Returns an error if either of the database queries fail.
pub async fn random_photo_theme(
    user: &User,
    pool: &PgPool,
    variant: Variant,
    contrast_level: f64,
) -> Result<Option<RandomPhotoResponse>, AppError> {
    // Count the total number of photos with associated colour data for the given user.
    let count: i64 = sqlx::query_scalar!(
        r#"
        SELECT COUNT(cd.visual_analysis_id)
        FROM color AS cd
        JOIN visual_analysis AS va ON cd.visual_analysis_id = va.id
        JOIN media_item AS mi ON va.media_item_id = mi.id
        WHERE mi.user_id = $1
          AND mi.deleted = false
          AND cardinality(cd.prominent_colors) > 0
        "#,
        user.id
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0); // Default to 0 if count is NULL

    if count == 0 {
        warn!("No photos with color data for user {}", user.id);
        return Ok(None);
    }

    // Use a thread-safe random number generator to select a random offset.
    let random_offset = rand::rng().random_range(0..count);

    // Fetch a single row from `color_data` using the random offset,
    // along with the associated `media_item_id`.
    let random_data = sqlx::query!(
        r#"
        SELECT
            cd.prominent_colors[1] as "source_color!",
            mi.id as "media_id!"
        FROM color AS cd
        JOIN visual_analysis AS va ON cd.visual_analysis_id = va.id
        JOIN media_item AS mi ON va.media_item_id = mi.id
        WHERE mi.user_id = $1
          AND mi.deleted = false
          AND cardinality(cd.prominent_colors) > 0
        ORDER BY mi.id
        LIMIT 1
        OFFSET $2
        "#,
        user.id,
        random_offset
    )
    .fetch_optional(pool)
    .await?;

    let Some(random_data) = random_data else {
        // This can happen in a race condition if photos are deleted between the COUNT and this query.
        warn!(
            "No photo found at offset {} for user {}",
            random_offset, user.id
        );
        return Ok(None);
    };

    let theme = theme_from_color(Argb::from_hex(&random_data.source_color)?)
        .variant(variant)
        .contrast_level(contrast_level)
        .call();

    Ok(Some(RandomPhotoResponse {
        media_id: random_data.media_id,
        theme,
    }))
}
