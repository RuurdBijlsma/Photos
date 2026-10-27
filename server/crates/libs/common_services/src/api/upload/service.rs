use crate::api::app_error::AppError;
use crate::api::auth::interfaces::AuthClaims;
use jsonwebtoken::{DecodingKey, Validation, decode};
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

pub fn decode_upload_token(token: &str, jwt_secret: &str) -> Result<AuthClaims, AppError> {
    let mut validation = Validation::default();
    validation.set_audience(&["upload"]);

    decode::<AuthClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized("Bad token".to_string()))
}

#[derive(serde::Deserialize)]
struct TusUploadInfo {
    id: String,
    size: Option<u64>,
    offset: u64,
}

/// Rebuilds a completed tus upload whose final assembly was interrupted.
///
/// `FileLoft` stores PATCH chunks as `_part_*` and marks `.info` complete before
/// concatenating them. A cancelled request (browser refresh, server restart) leaves
/// the final file as a prefix. Returns `true` if the final file was rebuilt.
pub async fn repair_tus_upload(tus_dir: &Path, upload_id: &str) -> std::io::Result<bool> {
    let info_path = tus_dir.join(format!("{upload_id}.info"));
    let info_json = match tokio::fs::read_to_string(&info_path).await {
        Ok(json) => json,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    let info: TusUploadInfo = serde_json::from_str(&info_json)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("invalid upload info: {e}")))?;
    if info.id != upload_id {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("upload info id {} does not match {upload_id}", info.id),
        ));
    }
    let Some(size) = info.size else {
        return Ok(false);
    };
    if info.offset != size {
        return Ok(false);
    }

    let final_path = tus_dir.join(upload_id);
    let final_is_complete = match tokio::fs::metadata(&final_path).await {
        Ok(metadata) => metadata.len() == size,
        Err(e) if e.kind() == ErrorKind::NotFound => false,
        Err(e) => return Err(e),
    };

    let part_prefix = format!("{upload_id}_part_");
    let mut parts: Vec<(u32, PathBuf)> = Vec::new();
    let mut entries = tokio::fs::read_dir(tus_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let Some(index) = name
            .to_str()
            .and_then(|name| name.strip_prefix(&part_prefix))
            .and_then(|suffix| suffix.parse::<u32>().ok())
        else {
            continue;
        };
        parts.push((index, entry.path()));
    }
    if parts.is_empty() {
        return Ok(false);
    }
    if final_is_complete {
        // The final file is complete; any parts are leftovers from an interrupted cleanup.
        for (_, part_path) in &parts {
            if let Err(e) = tokio::fs::remove_file(part_path).await {
                tracing::warn!("Could not remove stale upload part {part_path:?}: {e}");
            }
        }
        tracing::warn!("Removed stale parts for completed upload {upload_id}");
        return Ok(true);
    }

    parts.sort_unstable_by_key(|(index, _)| *index);
    let mut part_bytes = 0u64;
    for (expected, (index, path)) in parts.iter().enumerate() {
        let expected = u32::try_from(expected)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "too many upload parts"))?;
        if *index != expected {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("upload {upload_id} has a gap in its parts at {expected}"),
            ));
        }
        part_bytes = part_bytes
            .checked_add(tokio::fs::metadata(path).await?.len())
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "upload parts are too large"))?;
    }
    if part_bytes != size {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("upload {upload_id} has {part_bytes} bytes in parts, expected {size}"),
        ));
    }

    // Write beside the final file so the rename is atomic; parts stay until the new file is durable.
    let temp_path = tus_dir.join(format!(".{upload_id}.repair"));
    if let Err(e) = copy_parts_to(&parts, &temp_path, size).await {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(e);
    }
    if let Err(e) = tokio::fs::rename(&temp_path, &final_path).await {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(e);
    }
    for (_, part_path) in &parts {
        if let Err(e) = tokio::fs::remove_file(part_path).await {
            tracing::warn!("Could not remove repaired upload part {part_path:?}: {e}");
        }
    }
    tracing::warn!("Repaired interrupted tus finalization for upload {upload_id} ({size} bytes)");
    Ok(true)
}

