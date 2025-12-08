use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use icy_sixel::sixel_decode;
use std::fs;
use std::hint::black_box;

// Simple SIXEL test data
const SIMPLE_SIXEL: &[u8] = b"\x1bPq#0;2;100;0;0#0~~~\x1b\\";

// More complex pattern with colors
const COMPLEX_SIXEL: &[u8] = b"\x1bPq\
    #0;2;100;0;0#1;2;0;100;0#2;2;0;0;100\
    #0!10~#1!10~#2!10~-\
    #0!10@#1!10@#2!10@-\
    #0!10B#1!10B#2!10B\
    \x1b\\";

// SIXEL with repeat counts and multiple bands
const REPEATED_SIXEL: &[u8] = b"\x1bPq\
    #0;2;50;50;50\
    #0!50?!50@!50B!50F!50N!50^-\
    #0!50?!50@!50B!50F!50N!50^-\
    #0!50?!50@!50B!50F!50N!50^\
    \x1b\\";

fn bench_simple_decode(c: &mut Criterion) {
    c.bench_function("decode_simple_sixel", |b| {
        b.iter(|| {
            let result = sixel_decode(black_box(SIMPLE_SIXEL));
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_complex_decode(c: &mut Criterion) {
    c.bench_function("decode_complex_sixel", |b| {
        b.iter(|| {
            let result = sixel_decode(black_box(COMPLEX_SIXEL));
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_repeated_decode(c: &mut Criterion) {
    c.bench_function("decode_repeated_sixel", |b| {
        b.iter(|| {
            let result = sixel_decode(black_box(REPEATED_SIXEL));
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_real_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_files");

    // Test with map8.six if it exists
    if let Ok(map8_data) = fs::read("tests/data/map8.six") {
        group.bench_with_input(BenchmarkId::new("decode", "map8"), &map8_data, |b, data| {
            b.iter(|| {
                let result = sixel_decode(black_box(data));
                assert!(result.is_ok());
                result
            })
        });
    }

    // Test with snake.six if it exists
    if let Ok(snake_data) = fs::read("tests/data/snake.six") {
        group.bench_with_input(
            BenchmarkId::new("decode", "snake"),
            &snake_data,
            |b, data| {
                b.iter(|| {
                    let result = sixel_decode(black_box(data));
                    assert!(result.is_ok());
                    result
                })
            },
        );
    }

    group.finish();
}

fn bench_varying_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("varying_sizes");

    for size in [10, 50, 100, 200].iter() {
        let mut sixel_data = Vec::new();
        sixel_data.extend_from_slice(b"\x1bPq#0;2;100;0;0");

        // Generate bands of sixels
        for _ in 0..*size {
            sixel_data.extend_from_slice(b"#0!20~-");
        }
        sixel_data.extend_from_slice(b"\x1b\\");

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_bands", size)),
            &sixel_data,
            |b, data| {
                b.iter(|| {
                    let result = sixel_decode(black_box(data));
                    assert!(result.is_ok());
                    result
                })
            },
        );
    }

    group.finish();
}

fn bench_color_changes(c: &mut Criterion) {
    let mut group = c.benchmark_group("color_changes");

    for num_colors in [1, 4, 16, 64].iter() {
        let mut sixel_data = Vec::new();
        sixel_data.extend_from_slice(b"\x1bPq");

        // Define colors
        for i in 0..*num_colors {
            let r = (i * 100 / num_colors) % 100;
            let g = (i * 50) % 100;
            let b = (i * 75) % 100;
            sixel_data.extend_from_slice(format!("#{};2;{};{};{}", i, r, g, b).as_bytes());
        }

        // Use colors
        for i in 0..*num_colors {
            sixel_data.extend_from_slice(format!("#{}~~~", i).as_bytes());
        }

        sixel_data.extend_from_slice(b"\x1b\\");

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_colors", num_colors)),
            &sixel_data,
            |b, data| {
                b.iter(|| {
                    let result = sixel_decode(black_box(data));
                    assert!(result.is_ok());
                    result
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_simple_decode,
    bench_complex_decode,
    bench_repeated_decode,
    bench_real_files,
    bench_varying_sizes,
    bench_color_changes
);

criterion_main!(benches);
