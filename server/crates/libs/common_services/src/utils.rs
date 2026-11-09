use crate::database::UpdateField;
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

fn format_tz_offset(offset_seconds: i32) -> String {
    let sign = if offset_seconds >= 0 { '+' } else { '-' };
    let abs_seconds = offset_seconds.abs();
    let hours = abs_seconds / 3600;
    let minutes = (abs_seconds % 3600) / 60;
    format!("{sign}{hours:02}:{minutes:02}")
}

/// Write/update datetime tags on a media file (photo or video) using exiftool.
/// `formatted_datetime` is expected in "YYYY:MM:DD HH:MM:SS" format.
/// `timezone_offset_seconds` writes the offset tags, `SetNull` clears them and `Ignore` leaves
/// them untouched.
pub fn write_media_datetime(
    file_path: &Path,
    formatted_datetime: &str,
    timezone_offset_seconds: &UpdateField<i32>,
    is_video: bool,
) -> color_eyre::Result<()> {
    let et = exiftool::ExifTool::new()?;
    let path = file_path.to_string_lossy();
    // One exiftool call per tag set: every call rewrites the whole file.
    let run = |tags: &[String]| -> color_eyre::Result<()> {
        let mut args: Vec<&str> = vec!["-P", "-overwrite_original"];
        args.extend(tags.iter().map(String::as_str));
        args.push(path.as_ref());
        et.execute_raw(&args)?;
        Ok(())
    };

    if is_video {
        run(&[
            format!("-QuickTime:CreateDate={formatted_datetime}"),
            format!("-QuickTime:ModifyDate={formatted_datetime}"),
        ])?;

        // TODO: investigate whether these errors can be surfaced. They are ignored so a file
        // that cannot store the tag does not fail the whole edit.
        let _ = match timezone_offset_seconds {
            UpdateField::Value(offset) => run(&[format!(
                "-Keys:CreationDate={formatted_datetime}{}",
                format_tz_offset(*offset)
            )]),
            UpdateField::SetNull => run(&["-Keys:CreationDate=".to_string()]),
            UpdateField::Ignore => Ok(()),
        };
        return Ok(());
    }

    let mut tags = vec![format!("-AllDates={formatted_datetime}")];
    match timezone_offset_seconds {
        UpdateField::Value(offset) => {
            let offset = format_tz_offset(*offset);
            for tag in ["OffsetTimeOriginal", "OffsetTime", "OffsetTimeDigitized"] {
                tags.push(format!("-all:{tag}={offset}"));
            }
        }
        UpdateField::SetNull => {
            for tag in ["OffsetTimeOriginal", "OffsetTime", "OffsetTimeDigitized"] {
                tags.push(format!("-all:{tag}="));
            }
        }
        UpdateField::Ignore => {}
    }
    run(&tags)
}

/// Write/update the EXIF Orientation tag on a media file on disk.
pub fn write_exif_orientation(file_path: &Path, orientation: i32) -> color_eyre::Result<()> {
    // `-P` keeps the file mtime: a file without any EXIF date falls back to it for its datetime.
    let et = exiftool::ExifTool::new()?;
    et.write_tag(
        file_path,
        "all:Orientation",
        &orientation.to_string(),
        &["-P", "-n", "-overwrite_original"],
    )?;
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
            &["-P", "-n", "-overwrite_original"],
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
                &["-P", "-n", "-overwrite_original"],
            )?;
        }
    }
    Ok(())
}

/// Remove the GPS coordinates from a media file (photo or video) using exiftool.
pub fn remove_media_gps(file_path: &Path) -> color_eyre::Result<()> {
    let et = exiftool::ExifTool::new()?;
    let mime_type: String = et.read_tag(file_path, "MIMEType", &[])?;
    // Same tags as written above, with `all:` so other groups (e.g. the Android
    // ItemList location atom) are cleared too.
    let tags: &[&str] = if mime_type.starts_with("video/") {
        &["all:GPSCoordinates"]
    } else {
        &[
            "all:GPSLatitude",
            "all:GPSLatitudeRef",
            "all:GPSLongitude",
            "all:GPSLongitudeRef",
        ]
    };
    for tag in tags {
        et.write_tag(file_path, tag, "", &["-P", "-overwrite_original"])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::SystemTime;

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

    #[test]
    fn remove_media_gps_clears_photo_and_video() {
        let dir = tempfile::tempdir().expect("temp dir should be created");

        for name in ["PXL_20250916_155329360.jpg", "VID_20190223_155900.mp4"] {
            let path = copy_fixture(name, dir.path());

            write_media_gps(&path, 52.1, 5.3).expect("gps should be written");
            remove_media_gps(&path).expect("gps should be removed");

            let et = exiftool::ExifTool::new().expect("exiftool should be installed");
            for tag in ["GPSLatitude", "GPSLongitude"] {
                assert!(
                    et.read_tag::<f64>(&path, tag, &["-n"]).is_err(),
                    "{name}: {tag} should be gone"
                );
            }
        }
    }

    #[test]
    fn metadata_writes_preserve_file_modify_date() {
        let dir = tempfile::tempdir().expect("temp dir should be created");
        // Photos without an EXIF date fall back to FileModifyDate, so a metadata write
        // that resets it silently moves the item to today.
        // 2000-12-30 23:00:00 UTC, the era these camera files report.
        let mtime: SystemTime = chrono::DateTime::from_timestamp(978_217_200, 0)
            .expect("timestamp should be valid")
            .into();

        for name in ["PXL_20250916_155329360.jpg", "VID_20190223_155900.mp4"] {
            let path = copy_fixture(name, dir.path());
            std::fs::File::options()
                .write(true)
                .open(&path)
                .expect("fixture should open")
                .set_modified(mtime)
                .expect("mtime should be set");

            write_media_gps(&path, 46.8, 1.3).expect("gps should be written");
            write_exif_orientation(&path, 3).expect("orientation should be written");
            remove_media_gps(&path).expect("gps should be removed");

            let after = std::fs::metadata(&path)
                .expect("metadata should be readable")
                .modified()
                .expect("mtime should be readable");
            assert_eq!(after, mtime, "{name}: metadata writes must keep mtime");
        }
    }
}
