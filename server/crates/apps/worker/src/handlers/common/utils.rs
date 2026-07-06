use crate::context::WorkerContext;
use std::path::{Path, PathBuf};

/// Determines which thumbnail files should be sent to the ML analyzer.
#[must_use]
pub fn get_images_to_analyze(
    context: &WorkerContext,
    file_path: &Path,
    media_item_id: &str,
    percentages: &[u64],
    override_image_size: Option<u64>,
) -> Vec<(i32, PathBuf)> {
    let thumbnail_root = &context.settings.ingest.thumbnails_root;
    let thumb_dir = thumbnail_root.join(media_item_id);

    if context.settings.ingest.is_photo_file(file_path) {
        let analyze_image_size =
            override_image_size.unwrap_or(context.settings.ingest.analyzer.analyze_image_size);
        vec![(0, thumb_dir.join(format!("{analyze_image_size}p.avif")))]
    } else {
        percentages
            .iter()
            .map(|p| {
                (
                    i32::try_from(*p).expect("Percentage should fit in i32"),
                    thumb_dir.join(format!("{p}_percent.avif")),
                )
            })
            .collect()
    }
}
