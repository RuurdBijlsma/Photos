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

/// Write/update the GPS coordinates on a media file (photo or video) using exiftool.
pub fn write_media_gps(file_path: &Path, latitude: f64, longitude: f64) -> color_eyre::Result<()> {
    let et = exiftool::ExifTool::new()?;
    // Ask exiftool rather than the extension: videos only have the QuickTime location tag,
    // which stores an ISO 6709 string instead of separate EXIF latitude/longitude tags.
    let mime_type: String = et.read_tag(file_path, "MIMEType", &[])?;
    if mime_type.starts_with("video/") {
        et.write_tag(
            file_path,
            "Keys:GPSCoordinates",
            &format!("{latitude:+.6}{longitude:+.6}/"),
            &["-n", "-overwrite_original"],
        )?;
    } else {
        let latitude_ref = if latitude < 0.0 { "S" } else { "N" };
        let longitude_ref = if longitude < 0.0 { "W" } else { "E" };
        let tags = [
            ("GPSLatitude", latitude.to_string()),
            ("GPSLatitudeRef", latitude_ref.to_owned()),
            ("GPSLongitude", longitude.to_string()),
            ("GPSLongitudeRef", longitude_ref.to_owned()),
        ];
        for (tag, value) in tags {
            et.write_tag(
                file_path,
                &format!("all:{tag}"),
                &value,
                &["-n", "-overwrite_original"],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn copy_fixture(name: &str, dir: &Path) -> PathBuf {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_integration/assets/media_dir")
            .join(name);
        let target = dir.join(name);
        std::fs::copy(&source, &target).expect("fixture should be copied");
        target
    }

    fn read_coordinates(path: &Path) -> (f64, f64) {
        let et = exiftool::ExifTool::new().expect("exiftool should be installed");
        let latitude = et
            .read_tag::<f64>(path, "GPSLatitude", &["-n"])
            .expect("latitude should be readable");
        let longitude = et
            .read_tag::<f64>(path, "GPSLongitude", &["-n"])
            .expect("longitude should be readable");
        (latitude, longitude)
    }

    #[test]
    fn write_media_gps_updates_photo_and_video() {
        let dir = tempfile::tempdir().expect("temp dir should be created");

        for name in ["PXL_20250916_155329360.jpg", "VID_20190223_155900.mp4"] {
            let path = copy_fixture(name, dir.path());

            write_media_gps(&path, -33.8688, 4.895_168).expect("gps should be written");

            let (latitude, longitude) = read_coordinates(&path);
            assert!(
                (latitude + 33.8688).abs() < 1e-4,
                "{name}: unexpected latitude {latitude}"
            );
            assert!(
                (longitude - 4.895_168).abs() < 1e-4,
                "{name}: unexpected longitude {longitude}"
            );
        }
    }
}
