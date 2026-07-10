use crate::api::app_error::AppError;
use crate::api::search::cache::get_cached_text_embedding;
use crate::api::search::interfaces::{SearchMediaConfig, SearchMediaType, SearchSortBy};
use common_types::pb::api::SimpleTimelineItem;
use open_clip_inference::TextEmbedder;
use pgvector::Vector;
use sqlx::PgPool;
use std::sync::Arc;

pub async fn basic_search_media(
    user_id:i32,
    pool: &PgPool,
    embedder: Arc<TextEmbedder>,
    query: &str,
    config: SearchMediaConfig,
) -> Result<Vec<SimpleTimelineItem>, AppError> {
    let query_str = query.to_string();
    let query_embedding =
        get_cached_text_embedding(&query_str, &config.embedder_model_id, pool, embedder).await?;
    let vector_param = Vector::from(query_embedding);

    let limit = config.limit.unwrap_or(100).min(1000);
    let offset = config.offset.unwrap_or(0);
    let candidate_limit = limit * 3 + 300;
    let k = 60.0f64;
    // Only use semantic score limit when sorting by relevancy
    let semantic_score_threshold = if config.sort_by == SearchSortBy::Relevancy {
        2.0
    } else {
        config.semantic_score_threshold
    };

    let items = sqlx::query_as!(
        SimpleTimelineItem,
        r#"
        WITH
        fts AS (
            SELECT
                id,
                ts_rank_cd(search_vector, websearch_to_tsquery('english', $1)) as score,
                ROW_NUMBER() OVER (ORDER BY ts_rank_cd(search_vector, websearch_to_tsquery('english', $1)) DESC) as rank
            FROM media_item
            WHERE user_id = $2
              AND search_vector @@ websearch_to_tsquery('english', $1)
              AND deleted = false
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
                    SELECT media_item_id, embedding <=> $3::vector as distance
                    FROM visual_analysis
                    WHERE user_id = $2
                      AND deleted = false
                      AND (embedding <=> $3::vector) < $9
                    ORDER BY embedding <=> $3::vector
                    LIMIT $4 * 4
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
        ORDER BY sc.combined_score DESC
        LIMIT $5 OFFSET $10
         "#,
        query,                 // $1
        user_id,               // $2
        vector_param as _,     // $3
        candidate_limit,       // $4
        limit,                 // $5
        k,                     // $6
        config.text_weight,    // $7
        config.semantic_weight, // $8
        semantic_score_threshold, // $9
        offset,                   // $10
    )
        .fetch_all(pool)
        .await?;

    Ok(items)
}

#[allow(clippy::too_many_lines)]
pub async fn advanced_search_media(
    user_id:i32,
    pool: &PgPool,
    embedder: Arc<TextEmbedder>,
    query: &str,
    config: SearchMediaConfig,
) -> Result<Vec<SimpleTimelineItem>, AppError> {
    let query_str = query.to_string();
    let embedder_clone = embedder.clone();

    let q_emb_task = get_cached_text_embedding(
        &query_str,
        &config.embedder_model_id,
        pool,
        embedder_clone.clone(),
    );

    let (query_embedding, fts_query) = if let Some(negative_query) = &config.negative_query {
        let neg_str = negative_query.clone();

        let neg_emb_task =
            get_cached_text_embedding(&neg_str, &config.embedder_model_id, pool, embedder_clone);
        let (mut q_emb, neg_emb) = tokio::try_join!(q_emb_task, neg_emb_task)?;

        for (pos, neg) in q_emb.iter_mut().zip(neg_emb.iter()) {
            *pos = 0.5_f32.mul_add(-*neg, *pos);
        }
        let norm = q_emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-6 {
            for val in &mut q_emb {
                *val /= norm;
            }
        }
        let neg_terms: Vec<String> = negative_query
            .split_whitespace()
            .map(|s| format!("-{s}"))
            .collect();
        (q_emb, format!("{} {}", query, neg_terms.join(" ")))
    } else {
        let q_emb = q_emb_task.await?;
        (q_emb, query.to_string())
    };

    let vector_param = Vector::from(query_embedding);
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
        // With relevancy sort, threshold doesn't matter much. Limit + relevancy sort handles it
        2.0
    } else {
        config.semantic_score_threshold
    };

    let sort_by_str = match config.sort_by {
        SearchSortBy::Relevancy => "relevancy",
        SearchSortBy::Date => "date",
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
        user_id,                  // $2
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
        is_panorama_filter        // $18
    )
        .fetch_all(pool)
        .await?;

    Ok(items)
}

pub async fn filter_only_search_media(
    user_id:i32,
    pool: &PgPool,
    config: SearchMediaConfig,
) -> Result<Vec<SimpleTimelineItem>, AppError> {
    let limit = config.limit.unwrap_or(100).min(500);
    let offset = config.offset.unwrap_or(0);

    let (is_video_filter, is_panorama_filter) = match config.media_type {
        SearchMediaType::Video => (Some(true), Some(false)),
        SearchMediaType::Photo => (Some(false), Some(false)),
        SearchMediaType::Panorama => (Some(false), Some(true)),
        SearchMediaType::All => (None, None),
    };

    let items = sqlx::query_as!(
        SimpleTimelineItem,
        r#"
        SELECT
            mi.id::text as "id!",
            mi.is_video as "is_video!",
            mi.has_thumbnails as "has_thumbnails!",
            mi.duration_ms as "duration_ms: i32",
            (mi.width::real / mi.height::real) as "ratio!"
        FROM media_item mi
        WHERE mi.user_id = $1
          AND mi.deleted = false
          AND ($2::timestamptz IS NULL OR mi.taken_at_utc >= $2)
          AND ($3::timestamptz IS NULL OR mi.taken_at_utc <= $3)
          AND ($4::bool IS NULL OR mi.is_video = $4)
          AND ($10::bool IS NULL OR mi.use_panorama_viewer = $10)
          AND (cardinality($5::text[]) = 0 OR EXISTS (
              SELECT 1 FROM gps g JOIN location l ON g.location_id = l.id
              WHERE g.media_item_id = mi.id AND l.country_code = ANY($5)
          ))
          AND (cardinality($6::text[]) = 0 OR (
              SELECT COUNT(DISTINCT p.id)
              FROM visual_analysis va
              JOIN face f ON f.visual_analysis_id = va.id
              JOIN face_cluster fc ON f.face_cluster_id = fc.id
              JOIN person p ON fc.person_id = p.id
              WHERE va.media_item_id = mi.id AND p.id = ANY($6)
          ) >= (CASE WHEN $7 THEN cardinality($6) ELSE 1 END))
        ORDER BY mi.sort_timestamp DESC
        LIMIT $8 OFFSET $9
        "#,
        user_id,                   // $1
        config.start_date,         // $2
        config.end_date,           // $3
        is_video_filter,           // $4
        &config.country_codes,     // $5
        &config.person_ids,        // $6
        config.all_faces_required, // $7
        limit,                     // $8
        offset,                    // $9
        is_panorama_filter,        // $10
    )
    .fetch_all(pool)
    .await?;

    Ok(items)
}
