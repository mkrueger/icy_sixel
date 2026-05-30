//! Clean-room SIXEL encoder using quantette for high-quality color quantization.
//!
//! This encoder uses the quantette library (MIT/Apache licensed) for optimal
//! color palette generation and dithering, then encodes the result to SIXEL format.

use crate::{BackgroundMode, PixelAspectRatio, Result, SixelError};
use quantette::{deps::palette::Srgb, dither::FloydSteinberg, ImageRef, PaletteSize, Pipeline};

// Re-export QuantizeMethod for public API
pub use quantette::QuantizeMethod;

/// Color type for palette entries (RGB).
#[derive(Clone, Copy, Debug)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

/// A compact 1-bit-per-element mask backed by `u64` words.
///
/// Uses 1/8 the memory of a `Vec<bool>`, which improves cache behavior in the
/// per-color band-encoding loop that re-reads the opacity mask many times.
struct BitMask {
    words: Vec<u64>,
}

impl BitMask {
    /// Create a mask with `len` bits, all cleared.
    fn zeros(len: usize) -> Self {
        Self {
            words: vec![0; len.div_ceil(64)],
        }
    }

    /// Set the bit at `index` to 1.
    #[inline]
    fn set(&mut self, index: usize) {
        self.words[index >> 6] |= 1u64 << (index & 63);
    }

    /// Return whether the bit at `index` is set.
    #[inline]
    fn get(&self, index: usize) -> bool {
        (self.words[index >> 6] >> (index & 63)) & 1 != 0
    }
}

/// Options for the quantette-based SIXEL encoder.
#[derive(Clone, Debug)]
pub struct EncodeOptions {
    /// Maximum number of colors in the palette (2-256).
    /// Fewer colors = smaller SIXEL output but less accurate colors.
    pub max_colors: u16,

    /// Floyd-Steinberg error diffusion strength (0.0-1.0).
    ///
    /// Controls how much quantization error is spread to neighboring pixels:
    /// - **0.875**: Default (7/8), best for photographs with smooth gradients
    /// - **0.5**: Reduced dithering, less noise, good for graphics
    /// - **0.0**: No dithering, sharp edges but may show color banding
    ///
    /// Higher values produce smoother gradients but may introduce noise.
    /// Lower values preserve sharp edges but may show color banding.
    /// Values are clamped to the range 0.0-1.0.
    pub diffusion: f32,

    /// Color quantization method.
    ///
    /// Available methods:
    /// - [`QuantizeMethod::Wu`]: Wu's color quantizer (default, fast and high quality)
    /// - [`QuantizeMethod::Kmeans`]: K-means clustering (slower but may be more accurate)
    ///
    /// For most use cases, Wu's method provides excellent results.
    pub quantize_method: QuantizeMethod,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            max_colors: 256,
            diffusion: FloydSteinberg::DEFAULT_ERROR_DIFFUSION,
            quantize_method: QuantizeMethod::Wu,
        }
    }
}

/// Encode RGBA image data into a SIXEL string using quantette.
///
/// # Arguments
/// * `rgba` - Raw RGBA pixel data (4 bytes per pixel: R, G, B, A)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `opts` - Encoding options
///
/// # Returns
/// A SIXEL-encoded string that can be displayed on compatible terminals.
/// Transparent pixels (alpha=0) are preserved using SIXEL's P2=1 mode.
///
/// # Example
/// ```ignore
/// use icy_sixel::{SixelImage, EncodeOptions};
///
/// let rgba = vec![255u8, 0, 0, 255, 0, 255, 0, 255]; // 2 pixels: red, green
/// let image = SixelImage::from_rgba(rgba, 2, 1);
/// let sixel = image.encode_with(&EncodeOptions::default())?;
/// println!("{}", sixel);
/// ```
#[must_use = "this returns the encoded SIXEL string"]
pub fn sixel_encode(rgba: &[u8], width: usize, height: usize, opts: &EncodeOptions) -> Result<String> {
    sixel_encode_impl(rgba, width, height, opts, PixelAspectRatio::default(), BackgroundMode::default())
}

