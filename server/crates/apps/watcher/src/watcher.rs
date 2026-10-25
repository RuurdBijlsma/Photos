use crate::handlers::handle_create;
use app_state::IngestSettings;
use app_state::constants::MANAGED_FOLDERS;
use app_state::to_posix_string;
use color_eyre::eyre::Result;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use sqlx::PgPool;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use tokio::sync::{mpsc, watch};
use tokio::time::{Instant, sleep_until};
use tracing::{error, info, warn};

const BATCH_WINDOW: std::time::Duration = std::time::Duration::from_millis(1000);
const CHANNEL_CAPACITY: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchAction {
    Upsert,
    Remove,
}

/// Runs the file system watcher.
///
/// Events are collected for a fixed window and then flushed as one batch. Only
/// creates and removals are handled; renames are deliberately not interpreted.
pub async fn create_watcher(
    pool: &PgPool,
    settings: &IngestSettings,
    mut shutdown_rx: watch::Receiver<bool>,
) -> notify::Result<()> {
    let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Err(e) = tx.blocking_send(res) {
                error!("Failed to send event through channel: {}", e);
            }
        },
        Config::default(),
    )?;

    watcher.watch(&settings.media_root, RecursiveMode::Recursive)?;
    info!("\u{1f440} Watcher started on: {:?}", &settings.media_root);

    let mut batch: HashMap<PathBuf, BatchAction> = HashMap::new();
    let mut deadline: Option<Instant> = None;

    loop {
        if *shutdown_rx.borrow() {
            log_flush_result(flush_batch(pool, settings, &batch).await);
            info!("Shutdown requested. Exiting watcher loop.");
            break;
        }

        tokio::select! {
            maybe_result = rx.recv() => {
                let Some(result) = maybe_result else {
                    log_flush_result(flush_batch(pool, settings, &batch).await);
                    break;
                };
                match result {
                    Ok(event) => {
                        if let Some(action) = action_for_event(event.kind, &event.paths) {
                            for path in event.paths {
                                batch.insert(path, action);
                            }
                            if deadline.is_none() {
                                deadline = Some(Instant::now() + BATCH_WINDOW);
                            }
                        }
                    }
                    Err(err) => error!("Watch error: {:?}", err),
                }
            }
            () = sleep_until(deadline.unwrap_or_else(Instant::now)), if deadline.is_some() => {
                log_flush_result(flush_batch(pool, settings, &batch).await);
                batch.clear();
                deadline = None;
            }
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    log_flush_result(flush_batch(pool, settings, &batch).await);
                    info!("Shutdown signal received. Exiting watcher loop.");
                    break;
                }
            }
        }
    }

    Ok(())
}

fn log_flush_result(result: Result<()>) {
    if let Err(e) = result {
        warn!("Error while flushing watcher batch: {e:?}");
    }
}

const fn action_for_event(kind: EventKind, paths: &[PathBuf]) -> Option<BatchAction> {
    if paths.is_empty() {
        return None;
    }
    match kind {
        EventKind::Create(_) => Some(BatchAction::Upsert),
        EventKind::Remove(_) => Some(BatchAction::Remove),
        _ => None,
    }
}

async fn flush_batch(
    pool: &PgPool,
    settings: &IngestSettings,
    batch: &HashMap<PathBuf, BatchAction>,
) -> Result<()> {
    let mut removals = Vec::new();
    let mut upserts = Vec::new();
    for (path, action) in batch {
        let Some(relative_path) = relative_path_for_event(path, settings) else {
            continue;
        };
        match action {
            BatchAction::Remove => removals.push(relative_path),
            BatchAction::Upsert => upserts.push((relative_path, path.clone())),
        }
    }

    if !removals.is_empty() {
        let result =
            common_services::media_reconcile::reconcile_removed_paths(pool, settings, &removals)
                .await?;
        info!(
            "Reconciled {} removed path(s): {} marked missing, {} job(s) cancelled",
            removals.len(),
            result.marked_missing,
            result.cancelled_jobs
        );
    }

    for (relative_path, path) in upserts {
        if !path.exists() {
            info!("Skipping vanished path during watcher flush: {relative_path}");
            continue;
        }
        if let Err(e) = handle_create(pool, settings, &path).await {
            warn!("Error handling file event for {relative_path}: {e:?}");
        }
    }

    Ok(())
}

fn relative_path_for_event(path: &Path, settings: &IngestSettings) -> Option<String> {
    if path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|s| s.starts_with('.'))
    {
        return None;
    }

    let rel_path = path.strip_prefix(&settings.media_root).ok()?;
    if rel_path.components().any(|c| {
        if let Component::Normal(name) = c {
            MANAGED_FOLDERS.contains(&name.to_string_lossy().as_ref())
        } else {
            false
        }
    }) {
        return None;
    }

    let relative_path = to_posix_string(rel_path);
    if relative_path.is_empty() {
        None
    } else {
        Some(relative_path)
    }
}

#[cfg(test)]
mod tests {
    use super::{BatchAction, action_for_event};
    use notify::EventKind;
    use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
    use std::path::PathBuf;

    #[test]
    fn classifies_create_and_remove_only() {
        let paths = [PathBuf::from("photo.jpg")];
        assert_eq!(
            action_for_event(EventKind::Create(CreateKind::File), &paths),
            Some(BatchAction::Upsert)
        );
        assert_eq!(
            action_for_event(EventKind::Remove(RemoveKind::File), &paths),
            Some(BatchAction::Remove)
        );
        assert_eq!(
            action_for_event(
                EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
                &paths
            ),
            None
        );
        assert_eq!(
            action_for_event(EventKind::Remove(RemoveKind::File), &[]),
            None
        );
    }
}
