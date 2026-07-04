use std::fs;
use std::path::Path;
use std::time::Instant;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Accept an optional file path from the command line
    let args: Vec<String> = std::env::args().collect();
    let default_path = "media_dir/rutenl/MP.jpg".to_string();
    let input_path_str = args.get(1).unwrap_or(&default_path);
    let input_path = Path::new(input_path_str);

    if !input_path.exists() {
        eprintln!(
            "Error: Input file not found at '{}'.\n\
             Please pass a path as an argument, for example:\n  \
             cargo run --example carve -- /path/to/my_photo.jpg",
            input_path.display()
        );
        std::process::exit(1);
    }

    println!("Reading file: {}", input_path.display());
    let start_time = Instant::now();

    // 1. Read the JPEG into memory
    let data = fs::read(input_path)?;
    println!("File loaded ({} bytes)", data.len());

    // 2. Scan forward to find the true starting boundary of the MP4
    let Some(mp4_start_offset) = find_embedded_mp4_start(&data) else {
        println!("This JPEG does not appear to contain an embedded motion photo video.");
        return Ok(());
    };

    println!("Located embedded MP4 container starting at byte offset: {}", mp4_start_offset);

    // 3. Slice the buffer from the start of the MP4 to the end of the file
    let video_bytes = &data[mp4_start_offset..];
    println!("Extracted video payload size: {} bytes", video_bytes.len());

    if video_bytes.len() < 1000 {
        println!("Warning: Extracted stream size is suspiciously small.");
    }

    // 4. Write to disk
    let output_path = Path::new("extracted_motion.mp4");
    fs::write(output_path, video_bytes)?;

    let duration = start_time.elapsed();
    println!(
        "\nSuccess! Extracted video saved to: {}\nExtraction completed in: {:?}",
        output_path.display(),
        duration
    );

    Ok(())
}

/// Scans forward from the beginning of the file to locate the MP4 start boundary.
/// It returns the index exactly 4 bytes before the "ftyp" signature.
fn find_embedded_mp4_start(data: &[u8]) -> Option<usize> {
    if data.len() < 8 {
        return None;
    }

    let target = b"ftyp";

    // We scan forward looking for "ftyp"
    for i in 0..(data.len() - 7) {
        if &data[i..i + 4] == target {
            // Read the next 4 bytes representing the major brand
            let brand = &data[i + 4..i + 8];

            // Validate that the brand consists of standard alphanumeric characters (e.g. isom, mp42, avc1)
            if brand.iter().all(|&b| b.is_ascii_alphanumeric()) {
                if i >= 4 {
                    // Back up 4 bytes to include the 32-bit big-endian size header of the ftyp box
                    return Some(i - 4);
                }
            }
        }
    }
    None
}