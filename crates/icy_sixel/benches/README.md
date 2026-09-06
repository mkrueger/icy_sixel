# SIXEL Benchmarks

This directory contains Criterion benchmarks for both SIXEL encoder and decoder implementations.

## Running the Benchmarks

```bash
# Run decoder benchmarks
cargo bench --bench decoder_benchmark

# Run encoder benchmarks
cargo bench --bench encoder_benchmark

# Run all benchmarks
cargo bench
```

## Decoder Benchmark Categories

### 1. **Simple Decode** (`decode_simple_sixel`)

Tests decoding of a minimal SIXEL image with basic color definition.

- Expected time: ~200 ns

### 2. **Complex Decode** (`decode_complex_sixel`)

Tests decoding with multiple colors and patterns.

- Expected time: ~1.3 µs

### 3. **Repeated Decode** (`decode_repeated_sixel`)

Tests decoding with repeat counts across multiple bands.

- Expected time: ~12 µs

### 4. **Real Files** (`real_files/*`)

Tests decoding of actual SIXEL files from the test suite:

- `test_page.six`: Color test page (about 13 KiB).
- `beelitz_heilstätten.six`: Large photographic image (about 1 MiB).
- `transparency.six`: Transparent background (about 14 KiB).

Fixtures are embedded with `include_bytes!`: no timed file I/O, no dependency on the
working directory, and missing files fail compilation instead of silently skipping benchmarks.

### 5. **Varying Sizes** (`varying_sizes/*`)

Tests how performance scales with image size (number of bands):

- 10 bands: ~3 µs
- 50 bands: ~18 µs
- 100 bands: ~35 µs
- 200 bands: ~69 µs

### 6. **Color Changes** (`color_changes/*`)

Tests performance with different numbers of color definitions:

- 1 color: ~194 ns
- 4 colors: ~433 ns
- 16 colors: ~1.4 µs
- 64 colors: ~4.7 µs

### 7. **Canvas Growth** (`canvas_growth/*`)

Compares 1,000–8,000-column bands with and without raster dimensions to check canvas growth.

### 8. **Streaming** (`streaming/*`)

Compares the same minimal image and three real fixtures through:

- `batch_dcs`: `SixelImage::decode()` with a complete DCS.
- `batch_payload`: `SixelImage::decode_from_dcs()` with the header already removed.
- `payload/{size}`: `begin_frame()` / `feed()` / `finish()`.
- `dcs/{size}`: `begin_dcs()` / `feed()` / `finish()`.

Chunk sizes are 1, 64, 1,024 and 8,192 bytes, plus `full` (one feed). Small images
shorter than a requested chunk size naturally use one chunk. Payload inputs retain ST;
the matching header settings and payload slice are prepared outside measurement.

Every path includes a new decoder/palette, canvas allocation, parsing, finalization and
image disposal. Pixel buffers, dimensions and metadata are checked against batch decoding
outside the timed loops. Each streaming case uses 30 samples, 1 second of warm-up and
2 seconds of measurement. Throughput counts input bytes for the respective API.

Compare `payload` with `batch_payload`, and `dcs` with `batch_dcs`. Bytewise feeding is
a worst-case call-overhead test, not a recommended transport buffer size. These are
in-memory end-to-end latency measurements, not I/O, peak-memory or historical-version
comparisons. Earlier approximate timings elsewhere in this document are historical.

#### Measurement: 2026-09-06

Linux x86_64, AMD Ryzen 9 9950X3D, Rust 1.96.0, optimized bench profile; decoder at
`bfc4c6f` with the benchmark additions. Criterion point estimates (not guarantees):

| Fixture | Batch DCS | DCS, 1-byte chunks | DCS, 1-KiB chunks | DCS, 8-KiB chunks |
|---------|-----------|-------------------|------------------|------------------|
| Test page | 100.22 µs | 161.34 µs | 99.95 µs | 100.40 µs |
| Beelitz | 7.89 ms | 12.59 ms | 7.46 ms | 7.68 ms |
| Transparency | 49.78 µs | 104.80 µs | 47.21 µs | 46.34 µs |

For these real fixtures, 1-KiB payload streaming measured 98.78 µs / 7.39 ms / 47.02 µs,
versus payload batch baselines of 99.43 µs / 7.61 ms / 46.79 µs. Buffered feeding is
close to current batch latency; bytewise feeding incurs substantial call overhead.
Small differences between separately measured cases can include CPU scheduling/clock effects.

