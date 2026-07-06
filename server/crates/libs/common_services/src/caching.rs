use std::path::Path;

pub fn hash_file(path: &Path) -> color_eyre::Result<String> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_mmap_rayon(path)?;
    let hash = hasher.finalize();
    Ok(hash.to_hex().to_string())
}
