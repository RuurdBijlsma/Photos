use moka::future::Cache;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::warn;

const MOUNT_CACHE_TTL: Duration = Duration::from_secs(60);
const JUNK_DIR_NAMES: &[&str] = &[
    "lost+found",
    ".ds_store",
    ".trash",
    ".trashes",
    "desktop.ini",
    ".localized",
];

static MOUNT_CACHE: OnceLock<Cache<std::path::PathBuf, MediaMountStatus>> = OnceLock::new();

fn get_mount_cache() -> &'static Cache<std::path::PathBuf, MediaMountStatus> {
    MOUNT_CACHE.get_or_init(|| {
        Cache::builder()
            .max_capacity(1)
            .time_to_live(MOUNT_CACHE_TTL)
            .build()
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaFolderStatus {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Default for MediaFolderStatus {
    fn default() -> Self {
        Self {
            available: true,
            reason: None,
        }
    }
}

impl From<MediaMountStatus> for MediaFolderStatus {
    fn from(status: MediaMountStatus) -> Self {
        match status {
            MediaMountStatus::Available => Self::default(),
            MediaMountStatus::Unavailable { reason } => Self {
                available: false,
                reason: Some(reason),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaMountStatus {
    Available,
    Unavailable { reason: String },
}

impl MediaMountStatus {
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Available => None,
            Self::Unavailable { reason } => Some(reason),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaRootProbe {
    Missing,
    NotDirectory,
    Unreadable,
    Empty,
    HasContent,
}

/// Cheap filesystem probe of `media_root` (no DB, no walk of the library).
#[must_use]
pub fn probe_media_root(media_root: &Path) -> MediaRootProbe {
    if !media_root.exists() {
        return MediaRootProbe::Missing;
    }
    if !media_root.is_dir() {
        return MediaRootProbe::NotDirectory;
    }
    match dir_has_non_junk_entries(media_root) {
        Ok(true) => MediaRootProbe::HasContent,
        Ok(false) => MediaRootProbe::Empty,
        Err(_) => MediaRootProbe::Unreadable,
    }
}

fn dir_has_non_junk_entries(path: &Path) -> std::io::Result<bool> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Ok(true);
        };
        if JUNK_DIR_NAMES
            .iter()
            .any(|junk| junk.eq_ignore_ascii_case(name))
        {
            continue;
        }
        return Ok(true);
    }
    Ok(false)
}

fn status_from_probe(
    media_root: &Path,
    probe: MediaRootProbe,
    has_library_items: bool,
) -> MediaMountStatus {
    match probe {
        MediaRootProbe::Missing => MediaMountStatus::Unavailable {
            reason: format!(
                "Media folder does not exist ({}). The drive may be unmounted.",
                media_root.display()
            ),
        },
        MediaRootProbe::NotDirectory => MediaMountStatus::Unavailable {
            reason: format!(
                "Media folder is not a directory ({}).",
                media_root.display()
            ),
        },
        MediaRootProbe::Unreadable => MediaMountStatus::Unavailable {
            reason: format!(
                "Media folder cannot be read ({}). Check mount and permissions.",
                media_root.display()
            ),
        },
        MediaRootProbe::Empty if has_library_items => MediaMountStatus::Unavailable {
            reason: format!(
                "Media folder is empty ({}) but the library still has items. The drive may be unmounted.",
                media_root.display()
            ),
        },
        MediaRootProbe::HasContent | MediaRootProbe::Empty => MediaMountStatus::Available,
    }
}

async fn has_live_media_items(pool: &PgPool) -> color_eyre::Result<bool> {
    let exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM media_item WHERE deleted = false) AS "exists!""#
    )
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

async fn compute_media_mount_status(
    pool: &PgPool,
    media_root: &Path,
) -> color_eyre::Result<MediaMountStatus> {
    let root = media_root.to_path_buf();
    let probe = tokio::task::spawn_blocking(move || probe_media_root(&root)).await?;
    if probe == MediaRootProbe::HasContent {
        return Ok(MediaMountStatus::Available);
    }
    let has_items = has_live_media_items(pool).await?;
    let status = status_from_probe(media_root, probe, has_items);
    if let MediaMountStatus::Unavailable { reason } = &status {
        warn!("{reason}");
    }
    Ok(status)
}

/// Cached (60s TTL) check that the media root looks mounted.
///
/// An empty folder is only treated as unmounted when the library already has items.
pub async fn media_mount_status(
    pool: &PgPool,
    media_root: &Path,
) -> color_eyre::Result<MediaMountStatus> {
    let pool = pool.clone();
    let media_root = media_root.to_path_buf();
    Ok(get_mount_cache()
        .get_with(media_root.clone(), async move {
            match compute_media_mount_status(&pool, &media_root).await {
                Ok(status) => status,
                Err(e) => MediaMountStatus::Unavailable {
                    reason: format!("Could not check media folder mount: {e}"),
                },
            }
        })
        .await)
}

/// Uncached / fresh check that the media root looks mounted.
///
/// Bypasses the cache, computes the latest status directly, and updates the cache
/// so that subsequent queries immediately see the new state.
pub async fn media_mount_status_fresh(
    pool: &PgPool,
    media_root: &Path,
) -> color_eyre::Result<MediaMountStatus> {
    let status = match compute_media_mount_status(pool, media_root).await {
        Ok(status) => status,
        Err(e) => MediaMountStatus::Unavailable {
            reason: format!("Could not check media folder mount: {e}"),
        },
    };
    get_mount_cache()
        .insert(media_root.to_path_buf(), status.clone())
        .await;
    Ok(status)
}

/// Evicts the cached mount status for `media_root`.
pub async fn invalidate_mount_cache(media_root: &Path) {
    get_mount_cache()
        .invalidate(&media_root.to_path_buf())
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn missing_path_is_missing() {
        assert_eq!(
            probe_media_root(Path::new("/definitely/not/a/real/photos/root")),
            MediaRootProbe::Missing
        );
    }

    #[test]
    fn empty_dir_is_empty() {
        let dir = tempdir().expect("temp dir");
        assert_eq!(probe_media_root(dir.path()), MediaRootProbe::Empty);
    }

    #[test]
    fn junk_only_dir_is_empty() {
        let dir = tempdir().expect("temp dir");
        fs::create_dir(dir.path().join("lost+found")).expect("mkdir");
        fs::write(dir.path().join(".DS_Store"), []).expect("write");
        assert_eq!(probe_media_root(dir.path()), MediaRootProbe::Empty);
    }

    #[test]
    fn dir_with_file_has_content() {
        let dir = tempdir().expect("temp dir");
        fs::write(dir.path().join("photo.jpg"), b"x").expect("write");
        assert_eq!(probe_media_root(dir.path()), MediaRootProbe::HasContent);
    }

    #[test]
    fn empty_with_library_items_is_unavailable() {
        let dir = tempdir().expect("temp dir");
        let status = status_from_probe(dir.path(), MediaRootProbe::Empty, true);
        assert!(!status.is_available());
    }

    #[test]
    fn empty_without_library_items_is_available() {
        let dir = tempdir().expect("temp dir");
        let status = status_from_probe(dir.path(), MediaRootProbe::Empty, false);
        assert!(status.is_available());
    }

    #[test]
    fn has_content_is_available_even_with_items() {
        let dir = tempdir().expect("temp dir");
        let status = status_from_probe(dir.path(), MediaRootProbe::HasContent, true);
        assert!(status.is_available());
    }

    #[tokio::test]
    async fn cache_invalidation_works() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().to_path_buf();
        get_mount_cache()
            .insert(path.clone(), MediaMountStatus::Available)
            .await;
        assert_eq!(
            get_mount_cache().get(&path).await,
            Some(MediaMountStatus::Available)
        );
        invalidate_mount_cache(&path).await;
        assert_eq!(get_mount_cache().get(&path).await, None);
    }
}