async fn copy_parts_to(
    parts: &[(u32, PathBuf)],
    temp_path: &Path,
    size: u64,
) -> std::io::Result<()> {
    let mut out = tokio::fs::File::create(temp_path).await?;
    let mut copied = 0u64;
    for (_, part_path) in parts {
        let mut part = tokio::fs::File::open(part_path).await?;
        copied = copied
            .checked_add(tokio::io::copy(&mut part, &mut out).await?)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "upload parts are too large"))?;
    }
    out.flush().await?;
    out.sync_all().await?;
    if copied != size {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("copied {copied} bytes, expected {size}"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::repair_tus_upload;
    use std::path::Path;

    fn write_info(dir: &Path, id: &str, size: u64, offset: u64) -> std::io::Result<()> {
        std::fs::write(
            dir.join(format!("{id}.info")),
            format!(
                r#"{{"id":"{id}","size":{size},"offset":{offset},"metadata":{{}},"size_is_deferred":false,"expires_at":null,"is_partial":false,"is_final":false,"partial_uploads":[],"storage":{{}}}}"#
            ),
        )
    }

    #[tokio::test]
    async fn repairs_completed_upload_from_parts() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let id = "upload-1";
        write_info(dir.path(), id, 11, 11)?;
        std::fs::write(dir.path().join(format!("{id}_part_0")), b"hello")?;
        std::fs::write(dir.path().join(format!("{id}_part_1")), b" world")?;
        std::fs::write(dir.path().join(id), b"hel")?;

        assert!(repair_tus_upload(dir.path(), id).await?);
        assert_eq!(std::fs::read(dir.path().join(id))?, b"hello world");
        assert!(!dir.path().join(format!("{id}_part_0")).exists());
        assert!(!dir.path().join(format!("{id}_part_1")).exists());
        assert!(!repair_tus_upload(dir.path(), id).await?);
        Ok(())
    }

    #[tokio::test]
    async fn removes_stale_parts_when_final_is_already_complete() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let id = "upload-4";
        write_info(dir.path(), id, 11, 11)?;
        std::fs::write(dir.path().join(format!("{id}_part_0")), b"hello")?;
        std::fs::write(dir.path().join(id), b"hello world")?;

        assert!(repair_tus_upload(dir.path(), id).await?);
        assert_eq!(std::fs::read(dir.path().join(id))?, b"hello world");
        assert!(!dir.path().join(format!("{id}_part_0")).exists());
        Ok(())
    }

    #[tokio::test]
    async fn ignores_upload_that_is_still_receiving_bytes() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let id = "upload-2";
        write_info(dir.path(), id, 11, 5)?;
        std::fs::write(dir.path().join(format!("{id}_part_0")), b"hello")?;
        std::fs::write(dir.path().join(id), b"hel")?;

        assert!(!repair_tus_upload(dir.path(), id).await?);
        assert_eq!(std::fs::read(dir.path().join(id))?, b"hel");
        assert!(dir.path().join(format!("{id}_part_0")).exists());
        Ok(())
    }

    #[tokio::test]
    async fn rejects_upload_with_incomplete_parts() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let id = "upload-3";
        write_info(dir.path(), id, 11, 11)?;
        std::fs::write(dir.path().join(format!("{id}_part_0")), b"hello")?;
        std::fs::write(dir.path().join(id), b"hel")?;

        let result = repair_tus_upload(dir.path(), id).await;
        assert!(result.is_err());
        assert_eq!(
            std::fs::read(dir.path().join(id))?,
            b"hel",
            "a failed repair must leave the existing final file untouched"
        );
        assert!(dir.path().join(format!("{id}_part_0")).exists());
        Ok(())
    }
}
