# icy_sixel

A high-performance, 100% pure Rust implementation of a SIXEL encoder and decoder.

I wanted a pure Rust implementation to simplify deployment of my cross-platform applications.
In version 0.3.0, I completely rewrote the encoder using [imagequant](https://github.com/ImageOptim/libimagequant)
(the same library that powers [Gifski](https://gif.ski/)). The results are stunning — far better
than the old implementation, and it's faster too!

The decoder is a clean-room implementation based on the SIXEL specification, with SIMD optimizations for maximum performance.

## Features

- **SIXEL Encoder**: High-quality color quantization with imagequant and Floyd-Steinberg dithering
- **SIXEL Decoder**: Clean-room implementation with RGBA output and SSE2 SIMD acceleration
- **Transparency Support**: Full alpha channel handling in both encoder and decoder
- **Pure Rust**: No C dependencies, easy to build and deploy
- **Cross-platform**: Works on Linux, macOS, and Windows

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
icy_sixel = "0.3"
```

## Usage

### Encoding an Image to SIXEL

```rust
use icy_sixel::{sixel_encode, EncodeOptions};

// RGBA image data (4 bytes per pixel)
let rgba = vec![
    255, 0, 0, 255,   // Red pixel
    0, 255, 0, 255,   // Green pixel
    0, 0, 255, 255,   // Blue pixel
];

let options = EncodeOptions::default();
let sixel = sixel_encode(&rgba, 3, 1, &options)?;
print!("{}", sixel);
```

### Encoding with Custom Options

```rust
use icy_sixel::{sixel_encode, EncodeOptions};

let options = EncodeOptions {
    max_colors: 64,     // Use only 64 colors (2-256)
    quality: 80,        // Quality 0-100 (higher = better quality but slower)
};

let sixel = sixel_encode(&rgba, width, height, &options)?;
```

### Decoding SIXEL to Image Data

```rust
use icy_sixel::sixel_decode;

let sixel_data = b"\x1bPq#0;2;100;0;0#0~-\x1b\\";
let (rgba, width, height) = sixel_decode(sixel_data)?;
// rgba contains RGBA pixel data (4 bytes per pixel)
```

## CLI Examples

The crate includes command-line examples for encoding and decoding:

```bash
# Encode a PNG to SIXEL
cargo run --example encode -- image.png -o output.six

# Encode with custom settings
cargo run --example encode -- image.png -o output.six --colors 64 --quality 80

# Decode SIXEL to PNG
cargo run --example decode -- image.six -o output.png
```

## Architecture

### Encoder

The encoder uses [imagequant](https://github.com/ImageOptim/libimagequant) for high-quality
color quantization with dithering. This produces significantly better results than traditional
median-cut algorithms, especially for images with gradients or complex color distributions.

### Decoder

The decoder is a clean-room implementation derived from the SIXEL specification:

- Returns RGBA buffers (4 bytes per pixel) for easy integration with graphics libraries
- SIMD-accelerated horizontal span filling on x86/x86_64 (SSE2)
- Optimized with color caching and loop unrolling
- Comprehensive bounds checking prevents buffer overflows

## Showcase

Quality comparison using a 596×936 pixel photograph (original: 880 KB PNG):

| Settings | SIXEL Size | Decoded Result |
|----------|------------|----------------|
| `--quality 100` (default) | 1,346 KB | ![Default quality](tests/data/beelitz_heilstätten_six_decoded.png) |
| `--quality 1` | 247 KB | ![Low quality](tests/data/beelitz_heilstätten_six_low_decoded.png) |
| `--quality 1 --colors 2` | 100 KB | ![2 colors](tests/data/beelitz_heilstätten_six_2colors_decoded.png) |

Original image for reference:

![Original](tests/data/beelitz_heilstätten.png)

## License

Licensed under the Apache License, Version 2.0 — see [LICENSE](LICENSE) for details.
