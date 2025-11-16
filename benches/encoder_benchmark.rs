use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use icy_sixel::{
    sixel_string, DiffusionMethod, MethodForLargest, MethodForRep, PixelFormat, Quality,
};
use std::hint::black_box;

// Generate test images of different sizes
fn generate_gradient(width: usize, height: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            let r = ((x * 255) / width.max(1)) as u8;
            let g = ((y * 255) / height.max(1)) as u8;
            let b = 128;
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
        }
    }
    pixels
}

fn generate_checkerboard(width: usize, height: usize, cell_size: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            let is_white = ((x / cell_size) + (y / cell_size)) % 2 == 0;
            let color = if is_white { 255 } else { 0 };
            pixels.push(color);
            pixels.push(color);
            pixels.push(color);
        }
    }
    pixels
}

fn generate_colorful(width: usize, height: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            let r = ((x * 255) / width.max(1)) as u8;
            let g = ((y * 255) / height.max(1)) as u8;
            let b = (((x + y) * 255) / (width + height).max(1)) as u8;
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
        }
    }
    pixels
}

fn bench_encode_small_gradient(c: &mut Criterion) {
    let pixels = generate_gradient(64, 64);

    c.bench_function("encode_small_gradient_64x64", |b| {
        b.iter(|| {
            let result = sixel_string(
                black_box(&pixels),
                64,
                64,
                PixelFormat::RGB888,
                DiffusionMethod::FS,
                MethodForLargest::Auto,
                MethodForRep::Auto,
                Quality::HIGH,
            );
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_encode_medium_gradient(c: &mut Criterion) {
    let pixels = generate_gradient(200, 200);

    c.bench_function("encode_medium_gradient_200x200", |b| {
        b.iter(|| {
            let result = sixel_string(
                black_box(&pixels),
                200,
                200,
                PixelFormat::RGB888,
                DiffusionMethod::FS,
                MethodForLargest::Auto,
                MethodForRep::Auto,
                Quality::HIGH,
            );
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_encode_large_gradient(c: &mut Criterion) {
    let pixels = generate_gradient(600, 450);

    c.bench_function("encode_large_gradient_600x450", |b| {
        b.iter(|| {
            let result = sixel_string(
                black_box(&pixels),
                600,
                450,
                PixelFormat::RGB888,
                DiffusionMethod::FS,
                MethodForLargest::Auto,
                MethodForRep::Auto,
                Quality::HIGH,
            );
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_encode_dithering_methods(c: &mut Criterion) {
    let pixels = generate_gradient(200, 200);
    let mut group = c.benchmark_group("encode_dithering_methods");

    for (name, method) in [
        ("None", DiffusionMethod::None),
        ("FS", DiffusionMethod::FS),
        ("Atkinson", DiffusionMethod::Atkinson),
        ("Burkes", DiffusionMethod::Burkes),
    ] {
        group.bench_with_input(BenchmarkId::from_parameter(name), &method, |b, &m| {
            b.iter(|| {
                sixel_string(
                    black_box(&pixels),
                    200,
                    200,
                    PixelFormat::RGB888,
                    m,
                    MethodForLargest::Auto,
                    MethodForRep::Auto,
                    Quality::HIGH,
                )
            })
        });
    }
    group.finish();
}

fn bench_encode_checkerboard(c: &mut Criterion) {
    let pixels = generate_checkerboard(200, 200, 16);

    c.bench_function("encode_checkerboard_200x200", |b| {
        b.iter(|| {
            let result = sixel_string(
                black_box(&pixels),
                200,
                200,
                PixelFormat::RGB888,
                DiffusionMethod::FS,
                MethodForLargest::Auto,
                MethodForRep::Auto,
                Quality::HIGH,
            );
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_encode_colorful(c: &mut Criterion) {
    let pixels = generate_colorful(200, 200);

    c.bench_function("encode_colorful_200x200", |b| {
        b.iter(|| {
            let result = sixel_string(
                black_box(&pixels),
                200,
                200,
                PixelFormat::RGB888,
                DiffusionMethod::FS,
                MethodForLargest::Auto,
                MethodForRep::Auto,
                Quality::HIGH,
            );
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_encode_varying_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_varying_sizes");

    for size in [50, 100, 200, 400] {
        let pixels = generate_gradient(size, size);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", size, size)),
            &size,
            |b, &s| {
                b.iter(|| {
                    sixel_string(
                        black_box(&pixels),
                        s as i32,
                        s as i32,
                        PixelFormat::RGB888,
                        DiffusionMethod::FS,
                        MethodForLargest::Auto,
                        MethodForRep::Auto,
                        Quality::HIGH,
                    )
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_encode_small_gradient,
    bench_encode_medium_gradient,
    bench_encode_large_gradient,
    bench_encode_dithering_methods,
    bench_encode_checkerboard,
    bench_encode_colorful,
    bench_encode_varying_sizes
);
criterion_main!(benches);
