use crate::handlers::common::cache::common_cache::{ensure_parent_dir, get_json_path};
use color_eyre::Result;
use media_analyzer::MediaMetadata;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tracing::warn;

const INGEST_DIR: &str = "ingest";
const INGEST_CACHE_VERSION: u32 = 2;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CachedIngestResult {
    pub media_metadata: MediaMetadata,
    pub version: u32,
}

pub async fn get_ingest_cache(cache_root: &Path, hash: &str) -> Result<Option<MediaMetadata>> {
    let path = get_json_path(cache_root, INGEST_DIR, hash);
    if !path.exists() {
        return Ok(None);
    }

    let data = fs::read_to_string(&path).await?;
    if let Ok(cached) = serde_json::from_str::<CachedIngestResult>(&data)
        && cached.version == INGEST_CACHE_VERSION
    {
        return Ok(Some(cached.media_metadata));
    }

    warn!("Invalid ingest cache, deleting: {:?}", path);
    let _ = fs::remove_file(&path).await;
    Ok(None)
}

pub async fn write_ingest_cache(
    cache_root: &Path,
    hash: &str,
    media_metadata: MediaMetadata,
) -> Result<()> {
    let path = get_json_path(cache_root, INGEST_DIR, hash);
    ensure_parent_dir(&path).await?;

    let json = serde_json::to_string(&CachedIngestResult {
        version: INGEST_CACHE_VERSION,
        media_metadata,
    })?;
    fs::write(path, json).await?;
    Ok(())
}
