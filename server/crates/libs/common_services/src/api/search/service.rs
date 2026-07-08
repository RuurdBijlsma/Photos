use crate::api::app_error::AppError;
use crate::api::search::cache::{get_cached_image_embedding, get_cached_text_embedding};
use crate::api::search::interfaces::{
    SearchFilterRanges, SearchImage, SearchMediaConfig, SearchMediaType, SearchSortBy, VisionQuery,
};
use crate::api::search::search_variants::{
    advanced_search_media, basic_search_media, filter_only_search_media,
};
use crate::database::app_user::User;
use color_eyre::eyre::eyre;
use common_types::pb::api::{
    SearchSuggestion, SearchSuggestionsResponse, SimpleTimelineItem, SuggestionType,
};
use open_clip_inference::{TextEmbedder, VisionEmbedder};
use pgvector::Vector;
use sqlx::PgPool;
use std::sync::Arc;

pub async fn search_media(
    user: &User,
    pool: &PgPool,
    embedder: Arc<TextEmbedder>,
    query: Option<String>,
    config: SearchMediaConfig,
) -> Result<Vec<SimpleTimelineItem>, AppError> {
    let query = query.unwrap_or_default();
    if query.trim().is_empty() {
        if has_active_filters(&config) {
            return filter_only_search_media(user, pool, config).await;
        }
        return Ok(vec![]);
    }

    if config.media_type == SearchMediaType::All
        && config.sort_by == SearchSortBy::Relevancy
        && config.start_date.is_none()
        && config.end_date.is_none()
        && config.negative_query.is_none()
        && config.person_ids.is_empty()
        && config.country_codes.is_empty()
    {
        basic_search_media(user, pool, embedder, &query, config).await
    } else {
        advanced_search_media(user, pool, embedder, &query, config).await
    }
}

pub async fn search_by_media_items(
    user: &User,
    pool: &PgPool,
    text_embedder: Arc<TextEmbedder>,
    vision_embedder: Arc<VisionEmbedder>,
    query: Option<String>,
    config: SearchMediaConfig,
    media_item_ids: &[String],
) -> Result<Vec<SimpleTimelineItem>, AppError> {
    if media_item_ids.is_empty() {
        return Ok(vec![]);
    }

    // Query the database to calculate the average embedding vector of the specified media items
    let row = sqlx::query!(
        r#"
        SELECT AVG(embedding)::vector as "embedding: Vector"
        FROM visual_analysis
        WHERE user_id = $1 AND media_item_id = ANY($2) AND deleted = false
        "#,
        user.id,
        media_item_ids
    )
    .fetch_one(pool)
    .await?;

    let avg_embedding = row
        .embedding
        .ok_or_else(|| {
            AppError::Internal(eyre!("No embeddings found for the specified media items"))
        })?
        .to_vec();

    // Perform search using the combined average vector
    search_by_image(
        user,
        pool,
        text_embedder,
        vision_embedder,
        query,
        VisionQuery::Embedding(avg_embedding),
        config,
    )
    .await
}

