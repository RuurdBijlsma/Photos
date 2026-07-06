use crate::database::DbError;
use crate::database::visual_analysis::caption_data::ClassificationData;
use crate::database::visual_analysis::quality_judge::QualityJudge;
use crate::database::visual_analysis::visual_analysis::CreateVisualAnalysis;
use sqlx::PgTransaction;

pub struct VisualAnalysisStore;

impl VisualAnalysisStore {
    pub async fn add_llm_analysis(
        tx: &mut PgTransaction<'_>,
        visual_analysis_id: i64,
        classification: &ClassificationData,
        quality: Option<QualityJudge>,
    ) -> Result<i64, DbError> {
        // --- Quality Data ---
        if let Some(quality) = quality {
            sqlx::query!(
                r#"
            INSERT INTO quality_judge (
                visual_analysis_id,
                exposure,
                contrast,
                sharpness,
                color_accuracy,
                composition,
                subject_clarity,
                visual_impact,
                creativity,
                color_harmony,
                storytelling,
                style_suitability,
                weighted_score
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
                visual_analysis_id,
                i16::from(quality.exposure),
                i16::from(quality.contrast),
                i16::from(quality.sharpness),
                i16::from(quality.color_accuracy),
                i16::from(quality.composition),
                i16::from(quality.subject_clarity),
                i16::from(quality.visual_impact),
                i16::from(quality.creativity),
                i16::from(quality.color_harmony),
                i16::from(quality.storytelling),
                i16::from(quality.style_suitability),
                quality.weighted_score
            )
            .execute(&mut **tx)
            .await?;
        }

        // --- Classification Data ---
        sqlx::query!(
            r#"
            INSERT INTO classification (
                visual_analysis_id, caption, main_subject, search_term, contains_pets, contains_vehicle,
                contains_landmarks, contains_people, contains_animals, contains_text, is_indoor, is_food, is_drink,
                is_event, is_document, is_landscape, is_cityscape, is_activity, setting,
                animal_type, food_name, drink_name, vehicle_type, event_type, landmark_name, ocr_text,
                document_type, people_count, people_mood, photo_type, activity_description
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31
            )
            "#,
            visual_analysis_id,
            classification.caption,
            classification.main_subject,
            classification.search_term,
            classification.contains_pets,
            classification.contains_vehicle,
            classification.contains_landmarks,
            classification.contains_people,
            classification.contains_animals,
            classification.contains_text,
            classification.is_indoor,
            classification.is_food,
            classification.is_drink,
            classification.is_event,
            classification.is_document,
            classification.is_landscape,
            classification.is_cityscape,
            classification.is_activity,
            classification.setting,
            classification.animal_type,
            classification.food_name,
            classification.drink_name,
            classification.vehicle_type,
            classification.event_type,
            classification.landmark_name,
            classification.ocr_text,
            classification.document_type,
            classification.people_count,
            classification.people_mood,
            classification.photo_type,
            classification.activity_description,
        )
            .execute(&mut **tx)
            .await?;

        Ok(visual_analysis_id)
    }

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
