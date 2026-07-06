use common_types::ml_analysis::{ColorData, ColorHistogram, RGBChannels};
use image::{DynamicImage, Rgb};
use material_color_utils::utils::color_utils::Argb;
use palette::{FromColor, Hsv, Srgb};

fn average_hue_from_sums(x_sum: f32, y_sum: f32) -> f32 {
    let mut avg = y_sum.atan2(x_sum).to_degrees();
    if avg < 0.0 {
        avg += 360.0;
    }
    avg
}

/// Analyzes a `DynamicImage` to calculate its color properties.
pub fn get_color_data(img: &DynamicImage) -> color_eyre::Result<ColorData> {
    let rgb_image = img.to_rgb8();
    let (width, height) = rgb_image.dimensions();
    let pixel_count = (width * height) as f32;

    let mut hist_r = [0i32; 256];
    let mut hist_g = [0i32; 256];
    let mut hist_b = [0i32; 256];

    let mut x_sum = 0.0;
    let mut y_sum = 0.0;
    let mut sat_sum = 0.0;
    let mut val_sum = 0.0;

    for Rgb([r, g, b]) in rgb_image.pixels() {
        hist_r[*r as usize] += 1;
        hist_g[*g as usize] += 1;
        hist_b[*b as usize] += 1;

        // Convert to Palette HSV for color metric calculations
        let hsv = Hsv::from_color(
            Srgb::new(
                f32::from(*r) / 255.0,
                f32::from(*g) / 255.0,
                f32::from(*b) / 255.0,
            )
            .into_linear(),
        );

        let rad = hsv.hue.into_radians();
        x_sum += rad.cos();
        y_sum += rad.sin();
        sat_sum += hsv.saturation;
        val_sum += hsv.value;
    }

    let average_hue = average_hue_from_sums(x_sum, y_sum);
    let average_saturation = (sat_sum / pixel_count) * 100.0;
    let average_lightness = (val_sum / pixel_count) * 100.0;

    let prominent_colors = material_color_utils::extract_image_colors(img).call();

    let histogram = ColorHistogram {
        bins: 256,
        channels: RGBChannels {
            red: hist_r.to_vec(),
            green: hist_g.to_vec(),
            blue: hist_b.to_vec(),
        },
    };

    Ok(ColorData {
        prominent_colors: prominent_colors.iter().map(Argb::to_hex).collect(),
        average_hue,
        average_saturation,
        average_lightness,
        histogram,
    })
}
