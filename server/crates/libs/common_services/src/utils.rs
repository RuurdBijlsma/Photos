use std::path::Path;

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
pub fn write_exif_orientation(file_path: &Path, orientation: i32) -> color_eyre::Result<()> {
    let et = exiftool::ExifTool::new()?;
    et.write_tag(
        file_path,
        "all:Orientation",
        &orientation.to_string(),
        &["-n", "-overwrite_original"],
    )?;
    Ok(())
}

/// Write/update datetime tags on a media file (photo or video) using exiftool.
/// `formatted_datetime` is expected in "YYYY:MM:DD HH:MM:SS" format.
pub fn write_media_datetime(
    file_path: &Path,
    formatted_datetime: &str,
    is_video: bool,
) -> color_eyre::Result<()> {
    let et = exiftool::ExifTool::new()?;
    if is_video {
        et.write_tag(
            file_path,
            "QuickTime:CreateDate",
            &formatted_datetime,
            &["-overwrite_original"],
        )?;
        et.write_tag(
            file_path,
            "QuickTime:ModifyDate",
            &formatted_datetime,
            &["-overwrite_original"],
        )?;
    } else {
        et.write_tag(
            file_path,
            "AllDates",
            &formatted_datetime,
            &["-overwrite_original"],
        )?;
    }
    Ok(())
}
