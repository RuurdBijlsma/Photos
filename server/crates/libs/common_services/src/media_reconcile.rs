use crate::database::DbError;
use app_state::IngestSettings;
use sqlx::PgPool;
use std::path::Path;

/// Returns true when the path has a configured photo or video extension.
#[must_use]
pub fn is_media_path(path: &str, settings: &IngestSettings) -> bool {
    has_media_extension(
        path,
        &settings.file_detection.photo_extensions,
        &settings.file_detection.video_extensions,
    )
}

pub(crate) fn has_media_extension(
    path: &str,
    photo_exts: &[String],
    video_exts: &[String],
) -> bool {
    let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) else {
        return false;
    };
    photo_exts
        .iter()
        .chain(video_exts)
        .any(|e| e.eq_ignore_ascii_case(ext))
}

#[derive(Debug, Default)]
pub struct ReconcileRemovalResult {
    pub marked_missing: usize,
    pub cancelled_jobs: u64,
}

/// Marks removed files and directory descendants missing and cancels their active ingest jobs.
///
/// # Errors
///
/// Returns an error if the database transaction fails.
pub async fn reconcile_removed_paths(
    pool: &PgPool,
    settings: &IngestSettings,
    relative_paths: &[String],
) -> Result<ReconcileRemovalResult, DbError> {
    let mut paths: Vec<String> = relative_paths
        .iter()
        .filter(|path| !path.is_empty())
        .cloned()
        .collect();
    paths.sort_unstable();
    paths.dedup();
    if paths.is_empty() {
        return Ok(ReconcileRemovalResult::default());
    }

    // A directory whose name ends in a media extension is rare; the exact update still catches it.
    let dir_paths: Vec<String> = paths
        .iter()
        .filter(|path| !is_media_path(path, settings))
        .cloned()
        .collect();

    let mut tx = pool.begin().await?;
    let cancelled = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = 'cancelled'
        WHERE status IN ('queued', 'running')
          AND job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis')
          AND (
              relative_path = ANY($1)
              OR EXISTS (
                  SELECT 1
                  FROM unnest($2::text[]) AS p(prefix)
                  WHERE left(jobs.relative_path, length(p.prefix) + 1) = p.prefix || '/'
              )
          )
        "#,
        &paths,
        &dir_paths
    )
    .execute(&mut *tx)
    .await?;

    let marked = sqlx::query!(
        r#"
        UPDATE media_item
        SET missing_since = now(),
            updated_at = now()
        WHERE deleted = false
          AND missing_since IS NULL
          AND (
              relative_path = ANY($1)
              OR EXISTS (
                  SELECT 1
                  FROM unnest($2::text[]) AS p(prefix)
                  WHERE left(media_item.relative_path, length(p.prefix) + 1) = p.prefix || '/'
              )
          )
        "#,
        &paths,
        &dir_paths
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(ReconcileRemovalResult {
        marked_missing: marked.rows_affected() as usize,
        cancelled_jobs: cancelled.rows_affected(),
    })
}

#[cfg(test)]
mod tests {
    use super::has_media_extension;

    #[test]
    fn media_extension_detection_is_case_insensitive() {
        let photos = vec!["jpg".to_owned()];
        let videos = vec!["mp4".to_owned()];
        assert!(has_media_extension("a.JPG", &photos, &videos));
        assert!(has_media_extension("a.mp4", &photos, &videos));
        assert!(!has_media_extension("a", &photos, &videos));
        assert!(!has_media_extension("a.txt", &photos, &videos));
    }
}