#[allow(clippy::too_many_lines)]
pub async fn search_by_image(
    user: &User,
    pool: &PgPool,
    text_embedder: Arc<TextEmbedder>,
    vision_embedder: Arc<VisionEmbedder>,
    query: Option<String>,
    img: VisionQuery,
    config: SearchMediaConfig,
) -> Result<Vec<SimpleTimelineItem>, AppError> {
    // 1. Spawn vision embedding (CPU-bound / blocking task)
    let pool_clone = pool.clone();
    let model_id_clone = config.embedder_model_id.clone();
    let image_task = tokio::spawn(async move {
        match img {
            VisionQuery::Raw(img) => {
                get_cached_image_embedding(img, &model_id_clone, &pool_clone, vision_embedder).await
            }
            VisionQuery::Embedding(vector) => Ok(vector),
        }
    });

    // 2. Spawn optional text query embedding task
    let text_task = if let Some(ref q) = query {
        let q_clone = q.clone();
        let text_embedder_clone = text_embedder.clone();
        let pool_clone = pool.clone();
        let model_id_clone = config.embedder_model_id.clone();
        Some(tokio::spawn(async move {
            get_cached_text_embedding(&q_clone, &model_id_clone, &pool_clone, text_embedder_clone)
                .await
        }))
    } else {
        None
    };

    // 3. Spawn optional negative query embedding task
    let negative_task = if let Some(ref neg_q) = config.negative_query {
        let neg_q_clone = neg_q.clone();
        let text_embedder_clone = text_embedder.clone();
        let pool_clone = pool.clone();
        let model_id_clone = config.embedder_model_id.clone();
        Some(tokio::spawn(async move {
            get_cached_text_embedding(
                &neg_q_clone,
                &model_id_clone,
                &pool_clone,
                text_embedder_clone,
            )
            .await
        }))
    } else {
        None
    };

    // Await all embedding calculations
    let mut final_embedding = image_task.await??;

    if let Some(task) = text_task {
        let text_emb = task.await??;
        // Average text and image embedding
        for (img_val, text_val) in final_embedding.iter_mut().zip(text_emb.iter()) {
            *img_val = f32::midpoint(*img_val, *text_val);
        }
    }

    if let Some(task) = negative_task {
        let neg_emb = task.await??;
        // Subtract negative text embedding
        for (pos_val, neg_val) in final_embedding.iter_mut().zip(neg_emb.iter()) {
            *pos_val = 0.5f32.mul_add(-*neg_val, *pos_val);
        }
    }

    // Re-normalize the combined vector to unit length
    let norm = final_embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-6 {
        for val in &mut final_embedding {
            *val /= norm;
        }
    }

    let vector_param = Vector::from(final_embedding);
    let limit = config.limit.unwrap_or(100).min(500);
    let offset = config.offset.unwrap_or(0);
    let candidate_limit = limit * 3 + 300;
    let k = 60.0f64;

    let (is_video_filter, is_panorama_filter) = match config.media_type {
        SearchMediaType::Video => (Some(true), Some(false)),
        SearchMediaType::Photo => (Some(false), Some(false)),
        SearchMediaType::Panorama => (Some(false), Some(true)),
        SearchMediaType::All => (None, None),
    };

    let semantic_score_threshold = if config.sort_by == SearchSortBy::Relevancy {
        2.0
    } else {
        config.semantic_score_threshold
    };

    let sort_by_str = match config.sort_by {
        SearchSortBy::Relevancy => "relevancy",
        SearchSortBy::Date => "date",
    };

    if let Some(q) = query {
        // --- PATHWAY A: Text + Image query ---
        let fts_query = if let Some(ref negative_query) = config.negative_query {
            let neg_terms: Vec<String> = negative_query
                .split_whitespace()
                .map(|s| format!("-{s}"))
                .collect();
            format!("{} {}", q, neg_terms.join(" "))
        } else {
            q
        };

        let items = sqlx::query_as!(
            SimpleTimelineItem,
            r#"
        WITH
        filtered_media AS MATERIALIZED (
            SELECT mi.id, mi.search_vector
            FROM media_item mi
            WHERE mi.user_id = $2
              AND mi.deleted = false
              AND ($9::timestamptz IS NULL OR mi.taken_at_utc >= $9)
              AND ($10::timestamptz IS NULL OR mi.taken_at_utc <= $10)
              AND ($11::bool IS NULL OR mi.is_video = $11)
              AND ($18::bool IS NULL OR mi.use_panorama_viewer = $18)
              AND (cardinality($12::text[]) = 0 OR EXISTS (
                  SELECT 1 FROM gps g JOIN location l ON g.location_id = l.id
                  WHERE g.media_item_id = mi.id AND l.country_code = ANY($12)
              ))
              AND (cardinality($13::text[]) = 0 OR (
                  SELECT COUNT(DISTINCT p.id)
                  FROM visual_analysis va
                  JOIN face f ON f.visual_analysis_id = va.id
                  JOIN face_cluster fc ON f.face_cluster_id = fc.id
                  JOIN person p ON fc.person_id = p.id
                  WHERE va.media_item_id = mi.id AND p.id = ANY($13)
              ) >= (CASE WHEN $16 THEN cardinality($13) ELSE 1 END))
        ),
        fts AS (
            SELECT
                fm.id,
                ts_rank_cd(fm.search_vector, websearch_to_tsquery('english', $1)) as score,
                ROW_NUMBER() OVER (ORDER BY ts_rank_cd(fm.search_vector, websearch_to_tsquery('english', $1)) DESC) as rank
            FROM filtered_media fm
            WHERE fm.search_vector @@ websearch_to_tsquery('english', $1)
            LIMIT $4
        ),
        vec AS (
            SELECT
                id,
                1 - distance as score,
                ROW_NUMBER() OVER (ORDER BY distance) as rank
            FROM (
                SELECT DISTINCT ON (media_item_id)
                    media_item_id as id,
                    distance
                FROM (
                    SELECT va.media_item_id, va.embedding <=> $3::vector as distance
                    FROM visual_analysis va
                    WHERE va.user_id = $2
                      AND va.deleted = false
                      AND (va.embedding <=> $3::vector) < $15
                      AND EXISTS (
                          SELECT 1 FROM filtered_media fm
                          WHERE fm.id = va.media_item_id
                      )
                    ORDER BY va.embedding <=> $3::vector
                    LIMIT $4 * 5
                ) sub_ordered
                ORDER BY media_item_id, distance
            ) sub_unique
            ORDER BY distance
            LIMIT $4
        ),
        merged AS (
            SELECT id, rank, 1 as is_fts, 0 as is_vec FROM fts
            UNION ALL
            SELECT id, rank, 0 as is_fts, 1 as is_vec FROM vec
        ),
        scored_candidates AS (
            SELECT
                id,
                SUM(
                    CASE
                        WHEN is_fts = 1 THEN $7::float8 / ($6::float8 + rank::float8)
                        WHEN is_vec = 1 THEN $8::float8 / ($6::float8 + rank::float8)
                        ELSE 0
                    END
                )::real as combined_score
            FROM merged
            GROUP BY id
        )
        SELECT
            mi.id::text as "id!",
            mi.is_video as "is_video!",
            mi.has_thumbnails as "has_thumbnails!",
            mi.duration_ms as "duration_ms: i32",
            (mi.width::real / mi.height::real) as "ratio!"
        FROM scored_candidates sc
        JOIN media_item mi ON mi.id = sc.id
        ORDER BY
            (CASE WHEN $14 = 'date' THEN NULL ELSE sc.combined_score END) DESC NULLS LAST,
            mi.sort_timestamp DESC
        LIMIT $5 OFFSET $17
             "#,
            fts_query,                // $1
            user.id,                  // $2
            vector_param as _,        // $3
            candidate_limit,          // $4
            limit,                    // $5
            k,                        // $6
            config.text_weight,       // $7
            config.semantic_weight,   // $8
            config.start_date,        // $9
            config.end_date,          // $10
            is_video_filter,          // $11
            &config.country_codes,    // $12
            &config.person_ids,       // $13
            sort_by_str,              // $14
            semantic_score_threshold, // $15
            config.all_faces_required, // $16
            offset,                    // $17
            is_panorama_filter         // $18
        )
            .fetch_all(pool)
            .await?;

        Ok(items)
    } else {
        // --- PATHWAY B: Direct Vector Search (Image-only query) ---
        let items = sqlx::query_as!(
            SimpleTimelineItem,
            r#"
            WITH
            filtered_media AS MATERIALIZED (
                SELECT mi.id
                FROM media_item mi
                WHERE mi.user_id = $1
                  AND mi.deleted = false
                  AND ($5::timestamptz IS NULL OR mi.taken_at_utc >= $5)
                  AND ($6::timestamptz IS NULL OR mi.taken_at_utc <= $6)
                  AND ($7::bool IS NULL OR mi.is_video = $7)
                  AND ($14::bool IS NULL OR mi.use_panorama_viewer = $14)
                  AND (cardinality($8::text[]) = 0 OR EXISTS (
                      SELECT 1 FROM gps g JOIN location l ON g.location_id = l.id
                      WHERE g.media_item_id = mi.id AND l.country_code = ANY($8)
                  ))
                  AND (cardinality($9::text[]) = 0 OR (
                      SELECT COUNT(DISTINCT p.id)
                      FROM visual_analysis va
                      JOIN face f ON f.visual_analysis_id = va.id
                      JOIN face_cluster fc ON f.face_cluster_id = fc.id
                      JOIN person p ON fc.person_id = p.id
                      WHERE va.media_item_id = mi.id AND p.id = ANY($9)
                  ) >= (CASE WHEN $12 THEN cardinality($9) ELSE 1 END))
            ),
            vec AS (
                SELECT DISTINCT ON (media_item_id)
                    media_item_id as id,
                    distance
                FROM (
                    SELECT va.media_item_id, va.embedding <=> $2::vector as distance
                    FROM visual_analysis va
                    WHERE va.user_id = $1
                      AND va.deleted = false
                      AND (va.embedding <=> $2::vector) < $11
                      AND EXISTS (
                          SELECT 1 FROM filtered_media fm
                          WHERE fm.id = va.media_item_id
                      )
                    ORDER BY va.embedding <=> $2::vector
                    LIMIT $3 * 5
                ) sub_ordered
                ORDER BY media_item_id, distance
                LIMIT $3
            )
            SELECT
                mi.id::text as "id!",
                mi.is_video as "is_video!",
                mi.has_thumbnails as "has_thumbnails!",
                mi.duration_ms as "duration_ms: i32",
                (mi.width::real / mi.height::real) as "ratio!"
            FROM vec v
            JOIN media_item mi ON mi.id = v.id
            ORDER BY
                (CASE WHEN $10 = 'date' THEN NULL ELSE v.distance END) ASC NULLS LAST,
                mi.sort_timestamp DESC
            LIMIT $4 OFFSET $13
            "#,
            user.id,                   // $1
            vector_param as _,         // $2
            candidate_limit as i32,    // $3
            limit,                     // $4
            config.start_date,         // $5
            config.end_date,           // $6
            is_video_filter,           // $7
            &config.country_codes,     // $8
            &config.person_ids,        // $9
            sort_by_str,               // $10
            semantic_score_threshold,  // $11
            config.all_faces_required, // $12
            offset,                    // $13
            is_panorama_filter,        // $14
        )
        .fetch_all(pool)
        .await?;

        Ok(items)
    }
}

