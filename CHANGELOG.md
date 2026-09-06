# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Incremental SIXEL payload decoding through `SixelDecoder::begin_frame()` and `SixelStreamDecoder::feed()`/`finish()`/`abort()`.
- Streaming sessions preserve commands across arbitrary chunks, report consumed bytes and unconsumed terminators, and commit shared palettes only on successful completion.
- Complete DCS streaming through `SixelDecoder::begin_dcs()`, including split 7/8-bit introducers,
	headers and ST, explicit cancellation/interruption and remainder ownership, and strict EOF validation.

### Changed
- **Breaking:** Encoding rejects images the decoder cannot read back, instead of attempting them. Width may
	not exceed 1,000,000, height padded to a complete six-pixel band may not exceed 1,000,000 (999,996 input
	rows), and the padded area may not exceed 64 Mi pixels. Such sizes return `SixelError::InvalidDimensions`.
- **Breaking:** The decoder rejects incomplete or non-SIXEL DCS headers, which previously decoded as empty
	1x1 images. Raw payloads without a DCS introducer remain supported.
- **Breaking (CLI):** `--speed` must be finite and greater than zero, and `--loops` accepts `-1` or higher;
	other values are reported as usage errors. `--loops 0` now follows the GIF's own loop count as documented.
- Encoded output differs from previous releases because palette channels are rounded to the nearest SIXEL
	percentage rather than always downward.
- The CLI requires `image` 0.25.10 or later and uses `gif` directly to distinguish absent loop metadata from infinite repetition.
- CI checks formatting and Clippy for the separate fuzz workspace as well as the main workspace.

### Fixed
- Ignorable controls no longer split SIXEL numbers or parameter lists, preserving repeats, colors and raster attributes across line breaks.
- C1 controls terminate SIXEL decoding like their seven-bit escape equivalents, without interpreting subsequent data or palette commands.
- CAN/SUB now terminate SIXEL payload decoding, returning the partial image and preserving only preceding palette changes.
- Corrected P1 pixel-aspect mappings in decoding and encoding: 0/1/5/6 mean 2:1,
	2 means 5:1, and 3/4 mean 3:1. Explicit raster attributes still take precedence.
- CLI checks GIF canvas size and sets a decoder allocation budget before decoding frames, including single-frame extraction.
- GIFs without a loop extension play once by default instead of repeating indefinitely.
- `SixelImage` formatting writes an explicit error placeholder on encoding failure instead of causing `to_string()` to panic.
- GIF file export no longer computes unused playback delays, so very small valid speed multipliers do not prevent export.
- CLI animation reports unrepresentable frame delays instead of silently saturating them.
- GIF frame extraction stops at the requested frame, so a corrupt later frame no longer prevents it. Full
	animations decode raw frames incrementally, bound the SIXEL cache to 256 MiB, and avoid a second full
	output-string allocation.
- An omitted color index now refreshes the cached drawing color from register 0.
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
