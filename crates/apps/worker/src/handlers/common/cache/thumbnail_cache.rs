use color_eyre::Result;
use generate_thumbnails::link_or_copy_dir_contents;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tracing::{debug, warn};

// Category folder names
const THUMBNAILS_DIR: &str = "thumbnails";
const THUMBNAILS_CACHE_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CachedThumbnailMetadata {
    pub version: u32,
    pub has_panorama: bool,
}

pub async fn get_thumbnail_cache(
    cache_root: &Path,
    hash: &str,
    thumbnails_dest: &Path,
    pano_dest: &Path,
    require_panorama: bool,
) -> Result<bool> {
    let cache_item_dir = cache_root.join(THUMBNAILS_DIR).join(hash);
    let metadata_path = cache_item_dir.join("metadata.json");

    if !metadata_path.exists() {
        return Ok(false);
    }

    let data = fs::read_to_string(&metadata_path).await?;
    let cached: CachedThumbnailMetadata = if let Ok(c) = serde_json::from_str(&data) {
        c
    } else {
        warn!(
            "Invalid thumbnail cache metadata, deleting: {:?}",
            cache_item_dir
        );
        let _ = fs::remove_dir_all(&cache_item_dir).await;
        return Ok(false);
    };

    // Check version
    if cached.version != THUMBNAILS_CACHE_VERSION {
        warn!(
            "Thumbnail cache version mismatch (expected {}, found {}), deleting: {:?}",
            THUMBNAILS_CACHE_VERSION, cached.version, cache_item_dir
        );
        let _ = fs::remove_dir_all(&cache_item_dir).await;
        return Ok(false);
    }

    // Ensure we aren't using a non-pano cache if a panorama is required
    if require_panorama && !cached.has_panorama {
        debug!("Cache exists but does not contain required panorama.");
        return Ok(false);
    }

    // Restore thumbnails folder using hard links
    let cached_thumbs = cache_item_dir.join("thumbs");
    if cached_thumbs.exists() {
        if !thumbnails_dest.exists() {
            fs::create_dir_all(thumbnails_dest).await?;
        }
        // Changed to link_or_copy_dir_contents:
        link_or_copy_dir_contents(&cached_thumbs, thumbnails_dest).await?;
    } else {
        return Ok(false);
    }

    // Restore panorama folder if it was cached
    if cached.has_panorama {
        let cached_pano = cache_item_dir.join("pano");
        if cached_pano.exists() {
            if !pano_dest.exists() {
                fs::create_dir_all(pano_dest).await?;
            }
            // Changed to link_or_copy_dir_contents:
            link_or_copy_dir_contents(&cached_pano, pano_dest).await?;
        } else {
            return Ok(false);
        }
    }

    Ok(true)
}

pub async fn write_thumbnail_cache(
    cache_root: &Path,
    hash: &str,
    thumbnails_src: &Path,
    pano_src: &Path,
    has_panorama: bool,
) -> Result<()> {
    let cache_item_dir = cache_root.join(THUMBNAILS_DIR).join(hash);
    if cache_item_dir.exists() {
        fs::remove_dir_all(&cache_item_dir).await?;
    }
    fs::create_dir_all(&cache_item_dir).await?;

    // Hard link thumbnails to cache
    let cached_thumbs = cache_item_dir.join("thumbs");
    fs::create_dir_all(&cached_thumbs).await?;
    link_or_copy_dir_contents(thumbnails_src, &cached_thumbs).await?;

    // Hard link panorama to cache if requested and output directory exists
    let actual_has_panorama = has_panorama && pano_src.exists();
    if actual_has_panorama {
        let cached_pano = cache_item_dir.join("pano");
        fs::create_dir_all(&cached_pano).await?;
        // Changed to link_or_copy_dir_contents:
        link_or_copy_dir_contents(pano_src, &cached_pano).await?;
    }

    // Write metadata file
    let metadata_path = cache_item_dir.join("metadata.json");
    let metadata = CachedThumbnailMetadata {
        version: THUMBNAILS_CACHE_VERSION,
        has_panorama: actual_has_panorama,
    };
    let json = serde_json::to_string(&metadata)?;
    fs::write(metadata_path, json).await?;

    Ok(())
}

pub async fn delete_thumbnail_cache(cache_root: &Path, hash: &str) -> Result<()> {
    let dest_folder = cache_root.join(THUMBNAILS_DIR).join(hash);
    if dest_folder.exists() {
        fs::remove_dir_all(&dest_folder).await?;
    }
    Ok(())
}