pub(crate) fn sixel_encode_impl(
    rgba: &[u8],
    width: usize,
    height: usize,
    opts: &EncodeOptions,
    pixel_aspect_ratio: PixelAspectRatio,
    background_mode: BackgroundMode,
) -> Result<String> {
    if width == 0 || height == 0 {
        return Err(SixelError::InvalidDimensions { width, height });
    }
    let expected = width * height * 4;
    if rgba.len() != expected {
        return Err(SixelError::BufferSizeMismatch { expected, actual: rgba.len() });
    }

    // Single pass over the RGBA buffer building both the transparency mask
    // (set bit = opaque) and the Srgb<u8> pixels used for quantization
    // (quantette uses palette crate types).
    let pixel_count = width * height;
    let mut opacity_mask = BitMask::zeros(pixel_count);
    let mut rgb_pixels: Vec<Srgb<u8>> = Vec::with_capacity(pixel_count);
    for (i, c) in rgba.chunks_exact(4).enumerate() {
        if c[3] >= 128 {
            opacity_mask.set(i);
        }
        rgb_pixels.push(Srgb::new(c[0], c[1], c[2]));
    }

    // Set up quantette pipeline
    let max_colors = opts.max_colors.clamp(2, 256) as u8;
    let palette_size = PaletteSize::try_from(max_colors).unwrap_or(PaletteSize::MAX);

    // Create image reference for quantette
    let image = ImageRef::new(width as u32, height as u32, &rgb_pixels).map_err(|e| SixelError::Quantization(e.to_string()))?;

    // Use configured quantization method with diffusion-based dithering
    let diffusion = opts.diffusion.clamp(0.0, 1.0);
    let pipeline = Pipeline::new().palette_size(palette_size).quantize_method(opts.quantize_method.clone());

    // Apply dithering based on diffusion setting
    let indexed_image = if diffusion <= 0.0 {
        // No dithering - sharp edges, may show banding
        pipeline.ditherer(None).input_image(image).output_srgb8_indexed_image()
    } else {
        // Use Floyd-Steinberg dithering with specified diffusion strength
        let ditherer = FloydSteinberg::with_error_diffusion(diffusion).unwrap_or_default();
        pipeline.ditherer(ditherer).input_image(image).output_srgb8_indexed_image()
    };

    // Extract palette and indices
    let palette: Vec<Rgb> = indexed_image
        .palette()
        .iter()
        .map(|c| Rgb {
            r: c.red,
            g: c.green,
            b: c.blue,
        })
        .collect();

    let indices: Vec<u8> = indexed_image.indices().to_vec();

    // Encode to SIXEL with transparency support
    encode_indexed_to_sixel(&palette, &indices, &opacity_mask, width, height, pixel_aspect_ratio, background_mode)
}

/// Encode RGBA with default options.
#[inline]
#[deprecated(since = "0.5.0", note = "use SixelImage::from_rgba().encode() instead")]
#[must_use = "this returns the encoded SIXEL string"]
pub fn sixel_encode_default(rgba: &[u8], width: usize, height: usize) -> Result<String> {
    #[allow(deprecated)]
    sixel_encode(rgba, width, height, &EncodeOptions::default())
}

