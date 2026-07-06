#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use image::GenericImageView;
use image::imageops::FilterType;
use language_model::LlamaClient;
use ml_analysis::get_llm_classification;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::Builder;
use walkdir::WalkDir;

fn resize_image(input: &Path, max_dim: u32) -> image::ImageResult<PathBuf> {
    let img = image::open(input)?;
    let (w, h) = img.dimensions();
    let scale = (max_dim as f32 / w.max(h) as f32).min(1.0);
    let new_w = (w as f32 * scale).round() as u32;
    let new_h = (h as f32 * scale).round() as u32;
    let resized = img.resize(new_w, new_h, FilterType::CatmullRom);
    let tmp = Builder::new()
        .suffix(".jpg")
        .tempfile()
        .map_err(image::ImageError::IoError)?;
    resized.save(tmp.path())?;
    let (_file, path) = tmp
        .keep()
        .map_err(|e| image::ImageError::IoError(e.error))?;
    Ok(path)
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let llm_client = LlamaClient::with_base_url("http://localhost:8080")
        .repetition_penalty(1.2)
        .build();
    let root_folder = "C:/Users/Ruurd/Pictures/Bouke Zweden";
    let out_folder = Path::new("captions/with_search_term");
    if !out_folder.exists() {
        std::fs::create_dir_all(out_folder)?;
    }

    for entry in WalkDir::new(root_folder).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let image = entry.path();
            let image_filename = image.file_name().unwrap().to_string_lossy().to_string();
            let out_path = out_folder.join(format!("{image_filename}-caption.json"));

            if out_path.exists() {
                println!("{} already exists.", out_path.display());
                continue;
            }

            let resized_img_file = resize_image(image, 720)?;

            let now = Instant::now();
            let caption_data = get_llm_classification(&llm_client, &resized_img_file).await?;

            let file = File::create(&out_path)?;
            serde_json::to_writer_pretty(file, &caption_data)?;
            println!("{image_filename} {:?}", now.elapsed());
        }
    }

    Ok(())
}
