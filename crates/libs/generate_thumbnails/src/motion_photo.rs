use color_eyre::Result;
use std::path::Path;

pub fn generate_motion_thumbs(input_file: &Path, thumbnails_sub_folder: &Path) -> Result<()> {
    // Use something like `exiftool -b -W hi.mp4 -MotionPhotoVideo PXL_20260424_075902896.MP.jpg` with exiftool.bytes to get the video from Google pixel
    // samsung may be: -EmbeddedVideoFile
    // apple idk?
    // carve.rs works but it's dumb just use exiftool
    todo!("Extract motion video from motion jpegs and store as motion.mp4 in thumbnails sub folder (thumbnails/:id/motion.mpg");
    Ok(())
}
