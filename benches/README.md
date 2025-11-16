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

### 4. **From DCS** (`decode_from_dcs`)

Tests the `sixel_decode_from_dcs` API that accepts pre-parsed DCS parameters.

- Expected time: ~200 ns

### 5. **Real Files** (`real_files/*`)

Tests decoding of actual SIXEL files from the test suite:

- `map8.six`: Small color map image (~3.5 µs)
- `snake.six`: Large snake image (~3 ms)

### 6. **Varying Sizes** (`varying_sizes/*`)

Tests how performance scales with image size (number of bands):

- 10 bands: ~3 µs
- 50 bands: ~18 µs
- 100 bands: ~35 µs
- 200 bands: ~69 µs

### 7. **Color Changes** (`color_changes/*`)

Tests performance with different numbers of color definitions:

- 1 color: ~194 ns
- 4 colors: ~433 ns
- 16 colors: ~1.4 µs
- 64 colors: ~4.7 µs

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
4. **Real-world data**: Complex images like `snake.six` include color changes, raster attributes, and various SIXEL features

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
