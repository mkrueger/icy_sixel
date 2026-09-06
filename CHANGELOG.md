# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- CLI animation validates speed and loop options, honors GIF loop metadata, and rejects unrepresentable frame delays.
- GIF frame extraction stops at the requested frame; full animations decode raw frames incrementally,
	bound the SIXEL cache to 256 MiB, and avoid a second full output-string allocation.
- Decoder rejects incomplete or non-SIXEL DCS headers while retaining raw-payload compatibility.
- An omitted color index now refreshes the cached drawing color from register 0.
- Encoder enforces decoder-compatible dimensions and canvas area, including the complete last six-pixel band.
- Encoder rounds RGB palette channels to the nearest SIXEL percentage instead of always rounding down.
- Encoder splits long runs at 65,535 repetitions so its output stays within the decoder's repeat limit.
- Encoder rejects overflowing RGBA sizes and scratch-buffer sizes with an error instead of panicking,
	and checks dimension conversions before passing them to quantette.
- Transparent pixels (alpha < 128) no longer affect color-palette generation. Mask-aware Floyd–Steinberg
	dithering prevents error diffusion through transparent pixels; fully opaque images retain the existing quantette path.
- Decoder prefers explicit raster aspect ratios over DCS P1 when representable by `PixelAspectRatio`.
	Equivalent fractions are normalized; unsupported ratios retain the P1 fallback without changing the public enum.
- Decoder SIMD fills no longer compute pointers beyond the allocation when checking loop bounds.
- HLS hue normalization no longer overflows for large or saturated color parameters.
- Opaque background pixels consistently use register 0's color at frame start, independent of raster
	preallocation, canvas growth, or palette changes within the frame. Transparent backgrounds remain transparent.
- Fuzz targets use the current encoder options and decoder API. Roundtrip fuzzing now rejects encode/decode
	failures for valid input and checks dimensions, buffer length, and alpha preservation.

### Changed
- CI checks formatting and Clippy for the separate fuzz workspace as well as the main workspace.

## [0.6.0] - 2026-08-19

### Added
- Stateful `SixelDecoder` API for preserving the 256 SIXEL color registers across images.
- Transactional palette updates, independent decoder instances, and `reset_palette()` support for
  DEC Private Mode 1070 integrations.
- Benchmark group covering canvas growth for streams without raster attributes.

### Fixed
- Decoding a stream without raster attributes no longer reallocates the canvas for every column.
	The canvas now grows geometrically, turning quadratic decode time into linear (an
	8000-pixel-wide band decodes ~122x faster).
- `SixelImage::background_mode` now reflects the decoded pixels. Without a P2 parameter the
  decoder fills undrawn pixels opaquely but previously reported `Transparent`.

## [0.5.1] - 2026-08-09

### Fixed
- Encoder now emits the "set raster attributes" control (`"Pan;Pad;Ph;Pv`) after the DCS
	introducer. Multiplexers such as tmux discard the P1 macro parameter when re-emitting a
  SIXEL, which made images appear stretched vertically ([#19](https://github.com/mkrueger/icy_sixel/issues/19)).

## [0.5.0] - 2025-12-27

### Added
- `SixelImage` as the primary public type for both decoding and encoding
	- `SixelImage::decode()` for decoding a full ANSI SIXEL sequence
	- `SixelImage::decode_from_dcs()` for decoding a SIXEL payload with explicit `DcsSettings`
	- `FromStr` for `SixelImage` (parse SIXEL from `&str`)
	- `Display` for `SixelImage` (prints as SIXEL using default encoding)
- Encoding APIs on `SixelImage`
	- `SixelImage::encode()` (default options)
	- `SixelImage::encode_with()` (custom `EncodeOptions`)
- Image metadata preserved/configurable on `SixelImage`
	- `PixelAspectRatio` (P1 parameter)
	- `BackgroundMode` (P2 parameter)
	- Builder-style setters `with_aspect_ratio()` and `with_background_mode()`
- Safer construction with `SixelImage::try_from_rgba()` (validates dimensions, buffer size, overflow)
- CLI improvements
	- Uses `SixelImage` API for encoding
	- GIF animation support (`sixel animate`)
- CLI integration tests to ensure the `sixel` binary runs and supports basic encode/decode flows

### Fixed
- VT340 compatibility improvements with proper raster attributes
- `SixelImage::encode()` now honors the image’s configured `aspect_ratio` and `background_mode`

### Changed
- `EncodeOptions` is now focused on quantization/dithering knobs; pixel aspect ratio and background mode are configured on `SixelImage`
- Documentation and examples updated to use the `SixelImage`-centric API

### Deprecated
- Free functions for encoding/decoding are retained as compatibility wrappers but are deprecated in favor of `SixelImage` methods

## [0.4.3] - 2024-12-20

### Fixed
- Code cleanup and minor improvements

## [0.4.2] - 2024-12-15

### Added
- Command-line interface (`icy_sixel-cli` crate)
- Encoder benchmarks

## [0.4.1] - 2024-12-10

### Changed
- Replaced encoder implementation with new quantette-based encoder
- Improved documentation with examples

## [0.4.0] - 2024-12-05

### Changed
- Major rewrite of the encoder using quantette library
- New clean-room decoder implementation based on SIXEL specification
- Switched to [quantette](https://github.com/IanManske/quantette) for color quantization (MIT/Apache-2.0 licensed)
- Improved encoder quality with Wu's algorithm and Floyd-Steinberg dithering
- SIMD-accelerated decoder with SSE2 optimizations
- `EncodeOptions` now includes `pixel_aspect_ratio` and `background_mode` fields

### Added
- Transparency support in both encoder and decoder
- Configurable diffusion strength for dithering

## [0.3.0] and earlier

- Initial implementation based on libsixel
- Basic SIXEL encoding and decoding functionality
