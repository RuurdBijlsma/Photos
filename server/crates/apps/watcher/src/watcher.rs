use crate::handlers::{handle_create, handle_remove};
use app_state::IngestSettings;
use app_state::constants::ALBUM_IMPORT_FOLDER;
use color_eyre::eyre::{Result, eyre};
use common_services::alert;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use sqlx::PgPool;
use std::path::Component;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

const EXCLUDED_WATCH_FOLDER: [&str; 1] = [ALBUM_IMPORT_FOLDER];

pub async fn start_watching(pool: &PgPool, settings: &IngestSettings) -> Result<()> {
    if let Err(e) = run(pool, settings).await {
        alert!("Watcher failed with an error: {}", e);
    }

    Ok(())
}

/// Runs the file system watcher.
///
/// This function sets up a channel to receive file system events and processes them
/// in a loop. Each event is handled in a separate asynchronous task.
async fn run(pool: &PgPool, settings: &IngestSettings) -> notify::Result<()> {
    let (tx, mut rx) = mpsc::channel(100);

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Err(e) = tx.blocking_send(res) {
                error!("Failed to send event through channel: {}", e);
            }
        },
        Config::default(),
    )?;

    watcher.watch(&settings.media_root, RecursiveMode::Recursive)?;
    info!("👁️ Watcher started on: {:?}", &settings.media_root);

    while let Some(result) = rx.recv().await {
        let pool = pool.clone();
        let settings = settings.clone();
        tokio::spawn(async move {
            let event = match result {
                Ok(evt) => evt,
                Err(err) => {
                    error!("Watch error: {:?}", err);
                    return;
                }
            };
            if let Err(e) = process_event(&pool, &settings, event).await {
                warn!("Error while processing file watcher event {e}");
            }
        });
    }

    Ok(())
}

/// Processes a single file system event from the watcher.
async fn process_event(pool: &PgPool, settings: &IngestSettings, event: Event) -> Result<()> {
    let Some(path) = event.paths.first() else {
        return Ok(());
    };

    // Ignore temporary or hidden files.
    if path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|s| s.starts_with('.'))
    {
        info!("Ignoring hidden file event for: {:?}", path);
        return Ok(());
    }

    let rel_path = path.strip_prefix(&settings.media_root)?;
    if let Some(Component::Normal(name)) = rel_path.components().next()
        && EXCLUDED_WATCH_FOLDER.contains(&name.to_string_lossy().as_ref())
    {
        return Ok(());
    }

    let result = match event.kind {
        EventKind::Create(_) => handle_create(pool, settings, path).await,
        EventKind::Remove(_) => handle_remove(pool, settings, path).await,
        _ => Err(eyre!("Unknown event type")),
    };

    if let Err(e) = result {
        warn!("Error handling file event for {:?}: {:?}", path, e);
    }
    Ok(())
}