A separate comparison against pre-streaming commit `042c48a`, with identical lockfile,
compiler and 30-sample/1-second warm-up/2-second measurement settings, found a batch
regression. The current package was rebuilt after switching checkouts to exclude shared
target-directory cache collisions. Selected point estimates:

| Existing benchmark | Before streaming | Current | Latency increase |
|--------------------|------------------|---------|------------------|
| Simple | 197.65 ns | 264.76 ns | 34% |
| Complex | 751.56 ns | 946.98 ns | 26% |
| Repeated | 1.886 µs | 2.057 µs | 9% |
| Unsized 1,000-column band | 19.46 µs | 24.21 µs | 24% |
| Raster-sized 1,000-column band | 7.25 µs | 11.50 µs | 59% |
| Raster-sized 4,000-column band | 29.46 µs | 44.53 µs | 51% |

This historical comparison covers synthetic cases, not the real-image fixtures. It
motivated the optimization below; similar streaming and current batch times alone
do not establish absence of a regression.

#### Normal-character hot-path optimization: 2026-09-06

`perf` and Valgrind were unavailable, so this investigation used optimized disassembly
and controlled Criterion experiments, not a sampling CPU profile. The generated code
contained up to six out-of-line `paint_span` calls per SIXEL, plus parameter/control
dispatch on every character. Forcing the pixel-span helper inline removed those calls.
Contiguous SIXEL runs now dispatch pending commands only once, and already-visible
canvas regions avoid calling `ensure_visible`. Bounds checks and repeat consumption
remain in the shared drawing routine; neither batching nor streaming has a second parser.

Fresh pre-change baseline (`hotpath_before`, 30 samples, 1-second warm-up, 2-second
measurement) versus the final full suite (100 samples, 3-second warm-up, 5-second
measurement for these cases), same machine/compiler/lockfile as above:

| Benchmark | Before optimization | After | Latency change |
|-----------|---------------------|-------|----------------|
| Raster-sized 1,000-column band | 11.207 µs | 6.474 µs | -42% |
| Unsized 1,000-column band | 23.659 µs | 19.577 µs | -17% |
| Test page | 99.096 µs | 70.101 µs | -29% |
| Beelitz | 7.673 ms | 6.614 ms | -14% |
| Transparency | 49.740 µs | 50.557 µs | +2% |
| Simple | 235.73 ns | 255.09 ns | +8% |
| Complex | 888.28 ns | 934.06 ns | +5% |
| Repeated | 1.996 µs | 2.008 µs | +1% |

The intermediate `hotpath_runs` run used the same short settings as the baseline and
measured 6.474 µs / 19.878 µs / 70.133 µs / 6.602 ms for the first four cases,
confirming the direction of those gains. The final full suite completed all 70 cases.
Long ordinary-character runs recover the historical performance loss; small and
command-heavy inputs do not consistently improve and retain some streaming overhead.
These measurements do not claim that every decoder workload is now faster.

#### CPU-guided follow-up: 2026-09-06

After installing perf 7.1.3 and Valgrind 3.27.1, user-space cycle sampling exposed
large frame/palette copy costs for the minimal image (about 49% of samples in libc
memmove). For Beelitz, about 86% were in the shared feed/drawing path and 4.5% in
the batch header parser's redundant payload-terminator scan. Percentages describe
those profiles, not a guaranteed fraction of latency for other inputs.

The follow-up keeps finalization in place rather than moving the frame through
`Option::take` and a large return tuple. Only a successfully finalized palette is
copied back; abort/error rollback remains unchanged. Constructor inlining permits
further move elimination. Batch parsing now leaves terminator detection to the
same payload parser used by streaming, avoiding an extra full-image pass. No new
allocations, unsafe code or public API changes were introduced.

Matched short Criterion runs (`perf_before` and `perf_commit`, 30 samples,
1-second warm-up, 2-second measurement; same machine/compiler/lockfile):

| Benchmark | Before this follow-up | After | Latency change |
|-----------|-----------------------|-------|----------------|
| Simple | 269.02 ns | 219.17 ns | -19% |
| Complex | 938.31 ns | 856.41 ns | -9% |
| Repeated | 2.014 µs | 1.896 µs | -6% |
| Test page | 70.711 µs | 67.412 µs | -5% |
| Beelitz | 6.546 ms | 6.458 ms | -1% |
| Transparency | 48.863 µs | 45.906 µs | -6% |

The preceding single-scan experiment measured Beelitz at 6.319 ms; differences
between runs include scheduling/clock effects. These gains are relative to the
already optimized normal-character path, not directly to the pre-streaming release.

