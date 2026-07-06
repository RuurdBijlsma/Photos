use crate::runner::context::test_context::TestContext;
use color_eyre::Result;
use color_eyre::eyre::bail;
use common_services::api::auth::interfaces::{LoginUser, Tokens};
use common_types::dev_constants::{EMAIL, PASSWORD};
use std::path::PathBuf;
use walkdir::WalkDir;

pub async fn login(context: &TestContext) -> Result<String> {
    let url = format!("{}/auth/login", &context.settings.api.public_url);
    let response = context
        .http_client
        .post(url)
        .json(&LoginUser {
            email: EMAIL.to_owned(),
            password: PASSWORD.to_owned(),
        })
        .send()
        .await?;
    if response.status().is_success() {
        let tokens: Tokens = response.json().await?;
        Ok(tokens.access_token)
    } else {
        bail!("{} - {}", response.status(), response.text().await?)
    }
}

pub fn media_dir_contents(context: &TestContext) -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut photo_files = Vec::new();
    let mut video_files = Vec::new();

    for entry in WalkDir::new(&context.settings.ingest.media_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.into_path();
        if context.settings.ingest.is_video_file(&path) {
            video_files.push(path);
        } else if context.settings.ingest.is_photo_file(&path) {
            photo_files.push(path);
        }
    }

    Ok((photo_files, video_files))
}
