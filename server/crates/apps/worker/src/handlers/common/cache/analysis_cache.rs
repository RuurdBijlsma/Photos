use crate::handlers::common::cache::common_cache::{ensure_parent_dir, get_json_path};
use color_eyre::Result;
use color_eyre::eyre::bail;
use common_types::ml_analysis::MLFastAnalysis;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tracing::warn;

const ANALYSIS_DIR: &str = "analysis";
const ANALYSIS_CACHE_VERSION: u32 = 1;
const EXPECTED_EMBEDDING_LENGTH: usize = 768;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CachedAnalysisResult {
    pub fast_analyses: Vec<MLFastAnalysis>,
    pub version: u32,
}

pub async fn get_analysis_cache(
    cache_root: &Path,
    hash: &str,
) -> Result<Option<Vec<MLFastAnalysis>>> {
    let path = get_json_path(cache_root, ANALYSIS_DIR, hash);
    if !path.exists() {
        return Ok(None);
    }

    let data = fs::read_to_string(&path).await?;
    if let Ok(cached) = serde_json::from_str::<CachedAnalysisResult>(&data)
        && cached.version == ANALYSIS_CACHE_VERSION
    {
        if let Some(va) = cached.fast_analyses.first()
            && va.embedding.len() != EXPECTED_EMBEDDING_LENGTH
        {
            return Ok(None);
        }
        return Ok(Some(cached.fast_analyses));
    }

    warn!("Invalid analysis cache, deleting: {:?}", path);
    let _ = fs::remove_file(&path).await;
    Ok(None)
}

pub async fn write_analysis_cache(
    cache_root: &Path,
    hash: &str,
    analyses: &[MLFastAnalysis],
) -> Result<()> {
    for analysis in analyses {
        if analysis.embedding.len() != EXPECTED_EMBEDDING_LENGTH {
            bail!(
                "Incorrect embedding length: expected {}, found {}",
                EXPECTED_EMBEDDING_LENGTH,
                analysis.embedding.len()
            );
        }
    }

    let path = get_json_path(cache_root, ANALYSIS_DIR, hash);
    ensure_parent_dir(&path).await?;

    let json = serde_json::to_string(&CachedAnalysisResult {
        version: ANALYSIS_CACHE_VERSION,
        fast_analyses: Vec::from(analyses),
    })?;
    fs::write(path, json).await?;
    Ok(())
}
