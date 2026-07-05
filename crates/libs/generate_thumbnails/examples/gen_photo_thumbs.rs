#![allow(clippy::cast_precision_loss)]
use app_state::load_app_settings;
use generate_thumbnails::{ThumbnailConfig, generate_thumbnails};
use std::fs;
use std::path::Path;
use std::time::Instant;
use walkdir::WalkDir;

fn rolling_avg(times: &[u128]) -> f64 {
    if times.is_empty() {
        return 0.0;
    }

    let window = 50.min(times.len());
    let sum: u128 = times[times.len() - window..].iter().copied().sum();

    sum as f64 / window as f64
}

fn avg(times: &[u128]) -> f64 {
    if times.is_empty() {
        return 0.0;
    }
    let sum: u128 = times.iter().copied().sum();
    sum as f64 / times.len() as f64
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let root_folder = Path::new("media_dir");
    let mut times = vec![];
    let settings = load_app_settings()?;

    for entry in WalkDir::new(root_folder).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let now = Instant::now();
            let image = entry.path();
            let image_filename = image.file_name().unwrap().to_string_lossy().to_string();

            let out_folder = Path::new("test_out").join(&image_filename);
            fs::create_dir_all(&out_folder)?;
            generate_thumbnails(
                &settings.ingest,
                image,
                &out_folder,
                &out_folder,
                ThumbnailConfig::default(),
            )
            .await?;
            let elapsed = now.elapsed();
            times.push(elapsed.as_millis());
            println!(
                "{image_filename}: {elapsed:?}, rolling_avg: {:.1}s, total_avg: {:.1}s",
                rolling_avg(&times) / 1000.0,
                avg(&times) / 1000.0,
            );
        }
    }

    Ok(())
}
