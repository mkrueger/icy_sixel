# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