pub async fn search_filter_ranges(
    user: &User,
    pool: &PgPool,
) -> Result<SearchFilterRanges, AppError> {
    let months_task = sqlx::query!(
        r#"
        SELECT DISTINCT month_id AS "months!"
        FROM media_item
        WHERE user_id = $1
          AND deleted = false
        ORDER BY month_id
        "#,
        user.id
    )
    .fetch_all(pool);
    let countries_task = sqlx::query!(
        r#"
        SELECT DISTINCT l.country_code, l.country_name
        FROM location l
        JOIN gps g ON l.id = g.location_id
        JOIN media_item mi ON g.media_item_id = mi.id
        WHERE mi.user_id = $1 AND mi.deleted = false
        ORDER BY l.country_name
        "#,
        user.id
    )
    .fetch_all(pool);
    let people_task = sqlx::query!(
        r#"
        SELECT DISTINCT name, id
        FROM person
        WHERE user_id = $1 AND name IS NOT NULL AND name != ''
        ORDER BY name
        "#,
        user.id
    )
    .fetch_all(pool);

    let (available_month_records, countries_records, people_records) =
        tokio::try_join!(months_task, countries_task, people_task)?;
    let countries = countries_records
        .into_iter()
        .map(|c| (c.country_code, c.country_name))
        .collect();
    let people = people_records
        .into_iter()
        .filter_map(|c| c.name.map(|name| (name, c.id.clone())))
        .collect();
    let available_months = available_month_records.iter().map(|r| r.months).collect();

    Ok(SearchFilterRanges {
        available_months,
        people,
        countries,
    })
}

