use crate::api::app_error::AppError;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tokio::task;

pub async fn list_folders(user_folder: &Path) -> Result<Vec<PathBuf>, AppError> {
    let path_buf = user_folder.to_path_buf();
    task::spawn_blocking(move || {
        fs::read_dir(path_buf)?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|ft| ft.is_dir()))
            .map(|entry| Ok(entry.path()))
            .collect::<Result<Vec<_>, io::Error>>()
    })
    .await?
    .map_err(AppError::from)
}
