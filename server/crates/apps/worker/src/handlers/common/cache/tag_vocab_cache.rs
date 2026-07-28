use color_eyre::Result;
use rkyv::{Archive, Deserialize, Serialize, access, deserialize, rancor::Error, to_bytes};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

const STATIC_CACHE_DIR: &str = "assets/static-cache/tag-vocab";
const TAG_VOCAB_CACHE_VERSION: u32 = 1;

#[derive(Archive, Deserialize, Serialize, Debug)]
pub struct CachedTagVocabResult {
    pub version: u32,
    pub tags: HashMap<String, Vec<f32>>,
}

fn get_cache_file_path(model_id: &str) -> PathBuf {
    let sanitized = model_id.replace(['/', '\\', ':', ' '], "_");
    Path::new(STATIC_CACHE_DIR).join(format!("{sanitized}.bin"))
}

pub async fn load_tag_vocab_cache(model_id: &str) -> Result<HashMap<String, Vec<f32>>> {
    let path = get_cache_file_path(model_id);
    if !path.exists() {
        return Ok(HashMap::new());
    }

    match fs::read(&path).await {
        Ok(bytes) => match access::<ArchivedCachedTagVocabResult, Error>(&bytes) {
            Ok(archived) if archived.version == TAG_VOCAB_CACHE_VERSION => {
                match deserialize::<CachedTagVocabResult, Error>(archived) {
                    Ok(cached) => {
                        debug!(
                            "Loaded {} cached tag embeddings from {:?}",
                            cached.tags.len(),
                            path
                        );
                        Ok(cached.tags)
                    }
                    Err(e) => {
                        warn!(
                            "Failed to deserialize static tag vocab cache at {:?}: {e}",
                            path
                        );
                        Ok(HashMap::new())
                    }
                }
            }
            Ok(archived) => {
                warn!(
                    "Tag vocab cache version mismatch at {:?} (expected {}, found {})",
                    path, TAG_VOCAB_CACHE_VERSION, archived.version
                );
                Ok(HashMap::new())
            }
            Err(e) => {
                warn!("Invalid binary cache format at {:?}: {e}", path);
                Ok(HashMap::new())
            }
        },
        Err(e) => {
            warn!(
                "Failed to read static tag vocab cache file at {:?}: {e}",
                path
            );
            Ok(HashMap::new())
        }
    }
}

#[allow(clippy::implicit_hasher)]
pub async fn save_tag_vocab_cache(model_id: &str, cache: &HashMap<String, Vec<f32>>) -> Result<()> {
    let path = get_cache_file_path(model_id);

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await?;
        }
    }

    let payload = CachedTagVocabResult {
        version: TAG_VOCAB_CACHE_VERSION,
        tags: cache.clone(),
    };

    let bytes = to_bytes::<Error>(&payload)?;

    // Log a warning on write errors (e.g. if container filesystem is read-only at runtime)
    if let Err(e) = fs::write(&path, bytes.as_slice()).await {
        warn!("Could not update static tag vocab cache at {:?}: {e}", path);
    }

    Ok(())
}
