use crate::database::media_item::camera_settings::CameraSettings;
use crate::database::media_item::gps::Gps;
use crate::database::media_item::location::Location;
use crate::database::media_item::media_features::ReadMediaFeatures;
use crate::database::media_item::media_item::{
    CreateFullMediaItem, FullMediaItem, FullMediaItemRow,
};
use crate::database::media_item::time_details::TimeDetails;
use crate::database::media_item::weather::Weather;
use crate::database::structs::UpdateMediaItemPayload;
use crate::database::visual_analysis::visual_analysis::ReadVisualAnalysis;
use crate::database::{DbError, with_fallback_timezone};
use chrono::{DateTime, Utc};
use common_types::pb::api::{MapPhotoItem, SimpleTimelineItem};
use sqlx::postgres::PgQueryResult;
use sqlx::types::Json;
use sqlx::{Executor, PgTransaction, Postgres};
use std::path::Path;

pub struct MediaItemStore;

impl MediaItemStore {
    pub async fn find_relative_path_by_id(
        executor: impl Executor<'_, Database = Postgres>,
        media_item_id: &str,
    ) -> Result<Option<String>, DbError> {
        Ok(sqlx::query_scalar!(
            r#"
            SELECT relative_path
            FROM media_item
            WHERE id = $1
            "#,
            media_item_id
        )
        .fetch_optional(executor)
        .await?)
    }

    pub async fn find_id_by_relative_path(
        executor: impl Executor<'_, Database = Postgres>,
        relative_path: &str,
    ) -> Result<Option<String>, DbError> {
        Ok(sqlx::query_scalar!(
            r#"
            SELECT id
            FROM media_item
            WHERE relative_path = $1
            "#,
            relative_path
        )
        .fetch_optional(executor)
        .await?)
    }

    pub async fn find_user_by_id(
        executor: impl Executor<'_, Database = Postgres>,
        media_item_id: &str,
    ) -> Result<Option<i32>, DbError> {
        Ok(sqlx::query_scalar!(
            r#"
            SELECT user_id
            FROM media_item
            WHERE id = $1
            "#,
            media_item_id
        )
        .fetch_optional(executor)
        .await?)
    }

    /// Fetches a full media item with all related analyses and metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or the connection pool is invalid.
    #[allow(clippy::too_many_lines)]
    pub async fn find_by_id(
        executor: impl Executor<'_, Database = Postgres>,
        id: &str,
    ) -> Result<Option<FullMediaItem>, DbError> {
        let row_result = sqlx::query_as!(
            FullMediaItemRow,
            r#"
        WITH
        -- Collect all visual analyses and nested data
        visual_analyses AS (
            SELECT
                va.media_item_id,
                jsonb_agg(
                    jsonb_build_object(
                        'id', va.id,
                        'created_at', va.created_at,
                        'percentage', va.percentage,
                        'measured_quality', (
                            SELECT to_jsonb(qd)
                            FROM measured_quality qd WHERE qd.visual_analysis_id = va.id
                        ),
                        'quality_judge', (
                            SELECT to_jsonb(qj)
                            FROM quality_judge qj WHERE qj.visual_analysis_id = va.id
                        ),
                        'colors', (
                            SELECT to_jsonb(cld)
                            FROM color cld WHERE cld.visual_analysis_id = va.id
                        ),
                        'classification', (
                            SELECT to_jsonb(cpd)
                            FROM classification cpd WHERE cpd.visual_analysis_id = va.id
                        ),
                        'faces', (
                            SELECT COALESCE(
                                jsonb_agg(to_jsonb(f)),
                                '[]'::jsonb
                            ) FROM face f WHERE f.visual_analysis_id = va.id
                        ),
                        'objects', (
                            SELECT COALESCE(
                                jsonb_agg(to_jsonb(o)),
                                '[]'::jsonb
                            ) FROM object o WHERE o.visual_analysis_id = va.id
                        )
                    )
                    ORDER BY va.created_at DESC
                ) AS data
            FROM visual_analysis va
            GROUP BY va.media_item_id
        )

        SELECT
            mi.id,
            mi.user_id,
            mi.hash,
            mi.filename,
            mi.relative_path,
            mi.created_at,
            mi.updated_at,
            mi.width,
            mi.height,
            mi.is_video,
            mi.duration_ms,
            mi.taken_at_local,
            mi.taken_at_utc,
            mi.og_taken_at_local,
            mi.timezone_name,
            mi.timezone_offset_seconds,
            mi.og_timezone_offset_seconds,
            mi.use_panorama_viewer,
            mi.has_thumbnails,
            mi.user_caption,

            COALESCE(va.data, '[]'::jsonb) AS "visual_analyses!: Json<Vec<ReadVisualAnalysis>>",

            (SELECT to_jsonb(g)
                    || jsonb_build_object('location',
                        (SELECT to_jsonb(l.*) FROM location l WHERE l.id = g.location_id))
                FROM gps g WHERE g.media_item_id = mi.id
            ) AS "gps: Json<Gps>",

            (SELECT to_jsonb(td) FROM time td WHERE td.media_item_id = mi.id)
                AS "time!: Json<TimeDetails>",

            (SELECT to_jsonb(w) FROM weather w WHERE w.media_item_id = mi.id)
                AS "weather: Json<Weather>",

            (SELECT to_jsonb(d) FROM media_features d WHERE d.media_item_id = mi.id)
                AS "media_features!: Json<ReadMediaFeatures>",

            (SELECT to_jsonb(cd) FROM camera_settings cd WHERE cd.media_item_id = mi.id)
                AS "camera_settings!: Json<CameraSettings>",

            (SELECT to_jsonb(p.config) FROM panorama_config p WHERE p.media_item_id = mi.id)
                AS "panorama_config: Json<serde_json::Value>"

        FROM media_item mi
        LEFT JOIN visual_analyses va ON mi.id = va.media_item_id
        WHERE mi.id = $1 AND mi.deleted = false;
        "#,
            id,
        )
        .fetch_optional(executor)
        .await?;

        Ok(row_result.map(FullMediaItem::from))
    }

