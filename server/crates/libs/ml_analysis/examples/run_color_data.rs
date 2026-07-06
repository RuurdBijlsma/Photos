use ml_analysis::get_color_data;
use std::path::Path;
use std::time::Instant;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let images = vec![
        Path::new("media_dir/rutenl/tree.jpg"),
        Path::new("media_dir/rutenl/sunset.jpg"),
        Path::new("media_dir/rutenl/pics/PICT0017.JPG"),
    ];

    for image in images {
        let now = Instant::now();
        let img = image::open(image)?;
        let color_data = get_color_data(&img)?;
        println!(
            "{} color: {:?} {:?} {:?} {:?}",
            image.file_name().unwrap().to_string_lossy(),
            color_data.prominent_colors,
            color_data.average_hue,
            color_data.average_saturation,
            color_data.average_lightness,
        );
        println!("\tget_color_data {:?}", now.elapsed());
    }

    Ok(())
}
