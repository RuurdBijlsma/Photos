use color_eyre::Result;
use panorama_tiler::{
    DownscalingMethod, OutputConfig, OutputFormat, tile_panorama_with_guessed_angles,
};
use std::fs;
use std::path::Path;

pub fn generate_pano_thumbs(input_file: &Path, pano_sub_folder: &Path) -> Result<()> {
    if !pano_sub_folder.exists() {
        fs::create_dir_all(pano_sub_folder)?;
    }

    let output_config = OutputConfig {
        format: OutputFormat::Webp,
        quality: 85,
        downscaling_method: DownscalingMethod::Direct,
        ..Default::default()
    };

    tile_panorama_with_guessed_angles(input_file, pano_sub_folder, Some(output_config))?;

    Ok(())
}
