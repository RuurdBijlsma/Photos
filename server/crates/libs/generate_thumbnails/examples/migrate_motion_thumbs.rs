use app_state::load_app_settings;
use color_eyre::Result;
use generate_thumbnails::generate_motion_thumbs;
use sqlx::PgPool;
use tracing::{Level, error, info, warn};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set up tracing subscriber");
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")?;

    info!("Connecting to database...");
    let pool = PgPool::connect(&database_url).await?;
    info!("Database connection established.");

    info!("Loading app settings...");
    let settings = load_app_settings()?;
    let media_root = &settings.ingest.media_root;
    let thumbnails_root = &settings.ingest.thumbnails_root;

    info!(
        media_root = %media_root.display(),
        thumbnails_root = %thumbnails_root.display(),
        "Paths resolved."
    );

    // Fetch items that are labeled as motion photos, including their hash
    info!("Querying database for motion photo candidates...");
    let rows = sqlx::query!(
        "SELECT mi.id, mi.relative_path, mi.hash
         FROM media_item mi
         INNER JOIN media_features mf ON mf.media_item_id = mi.id
         WHERE mf.is_motion_photo = TRUE"
    )
    .fetch_all(&pool)
    .await?;

    info!("Found {} motion photo records to verify.", rows.len());

    let mut processed = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for row in rows {
        let id: String = row.id;
        let relative_path: String = row.relative_path;
        let hash: String = row.hash;

        let source_path = media_root.join(&relative_path);
        let out_folder = thumbnails_root.join(&id);
        let motion_mp4_path = out_folder.join("motion.mp4");

        if motion_mp4_path.exists() {
            info!(
                id = %id,
                path = %relative_path,
                "motion.mp4 already exists locally. Checking cache status..."
            );
            skipped += 1;
        } else {
            if !source_path.exists() {
                warn!(
                    id = %id,
                    path = %source_path.display(),
                    "Source file not found on disk. Skipping."
                );
                failed += 1;
                continue;
            }

            // Ensure parent output folder is created
            if !out_folder.exists()
                && let Err(e) = std::fs::create_dir_all(&out_folder)
            {
                error!(
                    id = %id,
                    error = %e,
                    "Failed to create output directory."
                );
                failed += 1;
                continue;
            }

            info!(
                id = %id,
                file = %relative_path,
                "Extracting motion photo elements..."
            );

            match generate_motion_thumbs(&source_path, &out_folder) {
                Ok(true) => {
                    info!(id = %id, "Successfully extracted motion video.");
                    processed += 1;
                }
                Ok(false) => {
                    warn!(
                        id = %id,
                        "No valid embedded or companion motion photo video recovered (returned false)."
                    );
                    failed += 1;
                    continue; // Skip cache synchronization
                }
                Err(e) => {
                    error!(
                        id = %id,
                        error = ?e,
                        "Failed during extraction process."
                    );
                    failed += 1;
                    continue; // Skip cache synchronization
                }
            }
        }

        // Cache Sync Pass:
        // Even if motion.mp4 existed locally (skipped extraction), it might still be missing from the cache folder.
        if settings.ingest.enable_cache && motion_mp4_path.exists() {
            let cache_thumbs_dir = settings
                .ingest
                .cache_root
                .join("thumbnails")
                .join(&hash)
                .join("thumbs");
            if cache_thumbs_dir.exists() {
                let cached_motion_mp4 = cache_thumbs_dir.join("motion.mp4");
                if cached_motion_mp4.exists() {
                    tracing::debug!(id = %id, "motion.mp4 already present in cache.");
                } else {
                    info!(id = %id, "Syncing motion.mp4 to cache directory...");

                    // Try to hard link first to preserve disk space; fall back to a copy
                    if let Err(link_err) = std::fs::hard_link(&motion_mp4_path, &cached_motion_mp4)
                    {
                        tracing::debug!(
                            id = %id,
                            "Hard linking motion.mp4 to cache failed ({:?}), falling back to copy",
                            link_err
                        );
                        if let Err(copy_err) = std::fs::copy(&motion_mp4_path, &cached_motion_mp4) {
                            error!(
                                id = %id,
                                error = ?copy_err,
                                "Failed to copy motion.mp4 to cache directory."
                            );
                        } else {
                            info!(id = %id, "Successfully copied motion.mp4 to cache directory.");
                        }
                    } else {
                        info!(id = %id, "Successfully hard-linked motion.mp4 to cache directory.");
                    }
                }
            } else {
                tracing::debug!(
                    id = %id,
                    "Cache directory for hash {} does not exist. Skipping cache sync.",
                    hash
                );
            }
        }
    }

    info!(
        "Extraction and cache sync pass finished. Summary: Extracted: {}, Skipped (Already Local): {}, Unresolved/Failed: {}",
        processed, skipped, failed
    );

    Ok(())
}
