use criterion::{criterion_group, criterion_main, Criterion};
use icy_sixel::{sixel_encode, EncodeOptions};
use std::hint::black_box;

fn load_snake_png() -> (Vec<u8>, usize, usize) {
    let img = image::open("tests/data/snake.png").expect("Failed to load snake.png");
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    (rgba_img.into_raw(), width as usize, height as usize)
}

fn generate_gradient_rgba(width: usize, height: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let r = ((x * 255) / width.max(1)) as u8;
            let g = ((y * 255) / height.max(1)) as u8;
            let b = 128;
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
            pixels.push(255); // Alpha
        }
    }
    pixels
}

fn bench_encode_snake(c: &mut Criterion) {
    let (rgba, width, height) = load_snake_png();

    let opts = EncodeOptions::default();

    c.bench_function("encode_snake_600x450", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), width, height, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_encode_snake_fast(c: &mut Criterion) {
    let (rgba, width, height) = load_snake_png();

    let opts = EncodeOptions {
        max_colors: 256,
        quality: 50, // Lower quality = faster encoding
    };

    c.bench_function("encode_snake_600x450_fast", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), width, height, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_encode_small(c: &mut Criterion) {
    let rgba = generate_gradient_rgba(64, 64);
    let opts = EncodeOptions::default();

    c.bench_function("encode_gradient_64x64", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), 64, 64, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_encode_medium(c: &mut Criterion) {
    let rgba = generate_gradient_rgba(200, 200);
    let opts = EncodeOptions::default();

    c.bench_function("encode_gradient_200x200", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), 200, 200, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

criterion_group!(
    benches,
    bench_encode_snake,
    bench_encode_snake_fast,
    bench_encode_small,
    bench_encode_medium,
);
criterion_main!(benches);
