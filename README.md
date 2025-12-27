# icy_sixel

A high-performance, 100% pure Rust implementation of SIXEL graphics encoding and decoding.

[![Crates.io](https://img.shields.io/crates/v/icy_sixel.svg)](https://crates.io/crates/icy_sixel)
[![Documentation](https://docs.rs/icy_sixel/badge.svg)](https://docs.rs/icy_sixel)
[![License](https://img.shields.io/crates/l/icy_sixel.svg)](LICENSE-APACHE)

## What is SIXEL?

SIXEL (Six Pixels) is a bitmap graphics format for terminals, originally developed by DEC for the VT200 series terminals in the 1980s. It allows displaying images directly in terminal emulators that support the format.

Modern terminals with SIXEL support include:
- xterm (with `+sixel` build option)
- mlterm, foot, WezTerm, Contour, ctx, and many more

## Crates

This repository contains two crates:

### [icy_sixel](crates/icy_sixel/README.md) - Library

The core Rust library for encoding and decoding SIXEL graphics.

```bash
cargo add icy_sixel
```

**Features:**
- High-quality color quantization (Wu's algorithm + Floyd-Steinberg dithering)
- SIMD-accelerated decoder
- Full transparency support
- Configurable pixel aspect ratio and background mode
- No C dependencies

### [icy_sixel-cli](crates/icy_sixel-cli/README.md) - Command-Line Tool

A CLI for converting images to/from SIXEL and playing animated GIFs.

```bash
cargo install icy_sixel-cli
```

**Commands:**
- `sixel encode` - Convert PNG/JPEG/GIF/WebP to SIXEL
- `sixel decode` - Convert SIXEL back to PNG
- `sixel animate` - Play animated GIFs in the terminal

## Quick Start

### Library Usage

```rust
use icy_sixel::{sixel_encode, EncodeOptions};

let rgba = vec![255, 0, 0, 255]; // Red pixel
let sixel = sixel_encode(&rgba, 1, 1, &EncodeOptions::default())?;
print!("{}", sixel);
```

### CLI Usage

```bash
# Display image in terminal
sixel encode image.png

# Play animated GIF
sixel animate animation.gif

# Convert SIXEL to PNG
sixel decode image.six -o output.png
```

## Building

```bash
git clone https://github.com/mkrueger/icy_sixel
cd icy_sixel

# Build everything
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Related Projects

- [libsixel](https://github.com/saitoha/libsixel) - The original C implementation
- [quantette](https://github.com/IanManske/quantette) - Color quantization library used by icy_sixel
