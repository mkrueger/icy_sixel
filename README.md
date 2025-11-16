# icy_sixel
A pure Rust implementation of SIXEL encoder and decoder.

For my projects I needed a rust sixel implementation. Unfortunately there aren't any.
There are several wrapper around the great libsixel library - but I do not want to struggle with another C dependency.

https://github.com/saitoha/libsixel

So I decided to port the essential parts to Rust - and here it is.

## Features

- ✅ **SIXEL Encoder**: Convert images to SIXEL format with various quantization and dithering options
- ✅ **SIXEL Decoder**: Parse SIXEL data back into indexed pixel buffers with palettes
- ✅ Color quantization (median cut algorithm)
- ✅ Multiple dithering methods (Floyd-Steinberg, Atkinson, Burkes, etc.)
- ✅ Quality modes (AUTO, HIGH, LOW, FULL, HIGHCOLOR)
- ✅ Support for different pixel formats (RGB888, PAL8, G8)
- ✅ Pure Rust, no C dependencies

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
println!("{}", sixel);
```

### Decoding

```rust
use icy_sixel::*;

let sixel_data = b"\x1bPq\"1;1;10;10#0;2;0;0;0#0~~@@~~\x1b\\";
let (pixels, width, height, palette, ncolors) = sixel_decode(sixel_data)?;
println!("Image: {}x{}, {} colors", width, height, ncolors);
// pixels: indexed image data (one byte per pixel)
// palette: RGB palette (3 bytes per color)
```

Note: The dither stuff may be obsolete, for now it's in. Thanks to Hayaki Saito and all people who made libsixel possible.

It's likely that all other code paths that I use contain bugs so every feature need to be tested against the original libsixel. The original C code is very good & understandable so it's easy to extend from here. 

Contributions welcome - I just translated the minimum I need for my projects.

Code translated from libsixel revision 6a5be8b72d84037b83a5ea838e17bcf372ab1d5f

## Updates

Reviewed against libsixel commits up to August 2025:
- FPE fixes (issues #166, #167): Already implemented - width/height validation present
- Heap-buffer-overflow fix (commit 316c086): Not applicable - affects debug code not present in port
- Memory leak fix (commit 92bb9b3): Not applicable - affects encoder structures not ported

The core encoding/quantization/dithering functionality remains compatible with libsixel v1.8.7+

## Decoder architecture

- The default decoder now lives in `src/cleanroom_decoder.rs` and is a clean-room implementation derived directly from the public SIXEL specification (see `doc/all-about-sixels.text`).
- ANSI parsing (DCS introducer, raster attributes, color redefinitions, repeat counts, etc.) is implemented from scratch with tight bounds checking to prevent overflows.
- All raster writes operate on contiguous RGB buffers; horizontal spans are filled using a SIMD-accelerated routine on x86/x86_64 targets (falls back to a fast scalar copier elsewhere).
- The legacy decoder derived from libsixel is still available under `icy_sixel::fromsixel` for comparison/testing, but the crate re-exports the clean-room version by default.
