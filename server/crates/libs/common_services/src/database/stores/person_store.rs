use crate::api::people::interfaces::{PersonSummary, UpdatePersonRequest};
use crate::database::DbError;
use crate::utils::nice_id;
use common_types::pb::api::SimpleTimelineItem;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use tracing::instrument;

pub struct PersonStore;

impl PersonStore {
    #[instrument(skip(executor))]
    pub async fn list_by_user_id(
        executor: impl PgExecutor<'_>,
        user_id: i32,
    ) -> Result<Vec<PersonSummary>, DbError> {
        let people = sqlx::query_as!(
            PersonSummary,
            r#"
            SELECT
                p.id,
                p.name,
                p.face_thumb_id,
                COUNT(DISTINCT va.media_item_id)::INT as "photo_count!",
                COALESCE(array_agg(DISTINCT fc.id) FILTER (WHERE fc.id IS NOT NULL), '{}') as "face_cluster_ids!"
            FROM person p
            LEFT JOIN face_cluster fc ON fc.person_id = p.id
            LEFT JOIN face f ON f.face_cluster_id = fc.id
            LEFT JOIN visual_analysis va ON f.visual_analysis_id = va.id
            WHERE p.user_id = $1
            GROUP BY p.id
            ORDER BY "photo_count!" DESC, p.id ASC
            "#,
            user_id
        )
            .fetch_all(executor)
            .await?;

        Ok(people)
    }

    #[instrument(skip(executor))]
    pub async fn find_by_id(
        executor: impl PgExecutor<'_>,
        person_id: &str,
        user_id: i32,
    ) -> Result<Option<PersonSummary>, DbError> {
        let person = sqlx::query_as!(
            PersonSummary,
            r#"
            SELECT
                p.id,
                p.name,
                p.face_thumb_id,
                COUNT(DISTINCT va.media_item_id)::INT as "photo_count!",
                COALESCE(array_agg(DISTINCT fc.id) FILTER (WHERE fc.id IS NOT NULL), '{}') as "face_cluster_ids!"
            FROM person p
            LEFT JOIN face_cluster fc ON fc.person_id = p.id
            LEFT JOIN face f ON f.face_cluster_id = fc.id
            LEFT JOIN visual_analysis va ON f.visual_analysis_id = va.id
            WHERE p.id = $1 AND p.user_id = $2
            GROUP BY p.id
            "#,
            person_id,
            user_id
        )
            .fetch_optional(executor)
            .await?;

        Ok(person)
    }

    #[instrument(skip(executor))]
    pub async fn update(
        executor: impl PgExecutor<'_>,
        person_id: &str,
        user_id: i32,
        payload: &UpdatePersonRequest,
    ) -> Result<u64, DbError> {
        let result = sqlx::query!(
            r#"
            UPDATE person
            SET
                name = CASE WHEN $3::boolean THEN name ELSE $4 END,
                face_thumb_id = CASE WHEN $5::boolean THEN face_thumb_id ELSE $6 END,
                updated_at = now()
            WHERE id = $1 AND user_id = $2
            "#,
            person_id,
            user_id,
            payload.name.is_ignore(),
            payload.name.clone().value(),
            payload.face_thumb_id.is_ignore(),
            payload.face_thumb_id.clone().value(),
        )
        .execute(executor)
        .await?;

        Ok(result.rows_affected())
    }

