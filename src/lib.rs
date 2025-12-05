//! # icy_sixel
//!
//! A 100% Rust SIXEL library for encoding and decoding SIXEL graphics.
//!
//! ## Features
//!
//! - **Decoder**: High-performance SIXEL decoder with SIMD optimization (SSE2)
//! - **Encoder**: High-quality SIXEL encoder using imagequant for color quantization
//!
//! ## Quick Start
//!
//! ### Encoding an image to SIXEL
//!
//! ```ignore
//! use icy_sixel::{sixel_encode, EncodeOptions};
//!
//! // RGBA image data (4 bytes per pixel)
//! let rgba = vec![255u8, 0, 0, 255, 0, 255, 0, 255]; // 2 red and green pixels
//! let sixel = sixel_encode(&rgba, 2, 1, &EncodeOptions::default())?;
//! print!("{}", sixel);
//! ```
//!
//! ### Decoding SIXEL to image data
//!
//! ```ignore
//! use icy_sixel::sixel_decode;
//!
//! let sixel_data = b"\x1bPq#0;2;100;0;0#0~-\x1b\\";
//! let (rgba, width, height) = sixel_decode(sixel_data)?;
//! // rgba contains RGBA pixel data (4 bytes per pixel)
//! ```

use std::error::Error;

pub mod decoder;
pub mod encoder;

pub use decoder::{sixel_decode, sixel_decode_from_dcs};
pub use encoder::{sixel_encode, sixel_encode_default, EncodeOptions};

/// Result type for SIXEL operations
pub type SixelResult<T> = Result<T, Box<dyn Error>>;

// Internal constants used by the decoder
pub(crate) const SIXEL_PALETTE_MAX: usize = 256;
pub(crate) const SIXEL_WIDTH_LIMIT: usize = 1000000;
pub(crate) const SIXEL_HEIGHT_LIMIT: usize = 1000000;

/// SIXEL-specific errors
#[derive(Debug, Clone)]
pub enum SixelError {
    /// Runtime error during processing
    RuntimeError,
    /// Logic error in code
    LogicError,
    /// Feature not enabled
    FeatureError,
    /// Interrupted by a signal
    Interrupted,
    /// Memory allocation failed
    BadAllocation,
    /// Invalid argument provided
    BadArgument,
    /// Invalid input data
    BadInput,
    /// Integer overflow detected
    BadIntegerOverflow,
    /// Feature not implemented
    NotImplemented,
}

impl std::fmt::Display for SixelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SixelError::RuntimeError => write!(f, "runtime error"),
            SixelError::LogicError => write!(f, "logic error"),
            SixelError::FeatureError => write!(f, "feature not enabled"),
            SixelError::Interrupted => write!(f, "interrupted by a signal"),
            SixelError::BadAllocation => write!(f, "memory allocation failed"),
            SixelError::BadArgument => write!(f, "invalid argument"),
            SixelError::BadInput => write!(f, "invalid input data"),
            SixelError::BadIntegerOverflow => write!(f, "integer overflow"),
            SixelError::NotImplemented => write!(f, "feature not implemented"),
        }
    }
}

impl Error for SixelError {
    fn description(&self) -> &str {
        "use std::display"
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}
