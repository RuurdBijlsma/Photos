use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use axum::Extension;
use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use common_services::api::app_error::AppError;
use common_services::database::media_item_store::MediaItemStore;
use http::{StatusCode, header};
use std::fs::File;
use std::io::{Error as IoError, ErrorKind, Read, Result as IoResult, Write};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument;

struct ChannelWriter {
    tx: mpsc::Sender<Result<Bytes, IoError>>,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let bytes = Bytes::copy_from_slice(buf);
        // Bounded channel will block here if the client stream is slow,
        // providing native backpressure [1].
        match self.tx.blocking_send(Ok(bytes)) {
            Ok(()) => Ok(buf.len()),
            Err(_) => Err(IoError::new(
                ErrorKind::ConnectionAborted,
                "Zipping aborted: downstream receiver dropped.",
            )),
        }
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ZipDownloadParams {
    pub ids: String, // comma-separated list of media item IDs
}

#[instrument(skip(context, user), err(Debug))]
pub async fn download_zip_stream_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Query(params): Query<ZipDownloadParams>,
) -> Result<impl IntoResponse, AppError> {
    let ids: Vec<String> = params
        .ids
        .split(',')
        .map(std::string::ToString::to_string)
        .collect();
    if ids.is_empty() {
        return Err(AppError::BadRequest("No media IDs provided".to_string()));
    }

    // Resolve file names and paths while verifying owner authorization
    let mut files_to_zip = Vec::new();
    for id in &ids {
        if let Some(item) = MediaItemStore::find_by_id(&context.pool, id).await?
            && item.user_id == user.id
        {
            let full_path = context.settings.ingest.media_root.join(&item.relative_path);
            files_to_zip.push((item.filename, full_path));
        }
    }

    if files_to_zip.is_empty() {
        return Err(AppError::NotFound("No authorized files found".to_string()));
    }

    // Set up a bounded channel with natural backpressure [1]
    let (tx, rx) = mpsc::channel::<Result<Bytes, IoError>>(32);
    let writer = ChannelWriter { tx };

    // Offload compression and disk IO to blocking pool threads
    tokio::task::spawn_blocking(move || {
        let mut zip = zip::ZipWriter::new_stream(writer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for (filename, path) in files_to_zip {
            if !path.exists() {
                continue;
            }

            if let Err(e) = zip.start_file(&filename, options) {
                tracing::error!("Failed to append entry {} to zip: {:?}", filename, e);
                break;
            }

            let mut file = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    tracing::error!("Skipped opening file {}: {:?}", path.display(), e);
                    continue;
                }
            };

            let mut buffer = vec![0; 64 * 1024]; // 64KB read chunks
            loop {
                match file.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if let Err(e) = zip.write_all(&buffer[..n]) {
                            tracing::warn!("Zipping aborted (client connection ended): {:?}", e);
                            return;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Read error on file {}: {:?}", path.display(), e);
                        break;
                    }
                }
            }
        }

        if let Err(e) = zip.finish() {
            tracing::error!("Could not finalize zip stream: {:?}", e);
        }
    });

    let stream = ReceiverStream::new(rx);
    let body = axum::body::Body::from_stream(stream);

    let zip_filename = format!("photos_{}.zip", chrono::Utc::now().format("%Y%m%d_%H%M%S"));

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{zip_filename}\""),
        )
        .body(body)
        .map_err(|e| {
            AppError::Internal(color_eyre::eyre::eyre!("Failed to build response: {}", e))
        })?;

    Ok(response)
}
