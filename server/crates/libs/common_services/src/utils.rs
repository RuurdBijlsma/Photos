use std::ffi::OsStr;
use std::fmt::Display;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::warn;

/// Generate a URL-safe random ID of a given length.
#[must_use]
pub fn nice_id(length: usize) -> String {
    const URL_SAFE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_";
    (0..length)
        .map(|_| {
            let idx = rand::random_range(0..URL_SAFE.len());
            URL_SAFE[idx] as char
        })
        .collect()
}

/// Logs a warning message with an 'ALERT:' prefix.
#[macro_export]
macro_rules! alert {
    ($($arg:tt)*) => {
        warn!("ALERT: {}", format_args!($($arg)*));
    };
}

/// Write/update the EXIF Orientation tag on a media file on disk.
pub fn write_exif_orientation(file_path: &Path, orientation: i32) -> color_eyre::Result<()> {
    let et = exiftool::ExifTool::new()?;
    et.write_tag(
        file_path,
        "all:Orientation",
        &orientation.to_string(),
        &["-n", "-overwrite_original"],
    )?;
    Ok(())
}

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
