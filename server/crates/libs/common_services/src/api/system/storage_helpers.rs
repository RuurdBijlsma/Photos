use crate::api::admin::interfaces::PathInfoResponse;
use crate::api::app_error::AppError;
use crate::api::system::interfaces::DiskInfo;
use app_state::to_posix_string;
use fs2::statvfs;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use tempfile::NamedTempFile;
use walkdir::WalkDir;

/// Measures the recursive folder size of files on disk.
pub fn get_folder_size(path: &Path) -> u64 {
    if !path.exists() || !path.is_dir() {
        return 0;
    }
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.metadata().map_or(0, |m| m.len()))
        .sum()
}

pub static ARE_SAME_DRIVE: OnceLock<bool> = OnceLock::new();
/// Identifies if the media folder and the thumbnail folder reside on the same drive.
#[must_use]
pub fn are_on_same_drive(p1: &Path, p2: &Path) -> bool {
    let p1_canon = fs::canonicalize(p1).unwrap_or_else(|_| p1.to_path_buf());
    let p2_canon = fs::canonicalize(p2).unwrap_or_else(|_| p2.to_path_buf());

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(m1), Ok(m2)) = (std::fs::metadata(&p1_canon), std::fs::metadata(&p2_canon)) {
            return m1.dev() == m2.dev();
        }
    }

    p1_canon.components().next() == p2_canon.components().next()
}

pub fn get_single_disk_info(folder: &Path) -> Result<DiskInfo, AppError> {
    let fs_stats = statvfs(folder)?;
    let available = fs_stats.available_space();
    let total = fs_stats.total_space();

    Ok(DiskInfo {
        disk_available: available,
        disk_total: total,
        disk_used: total.saturating_sub(available),
    })
}

pub fn check_drive_info(folder: &Path) -> Result<PathInfoResponse, AppError> {
    let (read, write) = check_read_write_access(folder);
    let disk_info = get_single_disk_info(folder)?;

    Ok(PathInfoResponse {
        folder: to_posix_string(folder),
        disk_available: disk_info.disk_available,
        disk_used: disk_info.disk_used,
        disk_total: disk_info.disk_total,
        read_access: read,
        write_access: write,
    })
}

#[must_use]
pub fn check_read_write_access(path: &Path) -> (bool, bool) {
    let can_read = fs::read_dir(path).is_ok();
    let can_write = NamedTempFile::new_in(path).is_ok();
    (can_read, can_write)
}
