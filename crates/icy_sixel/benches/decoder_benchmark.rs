use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use icy_sixel::{DcsSettings, SixelDcsFeedStatus, SixelDecoder, SixelFeedStatus, SixelImage};
use std::hint::black_box;
use std::time::Duration;

struct Fixture {
    name: &'static str,
    data: &'static [u8],
    header: &'static [u8],
    settings: DcsSettings,
}

fn real_files() -> [Fixture; 3] {
    [
        Fixture {
            name: "test_page",
            data: include_bytes!("../tests/data/test_page.six"),
            header: b"\x1bPq",
            settings: DcsSettings::default(),
        },
        Fixture {
            name: "beelitz",
            data: include_bytes!("../tests/data/beelitz_heilstätten.six"),
            header: b"\x1bPq",
            settings: DcsSettings::default(),
        },
        Fixture {
            name: "transparency",
            data: include_bytes!("../tests/data/transparency.six"),
            header: b"\x1bP0;1;0q",
            settings: DcsSettings::new(Some(0), Some(1), Some(0)),
        },
    ]
}

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
            let result = SixelImage::decode(black_box(SIMPLE_SIXEL));
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_complex_decode(c: &mut Criterion) {
    c.bench_function("decode_complex_sixel", |b| {
        b.iter(|| {
            let result = SixelImage::decode(black_box(COMPLEX_SIXEL));
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_repeated_decode(c: &mut Criterion) {
    c.bench_function("decode_repeated_sixel", |b| {
        b.iter(|| {
            let result = SixelImage::decode(black_box(REPEATED_SIXEL));
            assert!(result.is_ok());
            result
        })
    });
}

fn bench_real_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_files");

    for Fixture { name, data, .. } in real_files() {
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(BenchmarkId::new("decode", name), &data, |b, data| {
            b.iter(|| SixelImage::decode(black_box(data)).unwrap())
        });
    }

    group.finish();
}

fn decode_payload_chunks(data: &[u8], settings: DcsSettings, chunk_size: usize) -> SixelImage {
    let mut decoder = SixelDecoder::new();
    let mut frame = decoder.begin_frame(settings).unwrap();
    for chunk in data.chunks(chunk_size) {
        if matches!(frame.feed(chunk).unwrap().status, SixelFeedStatus::Terminated(_)) {
            break;
        }
    }
    frame.finish().unwrap()
}

fn decode_dcs_chunks(data: &[u8], chunk_size: usize) -> SixelImage {
    let mut decoder = SixelDecoder::new();
    let mut frame = decoder.begin_dcs();
    for chunk in data.chunks(chunk_size) {
        if frame.feed(chunk).unwrap().status != SixelDcsFeedStatus::NeedMoreData {
            break;
        }
    }
    frame.finish().unwrap()
}

fn assert_same_image(actual: SixelImage, expected: &SixelImage) {
    assert_eq!(actual.dimensions(), expected.dimensions());
    assert_eq!(actual.pixels, expected.pixels);
    assert_eq!(actual.aspect_ratio, expected.aspect_ratio);
    assert_eq!(actual.background_mode, expected.background_mode);
}

fn bench_streaming(c: &mut Criterion) {
    let simple = Fixture {
        name: "simple",
        data: SIMPLE_SIXEL,
        header: b"\x1bPq",
        settings: DcsSettings::default(),
    };
    for Fixture { name, data, header, settings } in std::iter::once(simple).chain(real_files()) {
        let payload = data.strip_prefix(header).expect("fixture DCS header changed");
        let expected = SixelImage::decode(data).unwrap();
        assert_same_image(SixelImage::decode_from_dcs(payload, settings).unwrap(), &expected);
        let mut group = c.benchmark_group(format!("streaming/{name}"));
        group
            .sample_size(30)
            .warm_up_time(Duration::from_secs(1))
            .measurement_time(Duration::from_secs(2));

        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_function("batch_dcs", |b| b.iter(|| SixelImage::decode(black_box(data)).unwrap()));
        group.throughput(Throughput::Bytes(payload.len() as u64));
        group.bench_function("batch_payload", |b| {
            b.iter(|| SixelImage::decode_from_dcs(black_box(payload), settings).unwrap())
        });

        for (label, chunk_size) in [("1", 1), ("64", 64), ("1024", 1024), ("8192", 8192), ("full", usize::MAX)] {
            // Validate outside timing; every path measures initialization, decoding and image disposal.
            assert_same_image(decode_payload_chunks(payload, settings, chunk_size), &expected);
            assert_same_image(decode_dcs_chunks(data, chunk_size), &expected);

            group.throughput(Throughput::Bytes(payload.len() as u64));
            group.bench_function(BenchmarkId::new("payload", label), |b| {
                b.iter(|| decode_payload_chunks(black_box(payload), settings, chunk_size))
            });
            group.throughput(Throughput::Bytes(data.len() as u64));
            group.bench_function(BenchmarkId::new("dcs", label), |b| b.iter(|| decode_dcs_chunks(black_box(data), chunk_size)));
        }
        group.finish();
    }
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

        group.bench_with_input(BenchmarkId::from_parameter(format!("{}_bands", size)), &sixel_data, |b, data| {
            b.iter(|| {
                let result = SixelImage::decode(black_box(data));
                assert!(result.is_ok());
                result
            })
        });
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

        group.bench_with_input(BenchmarkId::from_parameter(format!("{}_colors", num_colors)), &sixel_data, |b, data| {
            b.iter(|| {
                let result = SixelImage::decode(black_box(data));
                assert!(result.is_ok());
                result
            })
        });
    }

    group.finish();
}

/// Streams without raster attributes force the canvas to grow one column at a time,
/// so this contrasts them against the pre-sized path at identical pixel counts.
fn bench_canvas_growth(c: &mut Criterion) {
    let mut group = c.benchmark_group("canvas_growth");

    fn wide_band(width: usize, raster: bool) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"\x1bPq");
        if raster {
            data.extend_from_slice(format!("\"1;1;{};6", width).as_bytes());
        }
        data.extend_from_slice(b"#1;2;100;0;0");
        data.resize(data.len() + width, b'~');
        data.extend_from_slice(b"\x1b\\");
        data
    }

    for width in [1000usize, 2000, 4000, 8000] {
        for (label, raster) in [("unsized", false), ("raster", true)] {
            let data = wide_band(width, raster);
            group.bench_with_input(BenchmarkId::from_parameter(format!("{}_{}", label, width)), &data, |b, data| {
                b.iter(|| {
                    let result = SixelImage::decode(black_box(data));
                    assert!(result.is_ok());
                    result
                })
            });
        }
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
    bench_color_changes,
    bench_canvas_growth,
    bench_streaming
);

criterion_main!(benches);