fn has_active_filters(config: &SearchMediaConfig) -> bool {
    config.media_type != SearchMediaType::All
        || config.start_date.is_some()
        || config.end_date.is_some()
        || !config.country_codes.is_empty()
        || !config.person_ids.is_empty()
        || config.negative_query.is_some()
}

pub async fn get_search_suggestions(
    user: &User,
    pool: &PgPool,
    query: &str,
    limit: Option<i64>,
) -> Result<SearchSuggestionsResponse, AppError> {
    if query.trim().is_empty() {
        return Ok(SearchSuggestionsResponse::default());
    }

    let limit = limit.unwrap_or(10).min(50);
    let ilike_query = format!("%{query}%");
    let suggestions = sqlx::query!(
        r#"
        WITH matched_terms AS (
            (SELECT c.search_term as suggestion, COUNT(DISTINCT va.media_item_id) as photo_count, 'SEARCH' as "type!", NULL as "id"
            FROM classification c
            JOIN visual_analysis va ON c.visual_analysis_id = va.id
            WHERE va.user_id = $1
              AND c.search_term ILIKE $2
              AND c.search_term != ''
            GROUP BY c.search_term
            LIMIT $3 * 2)

            UNION ALL

            (SELECT p.name as suggestion, COUNT(DISTINCT va.media_item_id) as photo_count, 'PERSON' as "type!", p.id::text as "id"
            FROM person p
            JOIN face_cluster fc ON fc.person_id = p.id
            JOIN face f ON f.face_cluster_id = fc.id
            JOIN visual_analysis va ON f.visual_analysis_id = va.id
            WHERE p.user_id = $1
              AND p.name ILIKE $2
              AND p.name != ''
            GROUP BY p.name, p.id
            LIMIT $3 * 2)

            UNION ALL

            (SELECT loc.val as suggestion, COUNT(DISTINCT g.media_item_id) as photo_count, 'SEARCH' as "type!", NULL as "id"
            FROM (
                SELECT id, name as val FROM location WHERE name ILIKE $2
                UNION
                SELECT id, admin1 as val FROM location WHERE admin1 ILIKE $2
                UNION
                SELECT id, country_name as val FROM location WHERE country_name ILIKE $2
            ) loc
            JOIN gps g ON g.location_id = loc.id
            JOIN media_item mi ON g.media_item_id = mi.id
            WHERE mi.user_id = $1 AND mi.deleted = false
            GROUP BY loc.val
            LIMIT $3 * 2)

            UNION ALL

            (SELECT a.name as suggestion, COUNT(DISTINCT am.media_item_id) as photo_count, 'ALBUM' as "type!", a.id::text as "id"
            FROM album a
            LEFT JOIN album_media_item am ON a.id = am.album_id
            LEFT JOIN album_collaborator ac ON a.id = ac.album_id AND ac.user_id = $1
            WHERE (a.owner_id = $1 OR ac.user_id IS NOT NULL)
              AND a.name ILIKE $2
              AND a.name != ''
            GROUP BY a.name, a.id
            LIMIT $3 * 2)

            UNION ALL

            (SELECT o.tag as suggestion, COUNT(DISTINCT va.media_item_id) as photo_count, 'SEARCH' as "type!", NULL as "id"
            FROM object o
            JOIN visual_analysis va ON o.visual_analysis_id = va.id
            WHERE va.user_id = $1
              AND o.tag ILIKE $2
              AND o.tag != ''
            GROUP BY o.tag
            LIMIT $3 * 2)
        )
        SELECT suggestion as "suggestion!", "type!" as "type!", "id" as "id?", SUM(photo_count)::int8 as "photo_count!"
        FROM matched_terms
        GROUP BY suggestion, "type!", "id"
        ORDER BY (CASE WHEN "type!" = 'ALBUM' THEN 0 ELSE (CASE WHEN "type!" = 'PERSON' THEN 1 ELSE 2 END) END), "photo_count!" DESC, suggestion ASC
        LIMIT $3
        "#,
        user.id,
        ilike_query,
        limit as i32
    )
        .fetch_all(pool)
        .await?;

    Ok(SearchSuggestionsResponse {
        suggestions: suggestions
            .into_iter()
            .map(|row| SearchSuggestion {
                text: row.suggestion,
                suggestion_type: match row.r#type.as_str() {
                    "ALBUM" => SuggestionType::Album as i32,
                    "PERSON" => SuggestionType::Person as i32,
                    _ => SuggestionType::Search as i32,
                },
                id: row.id,
            })
            .collect(),
    })
}