    /// Merge target person into source person.
    ///
    /// Target person will be deleted.
    #[instrument(skip(pool))]
    pub async fn merge(
        pool: &PgPool,
        source_person_id: &str,
        target_person_id: &str,
    ) -> Result<(), DbError> {
        if source_person_id == target_person_id {
            return Ok(());
        }

        let mut tx = pool.begin().await?;

        // Move all clusters from target to source
        sqlx::query!(
            r#"
        UPDATE face_cluster
        SET person_id = $1, updated_at = now()
        WHERE person_id = $2
        "#,
            source_person_id,
            target_person_id
        )
        .execute(&mut *tx)
        .await?;

        // If source.name is NOT NULL, it stays. If it IS NULL, it takes target.name.
        // The same logic applies to face_thumb_id.
        sqlx::query!(
            r#"
        UPDATE person source
        SET
            name = COALESCE(source.name, target.name),
            face_thumb_id = COALESCE(source.face_thumb_id, target.face_thumb_id),
            updated_at = now()
        FROM person target
        WHERE source.id = $1 AND target.id = $2
        "#,
            source_person_id,
            target_person_id
        )
        .execute(&mut *tx)
        .await?;

        // Delete the target person
        sqlx::query!("DELETE FROM person WHERE id = $1", target_person_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    #[instrument(skip(pool))]
    pub async fn unmerge(
        pool: &PgPool,
        person: &PersonSummary,
        user_id: i32,
    ) -> Result<(), DbError> {
        let cluster_ids = Self::ordered_cluster_ids(person);
        if cluster_ids.len() <= 1 {
            return Ok(());
        }

        let kept_cluster_id = cluster_ids[0].clone();
        let mut tx = pool.begin().await?;

        sqlx::query!(
            r#"
            UPDATE person
            SET face_thumb_id = $1, updated_at = now()
            WHERE id = $2
            "#,
            kept_cluster_id,
            person.id
        )
        .execute(&mut *tx)
        .await?;

        for cluster_id in cluster_ids.iter().skip(1) {
            let new_person_id = nice_id(10);
            Self::move_cluster_to_new_person(&mut tx, user_id, &new_person_id, cluster_id).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    fn ordered_cluster_ids(person: &PersonSummary) -> Vec<String> {
        let mut cluster_ids = person.face_cluster_ids.clone();
        cluster_ids.sort_unstable();

        if let Some(thumb_id) = &person.face_thumb_id
            && let Some(thumb_index) = cluster_ids.iter().position(|id| id == thumb_id)
        {
            cluster_ids.swap(0, thumb_index);
        }

        cluster_ids
    }

    async fn move_cluster_to_new_person(
        tx: &mut Transaction<'_, Postgres>,
        user_id: i32,
        new_person_id: &str,
        cluster_id: &str,
    ) -> Result<(), DbError> {
        sqlx::query!(
            r#"
            INSERT INTO person (id, user_id, face_thumb_id)
            VALUES ($1, $2, $3)
            "#,
            new_person_id,
            user_id,
            cluster_id
        )
        .execute(&mut **tx)
        .await?;

        sqlx::query!(
            r#"
            UPDATE face_cluster
            SET person_id = $1, updated_at = now()
            WHERE id = $2
            "#,
            new_person_id,
            cluster_id
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    #[instrument(skip(pool))]
    pub async fn get_person_media_items(
        pool: &PgPool,
        person_id: &str, // Changed from i64
        user_id: i32,
    ) -> Result<Vec<SimpleTimelineItem>, DbError> {
        let items = sqlx::query_as!(
            SimpleTimelineItem,
            r#"
            SELECT
                id,
                is_video,
                has_thumbnails,
                duration_ms::INT as duration_ms,
                ratio as "ratio!"
            FROM (
                SELECT DISTINCT ON (mi.id)
                    mi.id,
                    mi.is_video,
                    mi.has_thumbnails,
                    mi.duration_ms,
                    (mi.width::real / mi.height::real) as ratio,
                    mi.sort_timestamp
                FROM person p
                JOIN face_cluster fc ON fc.person_id = p.id
                JOIN face f ON f.face_cluster_id = fc.id
                JOIN visual_analysis va ON f.visual_analysis_id = va.id
                JOIN media_item mi ON va.media_item_id = mi.id
                WHERE p.id = $1 AND mi.user_id = $2 AND mi.deleted = false
                ORDER BY mi.id
            ) sub
            ORDER BY sub.sort_timestamp DESC
            "#,
            person_id,
            user_id
        )
        .fetch_all(pool)
        .await?;

        Ok(items)
    }
}
