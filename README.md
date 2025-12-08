# icy_sixel

A high-performance, 100% pure Rust implementation of a SIXEL encoder and decoder.

I wanted a pure Rust implementation to simplify deployment of my cross-platform applications.
In version 0.4.0, I rewrote the encoder using [quantette](https://github.com/IanManske/quantette),
a high-quality color quantization library licensed under MIT/Apache-2.0. It uses Wu's algorithm
with Floyd-Steinberg dithering for excellent results.

The decoder is a clean-room implementation based on the SIXEL specification, with SIMD optimizations for maximum performance.

## Features

- **SIXEL Encoder**: High-quality color quantization with quantette (Wu's algorithm + Floyd-Steinberg dithering)
- **SIXEL Decoder**: Clean-room implementation with RGBA output and SSE2 SIMD acceleration
- **Transparency Support**: Full alpha channel handling in both encoder and decoder
- **Pure Rust**: No C dependencies, easy to build and deploy
- **Cross-platform**: Works on Linux, macOS, and Windows

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
icy_sixel = "0.4"
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
    quality: 100,       // Quality 0-100 (reserved for future use)
};

let sixel = sixel_encode(&rgba, width, height, &options)?;
```

### Decoding SIXEL to Image Data

```rust
use icy_sixel::sixel_decode;

let sixel_data = b"\x1bPq#0;2;100;0;0#0~-\x1b\\";
let image = sixel_decode(sixel_data)?;
// image.rgba contains RGBA pixel data (4 bytes per pixel)
// image.width and image.height contain dimensions
```

## CLI Tool

The crate includes a command-line tool for encoding and decoding:

```bash
# Install the CLI
cargo install icy_sixel

# Encode a PNG to SIXEL
sixel encode image.png -o output.six

# Encode with custom settings
sixel encode image.png -o output.six --colors 64

# Decode SIXEL to PNG
sixel decode image.six -o output.png

# Display image in terminal (requires SIXEL-capable terminal)
sixel show image.png
```

## Architecture

### Encoder

The encoder uses [quantette](https://github.com/IanManske/quantette) for high-quality
color quantization with Wu's algorithm and Floyd-Steinberg dithering. This produces
excellent results, especially for images with gradients or complex color distributions.

### Decoder

The decoder is a clean-room implementation derived from the SIXEL specification:

- Returns RGBA buffers (4 bytes per pixel) for easy integration with graphics libraries
- SIMD-accelerated horizontal span filling on x86/x86_64 (SSE2)
- Optimized with color caching and loop unrolling
- Comprehensive bounds checking prevents buffer overflows

## Showcase

Original image for reference:

![Original](crates/icy_sixel/tests/data/beelitz_heilstätten.png)

Quality comparison using a 596×936 pixel photograph (original: 879 KB PNG):

| Settings | SIXEL Size | Decoded Result |
|----------|------------|----------------|
| 256 colors (default) | 1,066 KB | ![Default quality](crates/icy_sixel/tests/data/beelitz_heilstätten_six_decoded.png) |
| 16 colors | 439 KB | ![Low quality](crates/icy_sixel/tests/data/beelitz_heilstätten_six_low_decoded.png) |
| 2 colors | 104 KB | ![2 colors](crates/icy_sixel/tests/data/beelitz_heilstätten_six_2colors_decoded.png) |

## License

Licensed under the Apache License, Version 2.0 — see [LICENSE](LICENSE) for details.
