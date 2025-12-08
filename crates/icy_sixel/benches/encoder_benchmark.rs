use criterion::{criterion_group, criterion_main, Criterion};
use icy_sixel::{sixel_encode, EncodeOptions, QuantizeMethod};
use std::hint::black_box;

fn load_test_page_png() -> (Vec<u8>, usize, usize) {
    let img = image::open("tests/data/test_page.png").expect("Failed to load test_page.png");
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    (rgba_img.into_raw(), width as usize, height as usize)
}

fn load_beelitz_png() -> (Vec<u8>, usize, usize) {
    let img = image::open("tests/data/beelitz_heilstätten.png")
        .expect("Failed to load beelitz_heilstätten.png");
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

fn bench_encode_test_page(c: &mut Criterion) {
    let (rgba, width, height) = load_test_page_png();

    let opts = EncodeOptions::default();

    c.bench_function(&format!("encode_test_page_{}x{}", width, height), |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), width, height, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_encode_beelitz(c: &mut Criterion) {
    let (rgba, width, height) = load_beelitz_png();

    let opts = EncodeOptions::default();

    c.bench_function("encode_beelitz_default", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), width, height, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

// Quantizer comparison benchmarks
fn bench_quantizer_wu(c: &mut Criterion) {
    let (rgba, width, height) = load_beelitz_png();

    let opts = EncodeOptions {
        max_colors: 256,
        diffusion: 0.875,
        quantize_method: QuantizeMethod::Wu,
    };

    c.bench_function("quantizer_wu_256colors", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), width, height, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_quantizer_kmeans(c: &mut Criterion) {
    let (rgba, width, height) = load_beelitz_png();

    let opts = EncodeOptions {
        max_colors: 256,
        diffusion: 0.875,
        quantize_method: QuantizeMethod::kmeans(),
    };

    c.bench_function("quantizer_kmeans_256colors", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), width, height, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

// Color count benchmarks
fn bench_colors_256(c: &mut Criterion) {
    let (rgba, width, height) = load_beelitz_png();

    let opts = EncodeOptions {
        max_colors: 256,
        diffusion: 0.875,
        quantize_method: QuantizeMethod::Wu,
    };

    c.bench_function("colors_256", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), width, height, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_colors_16(c: &mut Criterion) {
    let (rgba, width, height) = load_beelitz_png();

    let opts = EncodeOptions {
        max_colors: 16,
        diffusion: 0.875,
        quantize_method: QuantizeMethod::Wu,
    };

    c.bench_function("colors_16", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), width, height, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_colors_2(c: &mut Criterion) {
    let (rgba, width, height) = load_beelitz_png();

    let opts = EncodeOptions {
        max_colors: 2,
        diffusion: 0.875,
        quantize_method: QuantizeMethod::Wu,
    };

    c.bench_function("colors_2", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), width, height, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

// Diffusion strength benchmarks
fn bench_diffusion_off(c: &mut Criterion) {
    let (rgba, width, height) = load_beelitz_png();

    let opts = EncodeOptions {
        max_colors: 256,
        diffusion: 0.0,
        quantize_method: QuantizeMethod::Wu,
    };

    c.bench_function("diffusion_off", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), width, height, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_diffusion_low(c: &mut Criterion) {
    let (rgba, width, height) = load_beelitz_png();

    let opts = EncodeOptions {
        max_colors: 256,
        diffusion: 0.3,
        quantize_method: QuantizeMethod::Wu,
    };

    c.bench_function("diffusion_low", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), width, height, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_diffusion_medium(c: &mut Criterion) {
    let (rgba, width, height) = load_beelitz_png();

    let opts = EncodeOptions {
        max_colors: 256,
        diffusion: 0.5,
        quantize_method: QuantizeMethod::Wu,
    };

    c.bench_function("diffusion_medium", |b| {
        b.iter(|| {
            let result = sixel_encode(black_box(&rgba), width, height, &opts);
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_diffusion_full(c: &mut Criterion) {
    let (rgba, width, height) = load_beelitz_png();

    let opts = EncodeOptions {
        max_colors: 256,
        diffusion: 0.875,
        quantize_method: QuantizeMethod::Wu,
    };

    c.bench_function("diffusion_full", |b| {
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
    bench_encode_test_page,
    bench_encode_beelitz,
    // Quantizer comparison
    bench_quantizer_wu,
    bench_quantizer_kmeans,
    // Color count comparison
    bench_colors_256,
    bench_colors_16,
    bench_colors_2,
    // Diffusion strength comparison
    bench_diffusion_off,
    bench_diffusion_low,
    bench_diffusion_medium,
    bench_diffusion_full,
    // Synthetic benchmarks
    bench_encode_small,
    bench_encode_medium,
);
criterion_main!(benches);