    /// Inserts a full media item and all its associated metadata into the database.
    /// This function will first delete any existing media item with the same `relative_path`
    /// to ensure a clean insert.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the database deletion or insertion queries fail.
    #[allow(clippy::too_many_lines)]
    pub async fn create(
        tx: &mut PgTransaction<'_>,
        id: &str,
        relative_path: &str,
        user_id: i32,
        remote_user_id: Option<i32>,
        media_item: &CreateFullMediaItem,
    ) -> Result<(), DbError> {
        let sort_timestamp =
            with_fallback_timezone(media_item.taken_at_utc, &media_item.taken_at_local);
        let filename = Path::new(relative_path).file_name().map_or_else(
            || relative_path.to_string(),
            |f| f.to_string_lossy().to_string(),
        );
        // It's getting stuck here somtimes? Why??

        // Insert into the main media_item table
        sqlx::query!(
            r#"
            INSERT INTO media_item (
                id, relative_path, filename, user_id, remote_user_id, hash, width, height,
                is_video, duration_ms, taken_at_local, taken_at_utc,
                og_taken_at_local, sort_timestamp, orientation,
                use_panorama_viewer, timezone_name, timezone_offset_seconds, og_timezone_offset_seconds
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
            "#,
            id,
            relative_path,
            filename,
            user_id,
            remote_user_id,
            &media_item.hash,
            media_item.width,
            media_item.height,
            media_item.is_video,
            media_item.duration_ms,
            media_item.taken_at_local,
            media_item.taken_at_utc,
            media_item.og_taken_at_local,
            sort_timestamp,
            media_item.orientation,
            media_item.use_panorama_viewer,
            media_item.timezone_name,
            media_item.timezone_offset_seconds,
            media_item.og_timezone_offset_seconds,
        )
            .execute(&mut **tx)
            .await?;

        // Insert into related tables
        if let Some(gps_info) = &media_item.gps {
            let location_id = Self::get_or_create_location(tx, &gps_info.location).await?;

            sqlx::query!(
                r#"
                INSERT INTO gps (media_item_id, location_id, latitude, longitude, altitude, compass_direction)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                id,
                location_id,
                gps_info.latitude,
                gps_info.longitude,
                gps_info.altitude,
                gps_info.compass_direction,
            )
                .execute(&mut **tx)
                .await?;
        }

        if let Some(weather_info) = &media_item.weather {
            sqlx::query!(
                r#"
                INSERT INTO weather (
                    media_item_id, temperature, dew_point, relative_humidity, precipitation, snow,
                    wind_direction, wind_speed, peak_wind_gust, pressure, sunshine_minutes,
                    condition, sunrise, sunset, dawn, dusk, is_daytime
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
                "#,
                id,
                weather_info.temperature,
                weather_info.dew_point,
                weather_info.relative_humidity,
                weather_info.precipitation,
                weather_info.snow,
                weather_info.wind_direction,
                weather_info.wind_speed,
                weather_info.peak_wind_gust,
                weather_info.pressure,
                weather_info.sunshine_minutes,
                weather_info.condition,
                weather_info.sunrise,
                weather_info.sunset,
                weather_info.dawn,
                weather_info.dusk,
                weather_info.is_daytime,
            )
            .execute(&mut **tx)
            .await?;
        }

        sqlx::query!(
            r#"
                INSERT INTO time (
                    media_item_id, timezone_source, source_details, source_confidence
                )
                VALUES ($1, $2, $3, $4)
                "#,
            id,
            media_item.time.timezone_source,
            &media_item.time.source_details,
            &media_item.time.source_confidence,
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
                INSERT INTO media_features (
                    media_item_id, mime_type, size_bytes, is_motion_photo,
                    motion_photo_presentation_timestamp, is_hdr, is_burst, burst_id,
                    capture_fps, video_fps, is_nightsight, is_timelapse, exif, audio_format,
                    audio_channels, audio_sample_rate, compressor_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
                "#,
            id,
            media_item.media_features.mime_type,
            media_item.media_features.size_bytes,
            media_item.media_features.is_motion_photo,
            media_item
                .media_features
                .motion_photo_presentation_timestamp,
            media_item.media_features.is_hdr,
            media_item.media_features.is_burst,
            media_item.media_features.burst_id,
            media_item.media_features.capture_fps,
            media_item.media_features.video_fps,
            media_item.media_features.is_nightsight,
            media_item.media_features.is_timelapse,
            media_item.media_features.exif,
            media_item.media_features.audio_format,
            media_item.media_features.audio_channels.map(|i| i as i32),
            media_item
                .media_features
                .audio_sample_rate
                .map(|i| i as i32),
            media_item.media_features.compressor_id,
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
                INSERT INTO camera_settings (
                    media_item_id, iso, exposure_time, aperture, focal_length, camera_make,
                    camera_model, flash_fired, flash_mode, lens_make, lens_model,
                    digital_zoom_ratio, subject_distance, focal_length_in_35mm,
                    exposure_compensation
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                "#,
            id,
            media_item.camera_settings.iso,
            media_item.camera_settings.exposure_time,
            media_item.camera_settings.aperture,
            media_item.camera_settings.focal_length,
            media_item.camera_settings.camera_make,
            media_item.camera_settings.camera_model,
            media_item.camera_settings.flash_fired,
            media_item.camera_settings.flash_mode,
            media_item.camera_settings.lens_make,
            media_item.camera_settings.lens_model,
            media_item.camera_settings.digital_zoom_ratio,
            media_item.camera_settings.subject_distance,
            media_item.camera_settings.focal_length_in_35mm,
            media_item.camera_settings.exposure_compensation,
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Deletes a media item by its relative path and returns the ID of the deleted item.
    /// Database cascade rules are expected to clean up related data.
    pub async fn delete_by_relative_path(
        executor: impl Executor<'_, Database = Postgres>,
        relative_path: &str,
    ) -> Result<Option<String>, DbError> {
        Ok(sqlx::query_scalar!(
            r#"
            DELETE FROM media_item
            WHERE relative_path = $1
            RETURNING id
            "#,
            relative_path
        )
        .fetch_optional(executor)
        .await?)
    }

    /// Deletes multiple media items by their relative paths.
    pub async fn delete_by_relative_paths(
        executor: impl Executor<'_, Database = Postgres>,
        relative_paths: &[String],
    ) -> Result<PgQueryResult, DbError> {
        Ok(sqlx::query!(
            r#"
            DELETE FROM media_item
            WHERE relative_path = ANY($1)
            "#,
            relative_paths
        )
        .execute(executor)
        .await?)
    }

    /// Marks a media item as deleted without removing it from the database.
    pub async fn soft_delete_by_relative_path(
        executor: impl Executor<'_, Database = Postgres>,
        relative_path: &str,
    ) -> Result<Option<String>, DbError> {
        Ok(sqlx::query_scalar!(
            r#"
            UPDATE media_item
            SET deleted = true
            WHERE relative_path = $1
            RETURNING id
            "#,
            relative_path
        )
        .fetch_optional(executor)
        .await?)
    }

    pub async fn update(
        executor: impl Executor<'_, Database = Postgres>,
        id: &str,
        payload: UpdateMediaItemPayload,
    ) -> Result<PgQueryResult, DbError> {
        Ok(sqlx::query!(
            r#"
        UPDATE media_item
        SET
            user_caption = CASE WHEN $2::boolean THEN user_caption ELSE $3 END,
            taken_at_local = COALESCE($4, taken_at_local),
            taken_at_utc = CASE WHEN $5::boolean THEN taken_at_utc ELSE $6 END,
            sort_timestamp = COALESCE($7, sort_timestamp),
            timezone_offset_seconds = CASE WHEN $8::boolean THEN timezone_offset_seconds ELSE $9 END,
            use_panorama_viewer = COALESCE($10, use_panorama_viewer),
            updated_at = now()
        WHERE id = $1
        "#,
            id,
            payload.user_caption.is_ignore(),
            payload.user_caption.value(),
            payload.taken_at_local,
            payload.taken_at_utc.is_ignore(),
            payload.taken_at_utc.value(),
            payload.sort_timestamp,
            payload.timezone_offset_seconds.is_ignore(),
            payload.timezone_offset_seconds.value(),
            payload.use_panorama_viewer,
        )
            .execute(executor)
            .await?)
    }

    pub async fn update_remote_user_id(
        executor: impl Executor<'_, Database = Postgres>,
        id: &str,
        remote_user_id: i32,
    ) -> Result<PgQueryResult, DbError> {
        Ok(sqlx::query!(
            r#"
            UPDATE media_item
            SET remote_user_id = $1
            WHERE id = $2
            "#,
            remote_user_id,
            id
        )
        .execute(executor)
        .await?)
    }

    /// Retrieves an existing location's ID or creates a new one if it doesn't exist.
    async fn get_or_create_location(
        tx: &mut PgTransaction<'_>,
        location_data: &Location,
    ) -> Result<i32, DbError> {
        //todo: can this be done in 1 query? is better?
        let existing_id: Option<i32> = sqlx::query_scalar!(
            "SELECT id FROM location WHERE name = $1 AND admin1 = $2 AND country_code = $3",
            &location_data.name,
            &location_data.admin1,
            &location_data.country_code,
        )
        .fetch_optional(&mut **tx)
        .await?;

        if let Some(id) = existing_id {
            Ok(id)
        } else {
            let new_id: i32 = sqlx::query_scalar!(
                r#"
                INSERT INTO location (name, admin1, admin2, country_code, country_name)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id
                "#,
                &location_data.name,
                &location_data.admin1,
                &location_data.admin2,
                &location_data.country_code,
                &location_data.country_name,
            )
            .fetch_one(&mut **tx)
            .await?;
            Ok(new_id)
        }
    }

    pub async fn find_all_geo_by_user_id(
        executor: impl Executor<'_, Database = Postgres>,
        user_id: i32,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<Vec<MapPhotoItem>, DbError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                mi.id,
                mi.is_video,
                mi.has_thumbnails,
                mi.duration_ms::INT as duration_ms,
                (mi.width::real / mi.height::real) as "ratio!",
                g.latitude,
                g.longitude
            FROM media_item mi
            JOIN gps g ON mi.id = g.media_item_id
            WHERE mi.user_id = $1 AND mi.deleted = false
              AND ($2::TIMESTAMPTZ IS NULL OR mi.sort_timestamp >= $2)
              AND ($3::TIMESTAMPTZ IS NULL OR mi.sort_timestamp <= $3)
            ORDER BY mi.sort_timestamp DESC
            "#,
            user_id,
            start_date,
            end_date
        )
        .fetch_all(executor)
        .await?;

        let items = rows
            .into_iter()
            .map(|r| MapPhotoItem {
                latitude: r.latitude,
                longitude: r.longitude,
                item: Some(SimpleTimelineItem {
                    id: r.id,
                    is_video: r.is_video,
                    has_thumbnails: r.has_thumbnails,
                    duration_ms: r.duration_ms,
                    ratio: r.ratio,
                }),
            })
            .collect();

        Ok(items)
    }

    pub async fn upsert_panorama_config(
        executor: impl Executor<'_, Database = Postgres>,
        media_item_id: &str,
        config: &serde_json::Value,
    ) -> Result<(), DbError> {
        sqlx::query!(
            r#"
            INSERT INTO panorama_config (media_item_id, config)
            VALUES ($1, $2)
            ON CONFLICT (media_item_id)
            DO UPDATE SET config = EXCLUDED.config
            "#,
            media_item_id,
            config
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