fn encode_indexed_to_sixel(
    palette: &[Rgb],
    indices: &[u8],
    opacity_mask: &BitMask,
    width: usize,
    height: usize,
    aspect_ratio: PixelAspectRatio,
    background_mode: BackgroundMode,
) -> Result<String> {
    let mut out = String::new();

    // DCS introducer for SIXEL: ESC P p1 ; p2 ; p3 q
    // p1=aspect ratio, p2=background mode, p3=0 (grid size default)
    out.push_str("\x1bP");
    write_number(&mut out, aspect_ratio.to_p1_value() as usize);
    out.push(';');
    write_number(&mut out, background_mode.to_p2_value() as usize);
    out.push_str(";0q");

    // Define palette in RGB percent (0-100)
    for (i, c) in palette.iter().enumerate() {
        let r = (c.r as u32 * 100) / 255;
        let g = (c.g as u32 * 100) / 255;
        let b = (c.b as u32 * 100) / 255;
        out.push('#');
        write_number(&mut out, i);
        out.push(';');
        out.push('2');
        out.push(';');
        write_number(&mut out, r as usize);
        out.push(';');
        write_number(&mut out, g as usize);
        out.push(';');
        write_number(&mut out, b as usize);
    }

    let bands = height.div_ceil(6);
    let palette_len = palette.len();

    // Scratch buffer holding the 6-bit sixel value for every (color, column)
    // pair in the current band. Reused across bands; only the rows of colors
    // actually used in a band are cleared, so this stays cheap.
    let mut sixels = vec![0u8; palette_len * width];
    let mut colors_used = vec![false; palette_len];

    for band in 0..bands {
        let y0 = band * 6;
        let y_max = usize::min(y0 + 6, height);

        // Reset only the rows touched by the previous band.
        for (color_index, used) in colors_used.iter_mut().enumerate() {
            if *used {
                sixels[color_index * width..(color_index + 1) * width].fill(0);
                *used = false;
            }
        }

        // Single pass over the band: scatter each opaque pixel's bit into the
        // scratch buffer keyed by its color. This replaces the previous
        // O(colors x pixels) re-scan with a single O(pixels) pass.
        for y in y0..y_max {
            let bit = 1u8 << (y - y0);
            let row = y * width;
            for x in 0..width {
                let pixel_idx = row + x;
                if opacity_mask.get(pixel_idx) {
                    let color_index = indices[pixel_idx] as usize;
                    sixels[color_index * width + x] |= bit;
                    colors_used[color_index] = true;
                }
            }
        }

        // Emit each used color, run-length encoding consecutive identical sixels.
        for color_index in 0..palette_len {
            if !colors_used[color_index] {
                continue; // Skip colors not used in this band
            }

            // Select color map register
            out.push('#');
            write_number(&mut out, color_index);

            let row = &sixels[color_index * width..(color_index + 1) * width];
            let mut x = 0;
            while x < width {
                let bits = row[x];

                // Run-length encode consecutive identical sixel values
                let mut run_len = 1usize;
                while x + run_len < width && row[x + run_len] == bits {
                    run_len += 1;
                }

                // Write RLE or raw sixels
                if run_len > 3 {
                    out.push('!');
                    write_number(&mut out, run_len);
                    out.push((63 + bits) as char);
                } else {
                    let ch = (63 + bits) as char;
                    for _ in 0..run_len {
                        out.push(ch);
                    }
                }
                x += run_len;
            }

            // Carriage return to start of band for next color overlay
            out.push('$');
        }

        // Move to next band
        out.push('-');
    }

    // String terminator: ESC \
    out.push('\x1b');
    out.push('\\');

    Ok(out)
}

/// Fast number to string without allocation
#[inline]
fn write_number(out: &mut String, mut n: usize) {
    if n == 0 {
        out.push('0');
        return;
    }

    let mut buf = [0u8; 20];
    let mut i = buf.len();

    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }

    out.push_str(unsafe { std::str::from_utf8_unchecked(&buf[i..]) });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn test_encode_simple() {
        let rgba = vec![255u8, 0, 0, 255]; // 1x1 red pixel
        let result = sixel_encode(&rgba, 1, 1, &EncodeOptions::default());
        assert!(result.is_ok());
        let sixel = result.unwrap();
        assert!(sixel.starts_with("\x1bP9;1;0q"));
        assert!(sixel.ends_with("\x1b\\"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_encode_2x2() {
        let rgba = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            255, 255, 0, 255, // yellow
        ];
        let result = sixel_encode(&rgba, 2, 2, &EncodeOptions::default());
        assert!(result.is_ok());
    }

    //This test is a compatibility test, the P1 value needs to be 7-9 in order to have square pixels. If it is set to a different value
    //like 0 (default) then most terminals will do things correctly but the Windows Terminal will default to a non-square sixel making the
    //image print out with an incorrect aspect ratio.
    #[test]
    #[allow(deprecated)]
    fn test_encode_is_set_to_square_pixels() {
        let rgba = vec![
            255, 0, 0, 255, // red
            0, 0, 255, 255, // blue
        ];
        let sixel = sixel_encode(&rgba, 2, 1, &EncodeOptions::default()).unwrap();
        assert!(sixel.contains("\x1bP9;"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_invalid_dimensions() {
        let rgba = vec![0u8; 16];

        assert!(sixel_encode(&rgba, 0, 4, &EncodeOptions::default()).is_err());
        assert!(sixel_encode(&rgba, 4, 0, &EncodeOptions::default()).is_err());
        assert!(sixel_encode(&rgba, 10, 10, &EncodeOptions::default()).is_err());
    }
}
