use color_eyre::Result;
use exiftool::ExifTool;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Verifies if the extracted binary data looks like a valid MP4/MOV video.
/// This prevents writing empty files or corrupted small chunks.
fn is_valid_video(bytes: &[u8]) -> bool {
    if bytes.len() < 1000 {
        return false;
    }
    // Search the first 64 bytes for common MP4/MOV markers
    let first_chunk = if bytes.len() > 64 {
        &bytes[..64]
    } else {
        bytes
    };
    first_chunk
        .windows(4)
        .any(|w| w == b"ftyp" || w == b"mdat" || w == b"moov")
}

/// Scans forward from the beginning of the file to locate the MP4 start boundary.
/// It returns the index exactly 4 bytes before the "ftyp" signature.
fn find_embedded_mp4_start(data: &[u8]) -> Option<usize> {
    if data.len() < 8 {
        return None;
    }

    let target = b"ftyp";

    // Scan forward looking for the "ftyp" marker
    for i in 0..(data.len() - 7) {
        if &data[i..i + 4] == target {
            // Read the next 4 bytes representing the major brand
            let brand = &data[i + 4..i + 8];

            // Validate that the brand consists of standard alphanumeric characters (e.g. isom, mp42, avc1)
            if brand.iter().all(|&b| b.is_ascii_alphanumeric()) && i >= 4 {
                // Back up 4 bytes to include the 32-bit big-endian size header of the ftyp box
                return Some(i - 4);
            }
        }
    }
    None
}

pub fn generate_motion_thumbs(input_file: &Path, thumbnails_sub_folder: &Path) -> Result<bool> {
    let out_video = thumbnails_sub_folder.join("motion.mp4");

    // 1. Companion file check (Apple Live Photo style: side-by-side video asset)
    let companion_extensions = ["mov", "MOV", "mp4", "MP4"];
    for ext in &companion_extensions {
        let companion_path = input_file.with_extension(ext);
        if companion_path.exists() && companion_path.is_file() {
            std::fs::copy(&companion_path, &out_video)?;
            return Ok(true);
        }
    }

    // Initialize the ExifTool stay-open wrapper
    let et = ExifTool::new()?;

    // 2. Try extracting via MotionPhotoVideo (Google Pixel HEIC/JPEG standard)
    tracing::debug!("Attempting to extract video using MotionPhotoVideo tag...");
    if let Ok(video_bytes) = et.read_tag_binary(input_file, "MotionPhotoVideo") {
        if is_valid_video(&video_bytes) {
            std::fs::write(&out_video, video_bytes)?;
            return Ok(true);
        }
        tracing::debug!("MotionPhotoVideo tag found but data was invalid or too short.");
    }

    // 3. Try extracting via EmbeddedVideoFile (Samsung standard)
    tracing::debug!("Attempting to extract video using EmbeddedVideoFile tag...");
    if let Ok(video_bytes) = et.read_tag_binary(input_file, "EmbeddedVideoFile") {
        if is_valid_video(&video_bytes) {
            std::fs::write(&out_video, video_bytes)?;
            return Ok(true);
        }
        tracing::debug!("EmbeddedVideoFile tag found but data was invalid or too short.");
    }

    // 4. Try extracting using legacy Google Pixel MicroVideoOffset
    tracing::debug!("Attempting to extract video using legacy MicroVideoOffset fallback...");
    if let Ok(offset_val) = et.read_tag::<u64>(input_file, "MicroVideoOffset", &[])
        && offset_val > 0
    {
        let file_metadata = std::fs::metadata(input_file)?;
        let file_size = file_metadata.len();
        if file_size > offset_val {
            let start_offset = file_size - offset_val;

            let mut f = File::open(input_file)?;
            f.seek(SeekFrom::Start(start_offset))?;

            let mut video_bytes = Vec::with_capacity(offset_val as usize);
            f.take(offset_val).read_to_end(&mut video_bytes)?;

            if is_valid_video(&video_bytes) {
                std::fs::write(&out_video, video_bytes)?;
                return Ok(true);
            }
            tracing::debug!("Extracted MicroVideoOffset bytes were invalid or too short.");
        }
    }

    // 5. Try scanning the raw file for the embedded ftyp boundary
    tracing::debug!("Attempting to extract video by carving (scanning for 'ftyp')...");
    if let Ok(data) = std::fs::read(input_file)
        && let Some(mp4_start_offset) = find_embedded_mp4_start(&data)
    {
        let video_bytes = &data[mp4_start_offset..];
        if is_valid_video(video_bytes) {
            std::fs::write(&out_video, video_bytes)?;
            return Ok(true);
        }
        tracing::debug!("Carved video data was invalid or too short.");
    }

    tracing::debug!(
        "No valid embedded or companion motion photo video found for: {:?}",
        input_file
    );
    Ok(false)
}
