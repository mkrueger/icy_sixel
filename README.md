# icy_sixel

A high-performance, pure Rust implementation of SIXEL encoder and decoder with SIMD optimizations.
It started as a port of [libsixel](https://github.com/saitoha/libsixel) by Hayaki Saito (MIT license).

Wanted a pure rust implementation to make it easier to deploy my cross platform applicaitons :).

Added a decoder in 0.2.0 & updated encoder code. May still be broken for most options - maybe I'll just take them out. (I still don't care, just need max quality)

## Features

- **SIXEL Encoder**: Convert images to SIXEL format with color quantization and dithering
- **SIXEL Decoder**: Clean-room implementation with RGBA output and SIMD acceleration
- Color quantization (median cut algorithm)
- Multiple dithering methods (Floyd-Steinberg, Atkinson, Burkes, etc.)
- Quality modes (AUTO, HIGH, LOW, FULL, HIGHCOLOR)
- Pure Rust, no C dependencies

## Usage

### Encoding

```rust
use icy_sixel::*;

let pixels = vec![255, 0, 0, 0, 255, 0, 0, 0, 255]; // RGB data
let sixel = sixel_string(
    &pixels,
    3, 1,  // width, height
    PixelFormat::RGB888,
    DiffusionMethod::FS,
    MethodForLargest::Auto,
    MethodForRep::Auto,
    Quality::HIGH,
)?;
```

### Decoding

```rust
use icy_sixel::*;

let sixel_data = b"\x1bPq\"1;1;10;10#0;2;0;0;0#0~~@@~~\x1b\\";
let (rgba_pixels, width, height) = sixel_decode(sixel_data)?;
// rgba_pixels: RGBA image data (4 bytes per pixel: R, G, B, A=0xFF)
```

## Architecture

The decoder is a clean-room implementation derived from the SIXEL specification (`doc/all-about-sixels.text`):

- Returns RGBA buffers (4 bytes per pixel) for easy integration with graphics libraries
- SIMD-accelerated horizontal span filling on x86/x86_64 (SSE2)
- Optimized with color caching and loop unrolling
- Comprehensive bounds checking prevents buffer overflows

The encoder is based on libsixel v1.8.7+ with encoding and quantization algorithms translated to Rust.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

**Credits**: Encoder based on [libsixel](https://github.com/saitoha/libsixel) by Hayaki Saito (MIT license). Decoder is a clean-room implementation.
