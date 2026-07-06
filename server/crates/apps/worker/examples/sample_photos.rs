use rand::prelude::IndexedRandom;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let source = Path::new("C:/Users/Ruurd/Pictures/Photos");
    let destination = Path::new("C:/Users/Ruurd/Pictures/media_dir");
    let number = 5000;

    // 1. Validate Source
    if !source.exists() || !source.is_dir() {
        eprintln!("Error: Source path does not exist or is not a directory.");
        process::exit(1);
    }

    // 2. Create Destination if it doesn't exist
    if let Err(e) = fs::create_dir_all(destination) {
        eprintln!("Error creating output directory: {e}");
        process::exit(1);
    }

    println!("Scanning source directory...");

    // 3. Collect valid photo paths
    let mut candidates: Vec<PathBuf> = Vec::new();

    match fs::read_dir(source) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    candidates.push(path);
                }
            }
        }
        Err(e) => {
            eprintln!("Error reading source directory: {e}");
            process::exit(1);
        }
    }

    let total_found = candidates.len();
    println!("Found {total_found} photo files.");

    if total_found == 0 {
        println!("No photos found to sample.");
        return;
    }

    // 4. Sample N files
    // If requested number is larger than total, we just take all of them
    let amount_to_copy = std::cmp::min(number, total_found);

    let mut rng = rand::rng();
    // choose_multiple returns references, so we clone the paths we pick
    let selected_files: Vec<PathBuf> = candidates
        .sample(&mut rng, amount_to_copy)
        .cloned()
        .collect();

    println!("Sampling {amount_to_copy} random files...");

    // 5. Copy files
    let mut success_count = 0;
    for src_path in selected_files {
        if let Some(file_name) = src_path.file_name() {
            let dest_path = destination.join(file_name);

            match fs::copy(&src_path, &dest_path) {
                Ok(_) => success_count += 1,
                Err(e) => eprintln!("Failed to copy {}: {e}", src_path.display()),
            }
        }
    }

    println!(
        "Successfully copied {success_count} photos to {}",
        destination.display()
    );
}
