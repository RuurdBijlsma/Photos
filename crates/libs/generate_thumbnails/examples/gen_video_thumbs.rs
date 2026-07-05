use app_state::load_app_settings;
use generate_thumbnails::{ThumbnailConfig, generate_thumbnails};
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let file = Path::new("media_dir/rutenl/vids/jellyfish.mp4");
    let out_folder = Path::new("test_out");
    fs::create_dir_all(out_folder)?;
    let settings = load_app_settings()?;
    generate_thumbnails(
        &settings.ingest,
        file,
        out_folder,
        out_folder,
        ThumbnailConfig {
            orientation: 5,
            extract_motion_photo: false,
            generate_panorama_tiles: false,
        },
    )
    .await?;

    Ok(())
}
