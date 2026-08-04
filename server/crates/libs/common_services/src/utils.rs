/// Generate a URL-safe random ID of a given length.
#[must_use]
pub fn nice_id(length: usize) -> String {
    const URL_SAFE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_";
    (0..length)
        .map(|_| {
            let idx = rand::random_range(0..URL_SAFE.len());
            URL_SAFE[idx] as char
        })
        .collect()
}

/// Logs a warning message with an 'ALERT:' prefix.
#[macro_export]
macro_rules! alert {
    ($($arg:tt)*) => {
        warn!("ALERT: {}", format_args!($($arg)*));
    };
}

/// Write/update the EXIF Orientation tag on a media file on disk.
pub fn write_exif_orientation(
    file_path: &std::path::Path,
    orientation: i32,
) -> color_eyre::Result<()> {
    let et = exiftool::ExifTool::new()?;
    // todo: i dont think overwrite_original exists.
    // todo: copy file path here to some temp path, then exiftool it, then copy the modified file back
    et.write_tag(
        file_path,
        "Orientation",
        &orientation.to_string(),
        &["-overwrite_original"],
    )?;
    Ok(())
}
