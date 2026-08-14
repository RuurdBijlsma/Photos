use image::imageops::FilterType;
use std::io;
use std::path::Path;
use tracing::error;

pub fn convert_media_file_sync(
    input_path: &Path,
    output_path: &Path,
    image_out_size: u64,
) -> io::Result<()> {
    // Try native decoding and resizing
    let image_result = (|| -> io::Result<()> {
        let img = image::open(input_path).map_err(|e| io::Error::other(e.to_string()))?;
        let resized = img.resize(u32::MAX, image_out_size as u32, FilterType::Lanczos3);

        resized
            .save(output_path)
            .map_err(|e| io::Error::other(e.to_string()))?;
        Ok(())
    })();

    if image_result.is_ok() {
        return Ok(());
    }

    // Fallback to FFmpeg process
    let scale_filter = format!("scale=-2:{image_out_size}");
    let ffmpeg_output = std::process::Command::new("ffmpeg")
        .arg("-i")
        .arg(input_path)
        .arg("-vf")
        .arg(scale_filter)
        .arg("-y") // Overwrite output files without asking
        .arg(output_path)
        .output()?;

    if ffmpeg_output.status.success() {
        Ok(())
    } else {
        error!("Failed to ffmpeg process file {}", input_path.display());
        Err(io::Error::other(String::from_utf8_lossy(
            &ffmpeg_output.stderr,
        )))
    }
}
