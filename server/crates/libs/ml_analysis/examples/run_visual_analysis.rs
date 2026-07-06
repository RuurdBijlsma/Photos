#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
use app_state::load_app_settings;
use image::GenericImageView;
use image::imageops::FilterType;
use ml_analysis::VisualAnalyzer;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn resize_image(input: &Path, max_dim: u32) -> image::ImageResult<PathBuf> {
    let img = image::open(input)?;
    let (w, h) = img.dimensions();

    let scale = (max_dim as f32 / w.max(h) as f32).min(1.0);
    let new_w = (w as f32 * scale).round() as u32;
    let new_h = (h as f32 * scale).round() as u32;

    let resized = img.resize(new_w, new_h, FilterType::CatmullRom);

    let mut out = input.to_path_buf();
    out.set_file_name(format!(
        "{}_small.jpg",
        input.file_stem().unwrap().to_string_lossy()
    ));

    resized.save(&out)?;
    Ok(out)
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let settings = load_app_settings()?;
    let now = Instant::now();
    let analyzer = VisualAnalyzer::new(&settings.ingest.analyzer.search.embedder_model_id).await?;
    println!("VisualAnalyzer::new {:?}\n", now.elapsed());

    let now = Instant::now();
    let response = analyzer
        .llm_client
        .chat("In one sentence, what's your favourite painting?")
        .call()
        .await?;
    println!("chat response {response:?}");
    println!("analyzer.llm_chat {:?}\n", now.elapsed());

    let images = vec![
        Path::new("media_dir/rutenl/cat-pet.jpg"),
        Path::new("media_dir/rutenl/ocr-bug.jpg"),
        Path::new("media_dir/rutenl/orientation-5.jpg"),
        Path::new("media_dir/rutenl/PICT0016.JPG"),
        Path::new("media_dir/rutenl/road.jpg"),
        Path::new("media_dir/rutenl/sunset.jpg"),
        Path::new("media_dir/rutenl/tree.jpg"),
        Path::new("media_dir/rutenl/ocr_scheef.jpg"),
        Path::new("media_dir/rutenl/ocr_ikea.jpg"),
    ];

    for image in images {
        let resized_img_file = resize_image(image, 720)?;
        let image_filename = resized_img_file
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        println!("analyze image {image_filename}");
        let now = Instant::now();
        let analysis = analyzer
            .fast_image_analysis(&settings.ingest.analyzer, &resized_img_file, 0)
            .await?;
        let filename = format!("{image_filename}-analysis.json");
        let file = File::create(Path::new(&filename))?;
        serde_json::to_writer_pretty(file, &analysis)?;
        println!("{:?}\n", now.elapsed());
    }

    Ok(())
}
