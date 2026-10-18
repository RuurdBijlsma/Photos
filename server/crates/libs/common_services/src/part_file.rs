//! Staged writes into the media root.
//!
//! Writers of media files (takeout extraction, s2s album download, …) must
//! never leave partially-written bytes under a media-visible name: the file
//! watcher's hidden-file filter and scan.rs's extension filter would skip the
//! staging name, but an ingested incomplete file would keep its wrong hash and
//! metadata. The pattern here copies into a hidden `.part` file next to the
//! destination and renames it into place only once complete — first writer
//! wins, so concurrent writers never clobber each other.

use std::ffi::OsStr;
use std::fmt::Display;
use std::fs;
use std::io;
use std::ops::AsyncFnOnce;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::warn;

/// Builds the hidden `.part` staging path for a pending write to `destination`.
///
/// `tag` must be unique per concurrent writer (e.g. process id + sequence
/// number), so simultaneous writers never stage into the same temp file. The
/// staging file lives next to its destination, which keeps the completing
/// rename same-volume. The leading dot and the `.part` suffix keep it invisible
/// to both the file watcher's hidden-file filter and scan.rs's extension
/// filter, so partially-written bytes can never be discovered and ingested
/// under a media-visible name.
#[must_use]
pub fn part_file_path(destination: &Path, tag: impl Display) -> PathBuf {
    let file_name = destination
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("file");
    destination.with_file_name(format!("{file_name}.{tag}.part"))
}

/// Renames a completed `.part` staging file onto its final destination.
///
/// First writer wins: the staging file's bytes only replace a *missing*
/// destination. When the destination already holds a file (another import
/// finished first), that file's bytes win and the staging file is removed —
/// `Ok(false)` signals that outcome, which callers should treat exactly like
/// finding the destination already present. `Err` means the destination could
/// not be populated; the staging file is left on disk for stale cleanup and
/// the caller should abandon its copy.
///
/// # Errors
///
/// Propagates the rename failure if the destination was not materialized.
pub fn finalize_part_file(temp: &Path, destination: &Path) -> io::Result<bool> {
    match fs::rename(temp, destination) {
        Ok(()) => Ok(true),
        Err(e) => {
            if fs::metadata(destination).is_ok_and(|m| m.is_file()) {
                if let Err(del) = fs::remove_file(temp)
                    && del.kind() != io::ErrorKind::NotFound
                {
                    warn!("Failed to delete staged file {:?}: {}", temp, del);
                }
                Ok(false)
            } else {
                Err(e)
            }
        }
    }
}

/// Removes hidden `.part` staging files abandoned by a writer that crashed or
/// was killed mid-copy.
///
/// Only files untouched for longer than `stale_after` are removed, so a
/// staging file still being written (its mtime updates on every chunk) is
/// never deleted. Best-effort: failures are only logged.
pub fn remove_stale_part_files(folder: &Path, stale_after: Duration) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        let is_staged = name.starts_with('.')
            && Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("part"));
        let is_stale = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > stale_after);
        if is_staged
            && is_stale
            && let Err(e) = fs::remove_file(&path)
            && e.kind() != io::ErrorKind::NotFound
        {
            warn!("Failed to remove stale staged file {:?}: {}", path, e);
        }
    }
}

/// Copies bytes into a hidden `.part` staging file next to `destination` and
/// renames the finished copy into place, first writer wins.
///
/// `tag` must be unique per concurrent writer; see [`part_file_path`] for the
/// staging name's properties. On any failure the staging file is removed, so
/// only a hard process kill mid-write leaves one behind (for the stale sweep).
///
/// Returns `Ok(true)` when this call's bytes landed at `destination`, and
/// `Ok(false)` when the destination already held a file (another writer
/// finished first) whose bytes won instead — callers should treat that exactly
/// like finding the destination preexisting.
///
/// # Errors
///
/// Write or finalize errors. An occupied destination is not an error.
pub fn stage_part_file(
    destination: &Path,
    tag: impl Display,
    write: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> io::Result<bool> {
    let temp = part_file_path(destination, tag);
    let write_result = fs::File::create(&temp).and_then(|mut file| write(&mut file));
    match write_result {
        Err(e) => {
            discard_staged_file(&temp);
            Err(e)
        }
        Ok(()) => match finalize_part_file(&temp, destination) {
            Err(e) => {
                discard_staged_file(&temp);
                Err(e)
            }
            Ok(item_landed) => Ok(item_landed),
        },
    }
}

/// [`stage_part_file`] for async writers (e.g. streaming an HTTP response
/// body). Same staging, cleanup and first-writer-wins semantics.
///
/// # Errors
///
/// Write or finalize errors. An occupied destination is not an error.
pub async fn stage_part_file_async(
    destination: &Path,
    tag: impl Display,
    write: impl AsyncFnOnce(&mut tokio::fs::File) -> color_eyre::Result<()>,
) -> color_eyre::Result<bool> {
    let temp = part_file_path(destination, tag);
    let write_result: color_eyre::Result<()> = async {
        let mut file = tokio::fs::File::create(&temp).await?;
        write(&mut file).await
    }
    .await;
    match write_result {
        Err(e) => {
            discard_staged_file_async(&temp).await;
            Err(e)
        }
        Ok(()) => match finalize_part_file(&temp, destination) {
            Err(e) => {
                discard_staged_file_async(&temp).await;
                Err(color_eyre::eyre::Report::from(e))
            }
            Ok(item_landed) => Ok(item_landed),
        },
    }
}

/// Removes a staging file whose write or finalize failed. The `NotFound` kind
/// is not an error: the file may already be gone.
fn discard_staged_file(temp: &Path) {
    if let Err(e) = fs::remove_file(temp)
        && e.kind() != io::ErrorKind::NotFound
    {
        warn!("Failed to remove staged file {:?}: {}", temp, e);
    }
}

async fn discard_staged_file_async(temp: &Path) {
    if let Err(e) = tokio::fs::remove_file(temp).await
        && e.kind() != io::ErrorKind::NotFound
    {
        warn!("Failed to remove staged file {:?}: {}", temp, e);
    }
}
