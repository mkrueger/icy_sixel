//! # icy_sixel
//!
//! A 100% Rust SIXEL library for encoding and decoding SIXEL graphics.
//!
//! ## Features
//!
//! - **Decoder**: High-performance SIXEL decoder with SIMD optimization (SSE2)
//! - **Encoder**: High-quality SIXEL encoder using quantette for color quantization
//!
//! ## Quick Start
//!
//! ### Encoding an image to SIXEL
//!
//! ```ignore
//! use icy_sixel::SixelImage;
//!
//! // RGBA image data (4 bytes per pixel)
//! let rgba = vec![255u8, 0, 0, 255, 0, 255, 0, 255]; // 2 red and green pixels
//! let image = SixelImage::from_rgba(rgba, 2, 1);
//! let sixel = image.encode()?;
//! print!("{}", sixel);
//! ```
//!
//! ### Decoding SIXEL to image data
//!
//! ```ignore
//! use icy_sixel::SixelImage;
//!
//! let sixel_data = b"\x1bPq#0;2;100;0;0#0~-\x1b\\";
//! let image = SixelImage::decode(sixel_data)?;
//! // image.pixels contains RGBA pixel data (4 bytes per pixel)
//! println!("{}x{}", image.width, image.height);
//! ```

use thiserror::Error;

pub mod decoder;
pub mod encoder;
pub mod sixel_image;

#[allow(deprecated)]
pub use decoder::{sixel_decode, sixel_decode_from_dcs};
pub use decoder::{DcsSettings, SixelDecoder};
#[allow(deprecated)]
pub use encoder::{sixel_encode, sixel_encode_default};
pub use encoder::{EncodeOptions, QuantizeMethod};
pub use sixel_image::{BackgroundMode, PixelAspectRatio, SixelImage};

/// Errors that can occur during SIXEL encoding or decoding.
#[derive(Debug, Error)]
pub enum SixelError {
    /// Invalid image dimensions (width or height is zero or too large)
    #[error("invalid dimensions: {width}x{height}")]
    InvalidDimensions { width: usize, height: usize },

    /// Buffer size doesn't match expected size for dimensions
    #[error("buffer size mismatch: expected {expected} bytes, got {actual}")]
    BufferSizeMismatch { expected: usize, actual: usize },

    /// Invalid SIXEL data format
    #[error("invalid SIXEL data: {0}")]
    InvalidData(String),

    /// No SIXEL data found in input
    #[error("no SIXEL data found (missing DCS introducer)")]
    NoSixelData,

    /// Color quantization failed
    #[error("quantization error: {0}")]
    Quantization(String),

    /// Integer overflow during processing
    #[error("integer overflow")]
    IntegerOverflow,
}

/// Result type for SIXEL operations.
pub type Result<T> = core::result::Result<T, SixelError>;

// Internal SIXEL limits
pub(crate) const SIXEL_REPEAT_MAX: usize = 0xffff;
pub(crate) const SIXEL_PALETTE_MAX: usize = 256;
pub(crate) const SIXEL_WIDTH_LIMIT: usize = 1000000;
pub(crate) const SIXEL_HEIGHT_LIMIT: usize = 1000000;
pub(crate) const SIXEL_CELL_HEIGHT: usize = 6;
/// Maximum decoded canvas area (256 MiB of RGBA pixels, excluding growth overhead).
pub(crate) const SIXEL_MAX_PIXELS: usize = 64 * 1024 * 1024;

/// Reserve enough decoder space for the final SIXEL band, even if it is partly transparent.
pub(crate) fn validate_encode_dimensions(width: usize, height: usize) -> Result<()> {
    if width == 0 || height == 0 {
        return Err(SixelError::InvalidDimensions { width, height });
    }
    // Keep arithmetic failures distinct from valid arithmetic exceeding codec limits.
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(SixelError::IntegerOverflow)?;
    let band_height = height
        .div_ceil(SIXEL_CELL_HEIGHT)
        .checked_mul(SIXEL_CELL_HEIGHT)
        .ok_or(SixelError::IntegerOverflow)?;
    if width > SIXEL_WIDTH_LIMIT || band_height > SIXEL_HEIGHT_LIMIT || width.saturating_mul(band_height) > SIXEL_MAX_PIXELS {
        return Err(SixelError::InvalidDimensions { width, height });
    }
    Ok(())
}

#[cfg(test)]
mod dimension_tests {
    use super::*;

    #[test]
    fn encoder_limits_include_the_complete_last_band() {
        assert!(validate_encode_dimensions(SIXEL_WIDTH_LIMIT, 1).is_ok());
        assert!(validate_encode_dimensions(1, 999_996).is_ok());
        assert!(validate_encode_dimensions(1, 999_997).is_err());
        assert!(validate_encode_dimensions(SIXEL_WIDTH_LIMIT + 1, 1).is_err());
        // Raster area fits, but padding to a complete band must also fit.
        assert!(validate_encode_dimensions(8192, 8184).is_ok());
        assert!(validate_encode_dimensions(8192, 8192).is_err());
        assert!(matches!(validate_encode_dimensions(usize::MAX, 2), Err(SixelError::IntegerOverflow)));
    }
}
