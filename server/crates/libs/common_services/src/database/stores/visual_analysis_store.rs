use crate::database::DbError;
use crate::database::visual_analysis::visual_analysis::CreateVisualAnalysis;
use sqlx::PgTransaction;

pub struct VisualAnalysisStore;

impl VisualAnalysisStore {
    /// Stores the detailed results of a visual analysis for a media item in the database.
    ///
    /// This function takes a single `VisualImageData` object and persists it, including all nested
    /// data like faces, objects, and color information. It returns the ID of the newly created
    /// `visual_analysis` record.
    ///
    /// # Errors
    ///
    /// This function will return an error if any of the database insertion queries fail
    /// or if the color histogram data cannot be serialized to JSON.
    #[allow(clippy::too_many_lines)]
    pub async fn create_fast_analysis(
        tx: &mut PgTransaction<'_>,
        media_item_id: &str,
        user_id: i32,
        analysis: &CreateVisualAnalysis,
    ) -> Result<i64, DbError> {
        // Insert the main analysis record and get its new ID.
        let visual_analysis_id: i64 = sqlx::query_scalar!(
            r"
            INSERT INTO visual_analysis (media_item_id, user_id, embedding, percentage)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            ",
            media_item_id,
            user_id,
            analysis.embedding as _,
            analysis.percentage,
        )
        .fetch_one(&mut **tx)
        .await?;

        // --- Face Data ---
        for face in &analysis.faces {
            sqlx::query!(
                r#"
                INSERT INTO face (
                    visual_analysis_id, position_x, position_y, width, height, confidence, age, sex,
                    embedding
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
                visual_analysis_id,
                face.position_x,
                face.position_y,
                face.width,
                face.height,
                face.confidence,
                face.age,
                &face.sex,
                face.embedding as _,
            )
            .execute(&mut **tx)
            .await?;
        }

        // --- Object Data ---
        for object in &analysis.objects {
            sqlx::query!(
                r#"
                INSERT INTO object (
                    visual_analysis_id, position_x, position_y, width, height, confidence, tag
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
                visual_analysis_id,
                object.position_x,
                object.position_y,
                object.width,
                object.height,
                object.confidence,
                object.tag,
            )
            .execute(&mut **tx)
            .await?;
        }

        // --- Color Data ---
        let color = &analysis.colors;
        let histogram_json = serde_json::to_value(&color.histogram)?;

        sqlx::query!(
            r#"
            INSERT INTO color (
                visual_analysis_id, prominent_colors,
                average_hue, average_saturation, average_lightness, histogram
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            visual_analysis_id,
            &color.prominent_colors,
            color.average_hue,
            color.average_saturation,
            color.average_lightness,
            histogram_json,
        )
        .execute(&mut **tx)
        .await?;

        // --- Quality Data ---
        let quality = &analysis.quality;

        sqlx::query!(
            r#"
            INSERT INTO measured_quality (
                visual_analysis_id, blurriness, noisiness, exposure, accidentalness,
                weighted_score
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            visual_analysis_id,
            quality.blurriness,
            quality.noisiness,
            quality.exposure,
            quality.accidentalness,
            quality.weighted_score,
        )
        .execute(&mut **tx)
        .await?;

        Ok(visual_analysis_id)
    }
}
