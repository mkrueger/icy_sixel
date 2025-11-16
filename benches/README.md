# SIXEL Decoder Benchmarks

This directory contains Criterion benchmarks for the SIXEL decoder implementation.

## Running the Benchmarks

```bash
cargo bench --bench decoder_benchmark
```

## Benchmark Categories

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

## Viewing Results

After running benchmarks, HTML reports are generated in:
```
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

## Adding New Benchmarks

To add a new benchmark:

1. Add a benchmark function in `benches/decoder_benchmark.rs`
2. Add it to the `criterion_group!` macro at the bottom
3. Run `cargo bench` to see results
