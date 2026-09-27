use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use imgref::Img;
use jpegxl_rs::encode::EncoderSpeed;
use jpegxl_rs::encoder_builder;
use ravif::Encoder as AvifEncoder;
use rgb::FromSlice;

fn generate_synthetic_rgb(width: u32, height: u32) -> Vec<u8> {
    let mut buffer = vec![0u8; (width * height * 3) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 3) as usize;
            buffer[idx] = (x % 256) as u8;
            buffer[idx + 1] = (y % 256) as u8;
            buffer[idx + 2] = ((x + y) % 256) as u8;
        }
    }
    buffer
}

fn generate_synthetic_rgba(width: u32, height: u32) -> Vec<u8> {
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
    let mut group = c.benchmark_group("thumbnail_codecs");

    let test_resolutions = [(854u32, 480u32), (1920u32, 1080u32)];

    for (w, h) in test_resolutions {
        let raw_rgb = generate_synthetic_rgb(w, h);
        let raw_rgba = generate_synthetic_rgba(w, h);

        group.throughput(Throughput::Bytes(raw_rgb.len() as u64));

        // AVIF (settings.yaml: speed 4, quality 80)
        group.bench_with_input(
            BenchmarkId::new("AVIF_Speed4_q80", format!("{w}x{h}")),
            &raw_rgba,
            |b, data| {
                b.iter(|| {
                    let rgba = data.as_rgba();
                    let img_ref = Img::new(rgba, w as usize, h as usize);
                    let encoder = AvifEncoder::new()
                        .with_quality(80.0)
                        .with_speed(4)
                        .with_alpha_quality(80.0);
                    let res = encoder.encode_rgba(black_box(img_ref)).unwrap();
                    black_box(res.avif_file);
                });
            },
        );

        // JXL (distance 1.1, Falcon speed, RGB)
        group.bench_with_input(
            BenchmarkId::new("JXL_Falcon_d1.1", format!("{w}x{h}")),
            &raw_rgb,
            |b, data| {
                b.iter(|| {
                    let mut encoder = encoder_builder()
                        .lossless(false)
                        .quality(1.1)
                        .speed(EncoderSpeed::Falcon)
                        .build()
                        .unwrap();
                    let res = encoder.encode::<u8, u8>(black_box(data), w, h).unwrap();
                    black_box(res.data);
                });
            },
        );

        // WebP (quality 82, RGB)
        group.bench_with_input(
            BenchmarkId::new("WebP_q82", format!("{w}x{h}")),
            &raw_rgb,
            |b, data| {
                b.iter(|| {
                    let encoder = webp::Encoder::from_rgb(black_box(data), w, h);
                    let res = encoder.encode(82.0);
                    black_box(res.to_vec());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_encoders);
criterion_main!(benches);