use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, Resizer};
use imgref::Img;
use jpegxl_rs::encode::EncoderSpeed;
use jpegxl_rs::encoder_builder;
use ravif::Encoder as AvifEncoder;
use rgb::FromSlice;

fn generate_synthetic_image(width: u32, height: u32) -> Vec<u8> {
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            buffer[idx] = (x % 256) as u8;
            buffer[idx + 1] = (y % 256) as u8;
            buffer[idx + 2] = ((x + y) % 256) as u8;
            buffer[idx + 3] = 255;
        }
    }
    buffer
}

fn bench_encoders(c: &mut Criterion) {
    let mut group = c.benchmark_group("image_encoders");

    // Benchmark across 480p and 1080p target sizes
    let test_cases = [(854u32, 480u32), (1920u32, 1080u32)];

    for (w, h) in test_cases {
        let raw_rgba = generate_synthetic_image(w, h);
        group.throughput(Throughput::Bytes(raw_rgba.len() as u64));

        // JXL (Butteraugli 1.5, Falcon speed)
        group.bench_with_input(
            BenchmarkId::new("JXL_Falcon_q1.5", format!("{w}x{h}")),
            &raw_rgba,
            |b, data| {
                b.iter(|| {
                    let mut encoder = encoder_builder()
                        .lossless(false)
                        .quality(1.5)
                        .speed(EncoderSpeed::Falcon)
                        .build()
                        .unwrap();
                    let res = encoder.encode::<u8, u8>(black_box(data), w, h).unwrap();
                    black_box(res.data);
                });
            },
        );

        // AVIF (ravif, speed 6, quality 70.0)
        group.bench_with_input(
            BenchmarkId::new("AVIF_Speed6_q70", format!("{w}x{h}")),
            &raw_rgba,
            |b, data| {
                b.iter(|| {
                    let rgba = data.as_rgba();
                    let img_ref = Img::new(rgba, w as usize, h as usize);
                    let encoder = AvifEncoder::new()
                        .with_quality(70.0)
                        .with_speed(6)
                        .with_alpha_quality(70.0);
                    let res = encoder.encode_rgba(black_box(img_ref)).unwrap();
                    black_box(res.avif_file);
                });
            },
        );

        // WebP (quality 80.0)
        group.bench_with_input(
            BenchmarkId::new("WebP_q80", format!("{w}x{h}")),
            &raw_rgba,
            |b, data| {
                b.iter(|| {
                    let encoder = webp::Encoder::from_rgba(black_box(data), w, h);
                    let res = encoder.encode(80.0);
                    black_box(res.to_vec());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_encoders);
criterion_main!(benches);