use crate::handlers::common::cache::common_cache::{ensure_parent_dir, get_json_path};
use color_eyre::Result;
use common_types::ml_analysis::MLChatAnalysis;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tracing::warn;

const LLM_DIR: &str = "llm";
const LLM_CACHE_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CachedLlmResult {
    pub llm_analyses: Vec<MLChatAnalysis>,
    pub version: u32,
}

pub async fn get_llm_cache(cache_root: &Path, hash: &str) -> Result<Option<Vec<MLChatAnalysis>>> {
    let path = get_json_path(cache_root, LLM_DIR, hash);
    if !path.exists() {
        return Ok(None);
    }

    let data = fs::read_to_string(&path).await?;
    if let Ok(cached) = serde_json::from_str::<CachedLlmResult>(&data)
        && cached.version == LLM_CACHE_VERSION
    {
        return Ok(Some(cached.llm_analyses));
    }

    warn!("Invalid LLM cache, deleting: {:?}", path);
    let _ = fs::remove_file(&path).await;
    Ok(None)
}

pub async fn write_llm_cache(
    cache_root: &Path,
    hash: &str,
    analyses: &[MLChatAnalysis],
) -> Result<()> {
    let path = get_json_path(cache_root, LLM_DIR, hash);
    ensure_parent_dir(&path).await?;

    let json = serde_json::to_string(&CachedLlmResult {
        version: LLM_CACHE_VERSION,
        llm_analyses: Vec::from(analyses),
    })?;
    fs::write(path, json).await?;
    Ok(())
}