The complete 70-case suite subsequently measured simple/complex/repeated at
208.99 ns / 877.10 ns / 1.910 µs and the three real fixtures at
67.105 µs / 6.366 ms / 47.859 µs. An isolated apparent 20% slowdown in the unsized
4,000-column case did not reproduce: the retry measured 78.53 µs, versus the prior
normal-character run's 76.85 µs. Buffered streaming measurements still vary by a
few percent; this follow-up does not claim a speedup for every chunk size.

Fixed-workload Callgrind totals for 10,000 minimal decodes fell from 65.77 million
to 56.99 million instructions (-13%). Instructions attributed to libc memcpy
fell from 17.25 million to 10.40 million (-40%); copies are reduced, not eliminated.
The final Beelitz CPU profile attributes about 89% of cycles to feed/drawing and
6% to command finalization; the redundant scan is gone. Debug/release tests,
40,000 streaming fuzz runs and Memcheck on all three profiling fixtures passed
(zero reported memory errors or lost blocks; 544 bytes remained reachable at exit).

Use the deterministic profiling example to avoid measuring Criterion startup and
untimed fixture checks (which also run for non-selected streaming fixtures):

```bash
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --locked -p icy_sixel --release --example decoder_profile
perf record -e cycles:u -F 997 -- target/release/examples/decoder_profile simple 20000000
perf --no-pager report --stdio
valgrind --tool=callgrind --callgrind-out-file=/tmp/decoder.callgrind \
	target/release/examples/decoder_profile simple 10000
callgrind_annotate --auto=no /tmp/decoder.callgrind
```

Supported fixtures are `simple`, `beelitz` and `transparency`; the second argument
is a fixed decode count. Each iteration creates a fresh decoder and drops its
result. Use native Criterion timings for latency: Valgrind instruction counts
are not CPU-cycle counts and its emulated instruction set may choose a different
libc implementation than native execution.

## Encoder Benchmark Categories

### 1. **Image Sizes** (`encode_small_gradient_64x64`, `encode_medium_gradient_200x200`, `encode_large_gradient_600x450`)

Tests encoding performance across different image dimensions with gradient patterns.

- 64x64: ~977 µs
- 200x200: ~3.8 ms
- 600x450: ~28.3 ms

### 2. **Dithering Methods** (`encode_dithering_methods/*`)

Tests different dithering algorithms on 200x200 gradient images.

- None: ~2.84 ms (fastest)
- Atkinson: ~3.74 ms
- Floyd-Steinberg: ~3.78 ms
- Burkes: ~3.93 ms (slowest)

### 3. **Image Types** (`encode_checkerboard_200x200`, `encode_colorful_200x200`)

Tests encoding of different pattern types at 200x200 resolution.

- Checkerboard: ~1.13 ms (simple pattern)
- Gradient: ~3.8 ms
- Colorful: ~4.71 ms (most complex)

### 4. **Scaling** (`encode_varying_sizes/*`)

Tests how performance scales with image size.

- 50x50: ~816 µs
- 100x100: ~1.49 ms
- 200x200: ~3.83 ms
- 400x400: ~17.2 ms

## Viewing Results

After running benchmarks, HTML reports are generated in:

```bash
target/criterion/
```

Open `target/criterion/report/index.html` in a browser to view interactive charts and statistics.

## Comparing Performance

To compare against a baseline:

```bash
# Run benchmarks and save as baseline
cargo bench --bench decoder_benchmark -- --save-baseline my-baseline

# Make changes to code...

# Compare against baseline
cargo bench --bench decoder_benchmark -- --baseline my-baseline
```

## Performance Notes

The decoder performance is primarily affected by:

1. **Image dimensions**: Larger images take longer (roughly linear scaling)
2. **Number of colors**: More color definitions increase processing time
3. **Repeat counts**: Efficiently handled with minimal overhead
4. **Real-world data**: Photographic images include many color changes and SIXEL drawing commands

The encoder performance is primarily affected by:

1. **Image dimensions**: Roughly quadratic scaling with pixel count
2. **Dithering method**: No dithering is ~25% faster than dithered output
3. **Image complexity**: Simple patterns (checkerboard) encode much faster than complex gradients
4. **Color distribution**: More unique colors require more quantization work

## Adding New Benchmarks

To add a new benchmark:

1. Add a benchmark function in `benches/decoder_benchmark.rs` or `benches/encoder_benchmark.rs`
2. Add it to the `criterion_group!` macro at the bottom of the file
3. Run `cargo bench` to see results
