use std::path::{Component, Path};

pub const WORKER_HEARTBEAT_SECONDS: f64 = 150.;

// assets folder subfolders
pub const TAG_VOCAB_FOLDER: &str = "assets/tag-vocab";
// static cache folders (in assets)
pub const VOCAB_CACHE_FOLDER: &str = "assets/static-cache/tag-vocab";

// media_folder subfolders
pub const ALBUM_IMPORT_FOLDER: &str = "import";
pub const USER_UPLOAD_FOLDER: &str = "uploads";
pub const TAKEOUT_FOLDER: &str = "takeout";

/// Subfolders owned by a dedicated pipeline (album import, takeout import).
///
/// These folder names live directly inside a user's media folder. Those
/// pipelines create the media items and enqueue their ingest jobs themselves;
/// generic file-discovery paths (watcher events, scheduled scans) must not
/// pick these files up while the pipeline has not run yet.
pub const MANAGED_FOLDERS: [&str; 2] = [ALBUM_IMPORT_FOLDER, TAKEOUT_FOLDER];

/// Returns true when `path` points inside one of the `managed` folders.
///
/// Managed folders always sit directly below a user's media folder, which is
/// itself a single directory name below the media root — possibly empty. So
/// the managed name can only be the first or second [`Path`] component; a
/// deeper same-named folder belongs to the user and is *not* managed.
#[must_use]
pub fn path_enters_managed_folder(path: &Path, managed: &[&str]) -> bool {
    path.components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .take(2)
        .any(|c| {
            matches!(&c, Component::Normal(name)
            if managed.iter().any(|m| name.eq_ignore_ascii_case(m)))
        })
}

// app_data folders
pub const CACHE_FOLDER: &str = "cache";
pub const ON_DEMAND_THUMBNAIL_CACHE_FOLDER: &str = "cache/on-demand-thumbnails";
pub const TUS_UPLOADS_FOLDER: &str = "tus/uploads";
pub const TUS_LOCKS_FOLDER: &str = "tus/locks";
pub const HUGGINGFACE_CACHE_FOLDER: &str = "cache/huggingface";
// hosted folders (in app_data)
pub const HOSTED_FOLDER: &str = "hosted";
pub const PANO_FOLDER: &str = "hosted/pano";
pub const THUMBNAILS_FOLDER: &str = "hosted/thumbnails";
pub const FACE_CLUSTERS_FOLDER: &str = "hosted/face-clusters";
