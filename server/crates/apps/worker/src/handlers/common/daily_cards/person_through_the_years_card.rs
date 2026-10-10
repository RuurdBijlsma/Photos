use crate::handlers::common::clustering::cosine_similarity;
use crate::handlers::common::daily_cards::DailyCardGenerator;
use app_state::AppSettings;
use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDateTime};
use color_eyre::Result;
use pgvector::Vector;
use sqlx::PgTransaction;
use std::collections::HashMap;

pub struct PersonThroughTheYearsCardGenerator;

struct CandidatePhoto {
    id: String,
    width: i32,
    height: i32,
    duration_ms: Option<i64>,
    has_thumbnails: bool,
    is_video: bool,
    use_panorama_viewer: bool,
    taken_at_local: NaiveDateTime,
    composite_score: f64,
    embedding: Vec<f32>,
}

#[async_trait]
#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
impl DailyCardGenerator for PersonThroughTheYearsCardGenerator {
    fn card_type(&self) -> &'static str {
        "person_through_the_years"
    }

    async fn generate(
        &self,
        tx: &mut PgTransaction<'_>,
        user_id: i32,
        _settings: &AppSettings,
    ) -> Result<()> {
        // Limit unshown buffer to at most 7 cards
        let unshown_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM daily_card WHERE user_id = $1 AND card_type = 'person_through_the_years' AND shown = false",
            user_id
        )
            .fetch_one(&mut **tx)
            .await?
            .unwrap_or(0);

        if unshown_count >= 7 {
            return Ok(());
        }

        let to_generate = usize::try_from(7 - unshown_count).unwrap_or(0);

        // Query eligible named persons with >= 2 years (730 days) span and at least 4 photos
        let eligible_people = sqlx::query!(
            r#"
            SELECT
                p.id as person_id,
                p.name as "person_name!",
                MIN(mi.taken_at_local) as "min_date!",
                MAX(mi.taken_at_local) as "max_date!",
                COUNT(DISTINCT mi.id) as "photo_count!"
            FROM person p
            JOIN face_cluster fc ON fc.person_id = p.id
            JOIN face f ON f.face_cluster_id = fc.id
            JOIN visual_analysis va ON f.visual_analysis_id = va.id
            JOIN media_item mi ON va.media_item_id = mi.id
            WHERE p.user_id = $1
              AND p.name IS NOT NULL
              AND TRIM(p.name) != ''
              AND mi.deleted = false
            GROUP BY p.id, p.name
            HAVING EXTRACT(EPOCH FROM (MAX(mi.taken_at_local) - MIN(mi.taken_at_local))) >= (730.0 * 86400.0)
               AND COUNT(DISTINCT mi.id) >= 4
            "#,
            user_id
        )
            .fetch_all(&mut **tx)
            .await?;

        if eligible_people.is_empty() {
            return Ok(());
        }

        // Query person IDs already in unshown cards to avoid duplicates
        let unshown_person_ids: Vec<String> = sqlx::query_scalar!(
            r#"
            SELECT payload->>'personId' as "person_id!"
            FROM daily_card
            WHERE user_id = $1 AND card_type = 'person_through_the_years' AND shown = false
            "#,
            user_id
        )
        .fetch_all(&mut **tx)
        .await?;

        // Query last featured timestamps for all people to support fair rotation
        let last_featured_records = sqlx::query!(
            r#"
            SELECT payload->>'personId' as "person_id!", MAX(created_at) as "last_featured!"
            FROM daily_card
            WHERE user_id = $1 AND card_type = 'person_through_the_years'
            GROUP BY payload->>'personId'
            "#,
            user_id
        )
        .fetch_all(&mut **tx)
        .await?;

        let mut last_featured_map: HashMap<String, chrono::DateTime<chrono::Utc>> = HashMap::new();
        for record in last_featured_records {
            last_featured_map.insert(record.person_id, record.last_featured);
        }

        // Filter and sort candidates: never-featured first, then oldest last_featured
        let mut candidates: Vec<_> = eligible_people
            .into_iter()
            .filter(|p| !unshown_person_ids.contains(&p.person_id))
            .collect();

        if candidates.is_empty() {
            return Ok(());
        }

        candidates.sort_by(|a, b| {
            let a_last = last_featured_map.get(&a.person_id);
            let b_last = last_featured_map.get(&b.person_id);
            match (a_last, b_last) {
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (Some(ta), Some(tb)) => ta.cmp(tb),
                (None, None) => a.person_id.cmp(&b.person_id),
            }
        });

        let mut generated_count = 0;

        // Try candidate persons until we satisfy the buffer or run out of eligible people
        for person in candidates {
            let person_id = person.person_id;
            let person_name = person.person_name;

            // Fetch candidate photos with face, quality, and embedding data
            let raw_photos = sqlx::query!(
                r#"
                SELECT
                    mi.id,
                    mi.width,
                    mi.height,
                    mi.is_video,
                    mi.duration_ms,
                    mi.has_thumbnails,
                    mi.use_panorama_viewer,
                    mi.taken_at_local,
                    f.width as "face_w!: f32",
                    f.height as "face_h!: f32",
                    f.position_x as "face_x!: f32",
                    f.position_y as "face_y!: f32",
                    f.confidence as "face_conf!: f32",
                    mq.weighted_score as "quality_score?",
                    mq.accidentalness as "accidentalness?",
                    va.embedding as "embedding!: Vector"
                FROM person p
                JOIN face_cluster fc ON fc.person_id = p.id
                JOIN face f ON f.face_cluster_id = fc.id
                JOIN visual_analysis va ON f.visual_analysis_id = va.id
                JOIN media_item mi ON va.media_item_id = mi.id
                LEFT JOIN measured_quality mq ON mq.visual_analysis_id = va.id
                WHERE p.id = $1 AND p.user_id = $2 AND mi.deleted = false AND mi.is_video = false
                ORDER BY mi.taken_at_local ASC
                "#,
                person_id,
                user_id
            )
            .fetch_all(&mut **tx)
            .await?;

            // Deduplicate multiple faces of the same person in the same photo
            let mut photo_map: HashMap<String, CandidatePhoto> = HashMap::new();
            for p in raw_photos {
                // Filter out accidental or low confidence faces
                if p.accidentalness.unwrap_or(0.0) > 0.35 || p.face_conf < 0.60 {
                    continue;
                }

                let face_area = p.face_w * p.face_h;
                // Require at least 0.4% of image area to filter out background crowd
                if face_area < 0.004 {
                    continue;
                }

                let quality_norm = p.quality_score.unwrap_or(50.0).clamp(0.0, 100.0) / 100.0;
                let face_size = f64::from((face_area).sqrt().clamp(0.0, 0.5) * 2.0);
                let center_x = f64::from(p.face_x + p.face_w / 2.0);
                let center_y = f64::from(p.face_y + p.face_h / 2.0);
                let center_dist = (center_x - 0.5).hypot(center_y - 0.5);
                let centrality = f64::mul_add(center_dist, -2.0, 1.0).clamp(0.0, 1.0);
                let conf = f64::from(p.face_conf.clamp(0.0, 1.0));

                let composite_score = 0.10f64.mul_add(
                    conf,
                    0.15f64.mul_add(centrality, 0.35f64.mul_add(face_size, 0.40 * quality_norm)),
                );

                if let Some(existing) = photo_map.get_mut(&p.id) {
                    if composite_score > existing.composite_score {
                        existing.composite_score = composite_score;
                        existing.embedding = p.embedding.to_vec();
                    }
                } else {
                    photo_map.insert(
                        p.id.clone(),
                        CandidatePhoto {
                            id: p.id,
                            width: p.width,
                            height: p.height,
                            duration_ms: p.duration_ms,
                            has_thumbnails: p.has_thumbnails,
                            is_video: p.is_video,
                            use_panorama_viewer: p.use_panorama_viewer,
                            taken_at_local: p.taken_at_local,
                            composite_score,
                            embedding: p.embedding.to_vec(),
                        },
                    );
                }
            }

            let mut sorted_photos: Vec<CandidatePhoto> = photo_map.into_values().collect();
            if sorted_photos.len() < 4 {
                continue;
            }
            sorted_photos.sort_by_key(|p| p.taken_at_local);

            let t_min = sorted_photos.first().expect("non-empty").taken_at_local;
            let t_max = sorted_photos.last().expect("non-empty").taken_at_local;
            let total_days = (t_max - t_min).num_days();

            if total_days < 730 {
                continue;
            }

            // Determine target photo count (6 to 12 photos based on span)
            let years = total_days as f64 / 365.25;
            let target_count = ((years * 1.5).round() as usize).clamp(6, 12);
            let epoch_days = (total_days / target_count as i64).max(30);

            let mut selected_indices: Vec<usize> = Vec::new();
            let min_gap = Duration::days(30);
            let lambda: f32 = 0.65; // Weight between quality score and visual diversity

            for epoch_idx in 0..target_count {
                let epoch_start = t_min + Duration::days(epoch_idx as i64 * epoch_days);
                let epoch_end = if epoch_idx == target_count - 1 {
                    t_max + Duration::days(1)
                } else {
                    epoch_start + Duration::days(epoch_days)
                };

                // Collect valid candidates within this epoch that satisfy the 30-day gap
                let mut epoch_candidates: Vec<usize> = Vec::new();
                for (idx, photo) in sorted_photos.iter().enumerate() {
                    if photo.taken_at_local >= epoch_start && photo.taken_at_local < epoch_end {
                        let has_valid_gap = selected_indices.iter().all(|&sel_idx| {
                            let diff = (photo.taken_at_local
                                - sorted_photos[sel_idx].taken_at_local)
                                .abs();
                            diff >= min_gap
                        });
                        if has_valid_gap {
                            epoch_candidates.push(idx);
                        }
                    }
                }

                if epoch_candidates.is_empty() {
                    continue;
                }

                // Pick the candidate that maximizes MMR
                let mut best_idx = epoch_candidates[0];
                let mut best_mmr = f32::NEG_INFINITY;

                for &cand_idx in &epoch_candidates {
                    let cand = &sorted_photos[cand_idx];
                    let max_sim = if selected_indices.is_empty() {
                        0.0
                    } else {
                        selected_indices
                            .iter()
                            .map(|&sel_idx| {
                                cosine_similarity(
                                    &cand.embedding,
                                    &sorted_photos[sel_idx].embedding,
                                )
                            })
                            .fold(f32::NEG_INFINITY, f32::max)
                    };

                    let mmr =
                        (1.0 - lambda).mul_add(-max_sim, lambda * (cand.composite_score as f32));
                    if mmr > best_mmr {
                        best_mmr = mmr;
                        best_idx = cand_idx;
                    }
                }

                selected_indices.push(best_idx);
            }

            // Require at least 4 photos spanning at least 4 windows
            if selected_indices.len() < 4 {
                continue;
            }

            selected_indices.sort_by_key(|&idx| sorted_photos[idx].taken_at_local);
            let selected_photos: Vec<&CandidatePhoto> = selected_indices
                .into_iter()
                .map(|idx| &sorted_photos[idx])
                .collect();

            // Cover thumbnail: choose photo with highest composite portrait score
            let thumbnail_photo = selected_photos
                .iter()
                .max_by(|a, b| {
                    a.composite_score
                        .partial_cmp(&b.composite_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .expect("non-empty");

            let start_year = selected_photos
                .first()
                .expect("non-empty")
                .taken_at_local
                .year();
            let end_year = selected_photos
                .last()
                .expect("non-empty")
                .taken_at_local
                .year();

            let title = format!("{person_name} Through the Years");
            let subtitle = format!("From {start_year} to {end_year}");

            let payload_items: Vec<serde_json::Value> = selected_photos
                .iter()
                .map(|i| {
                    serde_json::json!({
                        "id": i.id,
                        "ratio": f64::from(i.width) / f64::from(i.height),
                        "durationMs": i.duration_ms,
                        "hasThumbnails": i.has_thumbnails,
                        "isVideo": i.is_video,
                        "width": i.width,
                        "height": i.height,
                        "usePanoramaViewer": i.use_panorama_viewer,
                        "takenAtLocal": i.taken_at_local,
                    })
                })
                .collect();

            let payload = serde_json::json!({
                "personId": person_id,
                "personName": person_name,
                "mediaItems": payload_items,
            });

            sqlx::query!(
                r#"
                INSERT INTO daily_card (user_id, card_type, title, subtitle, thumbnail_media_item_id, payload)
                VALUES ($1, 'person_through_the_years', $2, $3, $4, $5)
                "#,
                user_id,
                title,
                subtitle,
                thumbnail_photo.id,
                payload
            )
                .execute(&mut **tx)
                .await?;

            generated_count += 1;
            if generated_count >= to_generate {
                break;
            }
        }

        Ok(())
    }
}
