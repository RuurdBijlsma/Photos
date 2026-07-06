use std::path::{Path, PathBuf};
use tokio::fs;

/// Helper to get the path for a JSON cache file: cache/{category}/{hash}.json
#[must_use]
pub fn get_json_path(cache_root: &Path, category: &str, hash: &str) -> PathBuf {
    cache_root.join(category).join(format!("{hash}.json"))
}

/// Helper to ensure the parent directory exists before writing a file
pub async fn ensure_parent_dir(path: &Path) -> color_eyre::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).await?;
    }

    Ok(())
}
