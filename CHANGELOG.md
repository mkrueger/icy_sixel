# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - 2024-12-27

### Added
- `PixelAspectRatio` option for encoder (P1 parameter in DCS introducer)
- `BackgroundMode` option for encoder (P2 parameter for transparency control)
- `sixel_decode_with_settings()` function for decoding with custom DCS settings
- GIF animation support in CLI (`sixel animate` command)

### Changed
- Switched to [quantette](https://github.com/IanManske/quantette) for color quantization (MIT/Apache-2.0 licensed)
- Improved encoder quality with Wu's algorithm and Floyd-Steinberg dithering
- SIMD-accelerated decoder with SSE2 optimizations
- `EncodeOptions` now includes `pixel_aspect_ratio` and `background_mode` fields

### Fixed
- VT340 compatibility improvements with proper raster attributes

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

### Added
- Transparency support in both encoder and decoder
- Configurable diffusion strength for dithering

## [0.3.0] and earlier

- Initial implementation based on libsixel
- Basic SIXEL encoding and decoding functionality