pub async fn get_random_search_suggestion(
    user: &User,
    pool: &PgPool,
) -> Result<Option<String>, AppError> {
    let rows = sqlx::query!(
        r#"
        WITH matched_terms AS (
            (SELECT c.search_term as suggestion
            FROM classification c
            JOIN visual_analysis va ON c.visual_analysis_id = va.id
            WHERE va.user_id = $1
              AND c.search_term != ''
            GROUP BY c.search_term
            ORDER BY COUNT(DISTINCT va.media_item_id) DESC
            LIMIT 100)

            UNION ALL

            (SELECT p.name as suggestion
            FROM person p
            JOIN face_cluster fc ON fc.person_id = p.id
            JOIN face f ON f.face_cluster_id = fc.id
            JOIN visual_analysis va ON f.visual_analysis_id = va.id
            WHERE p.user_id = $1
              AND p.name != ''
            GROUP BY p.name
            ORDER BY COUNT(DISTINCT va.media_item_id) DESC
            LIMIT 100)

            UNION ALL

            (SELECT val as suggestion
            FROM (
                (SELECT l.name as val, COUNT(mi.id) as cnt
                FROM location l
                JOIN gps g ON g.location_id = l.id
                JOIN media_item mi ON g.media_item_id = mi.id
                WHERE mi.user_id = $1 AND mi.deleted = false AND l.name != ''
                GROUP BY l.name
                LIMIT 100)
                UNION ALL
                (SELECT l.admin1 as val, COUNT(mi.id) as cnt
                FROM location l
                JOIN gps g ON g.location_id = l.id
                JOIN media_item mi ON g.media_item_id = mi.id
                WHERE mi.user_id = $1 AND mi.deleted = false AND l.admin1 != ''
                GROUP BY l.admin1
                LIMIT 100)
                UNION ALL
                (SELECT l.country_name as val, COUNT(mi.id) as cnt
                FROM location l
                JOIN gps g ON g.location_id = l.id
                JOIN media_item mi ON g.media_item_id = mi.id
                WHERE mi.user_id = $1 AND mi.deleted = false AND l.country_name != ''
                GROUP BY l.country_name
                LIMIT 100)
            ) locs
            ORDER BY cnt DESC
            LIMIT 100)

            UNION ALL

            (SELECT a.name as suggestion
            FROM album a
            LEFT JOIN album_collaborator ac ON a.id = ac.album_id AND ac.user_id = $1
            JOIN album_media_item am ON a.id = am.album_id
            WHERE (a.owner_id = $1 OR ac.user_id IS NOT NULL)
              AND a.name != ''
            GROUP BY a.name, a.id
            ORDER BY COUNT(DISTINCT am.media_item_id) DESC
            LIMIT 100)

            UNION ALL

            (SELECT o.tag as suggestion
            FROM object o
            JOIN visual_analysis va ON o.visual_analysis_id = va.id
            WHERE va.user_id = $1
              AND o.tag != ''
            GROUP BY o.tag
            ORDER BY RANDOM()
            LIMIT 100)
        )
        SELECT suggestion as "suggestion!"
        FROM matched_terms
        ORDER BY RANDOM()
        -- `LIMIT 500` because I get like 95%/5% ratio of locations/objects otherwise
        -- I think Postgres is optimizing something away if I `LIMIT 1`
        -- This endpoint doesn't have to be fast anyway
        LIMIT 500
        "#,
        user.id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.first().map(|r| r.suggestion.clone()))
}
