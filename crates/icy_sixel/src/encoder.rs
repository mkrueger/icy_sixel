//! Clean-room SIXEL encoder using quantette for high-quality color quantization.
//!
//! This encoder uses the quantette library (MIT/Apache licensed) for optimal
//! color palette generation and dithering, then encodes the result to SIXEL format.

use crate::{Result, SixelError};
use quantette::{
    deps::palette::Srgb, dither::FloydSteinberg, ImageRef, PaletteSize, Pipeline, QuantizeMethod,
};

/// Color type for palette entries (RGB).
#[derive(Clone, Copy, Debug)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

/// Options for the quantette-based SIXEL encoder.
#[derive(Clone, Debug)]
pub struct EncodeOptions {
    /// Maximum number of colors in the palette (2-256).
    /// Fewer colors = smaller SIXEL output but less accurate colors.
    pub max_colors: u16,

    /// Quality setting (0-100). Currently unused but reserved for future use.
    /// quantette uses Wu's algorithm which provides consistent high quality.
    pub quality: u8,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            max_colors: 256,
            quality: 100,
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
/// use icy_sixel::{sixel_encode, EncodeOptions};
///
/// let rgba = vec![255u8, 0, 0, 255, 0, 255, 0, 255]; // 2 pixels: red, green
/// let sixel = sixel_encode(&rgba, 2, 1, &EncodeOptions::default())?;
/// println!("{}", sixel);
/// ```
#[must_use = "this returns the encoded SIXEL string"]
pub fn sixel_encode(
    rgba: &[u8],
    width: usize,
    height: usize,
    opts: &EncodeOptions,
) -> Result<String> {
    if width == 0 || height == 0 {
        return Err(SixelError::InvalidDimensions { width, height });
    }
    let expected = width * height * 4;
    if rgba.len() != expected {
        return Err(SixelError::BufferSizeMismatch {
            expected,
            actual: rgba.len(),
        });
    }

    // Check if image has any transparency
    let has_transparency = rgba.chunks_exact(4).any(|c| c[3] < 128);

    // Create transparency mask (true = opaque, false = transparent)
    let opacity_mask: Vec<bool> = rgba.chunks_exact(4).map(|c| c[3] >= 128).collect();

    // Convert RGBA to Srgb<u8> for quantization (quantette uses palette crate types)
    let rgb_pixels: Vec<Srgb<u8>> = rgba
        .chunks_exact(4)
        .map(|c| Srgb::new(c[0], c[1], c[2]))
        .collect();

    // Set up quantette pipeline
    let max_colors = opts.max_colors.clamp(2, 256) as u8;
    let palette_size = PaletteSize::try_from(max_colors).unwrap_or(PaletteSize::MAX);

    // Create image reference for quantette
    let image = ImageRef::new(width as u32, height as u32, &rgb_pixels)
        .map_err(|e| SixelError::Quantization(e.to_string()))?;

    // Use Wu's quantization method for better quality with dithering
    let indexed_image = Pipeline::new()
        .palette_size(palette_size)
        .quantize_method(QuantizeMethod::Wu)
        .ditherer(FloydSteinberg::new())
        .input_image(image)
        .output_srgb8_indexed_image();

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
    encode_indexed_to_sixel(
        &palette,
        &indices,
        &opacity_mask,
        width,
        height,
        has_transparency,
    )
}

/// Encode RGBA with default options.
#[inline]
#[must_use = "this returns the encoded SIXEL string"]
pub fn sixel_encode_default(rgba: &[u8], width: usize, height: usize) -> Result<String> {
    sixel_encode(rgba, width, height, &EncodeOptions::default())
}

fn encode_indexed_to_sixel(
    palette: &[Rgb],
    indices: &[u8],
    opacity_mask: &[bool],
    width: usize,
    height: usize,
    has_transparency: bool,
) -> Result<String> {
    let mut out = String::new();

    // DCS introducer for SIXEL: ESC P p1 ; p2 ; p3 q
    // p1=0 (aspect ratio auto), p2=1 (transparent pixels stay transparent), p3=0 (grid size default)
    out.push('\x1b');
    out.push('P');
    if has_transparency {
        out.push_str("0;1;0"); // P2=1 means transparent pixels remain unchanged
    }
    out.push('q');

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

    for band in 0..bands {
        let y0 = band * 6;
        let y_max = usize::min(y0 + 6, height);

        // Find which colors are used in this band (only for opaque pixels)
        let mut colors_used = [false; 256];
        for y in y0..y_max {
            for x in 0..width {
                let pixel_idx = y * width + x;
                // Only count opaque pixels
                if opacity_mask[pixel_idx] {
                    let idx = indices[pixel_idx] as usize;
                    colors_used[idx] = true;
                }
            }
        }

        // Encode each used color
        for (color_index, &is_used) in colors_used.iter().enumerate().take(palette.len()) {
            if !is_used {
                continue; // Skip colors not used in this band
            }

            // Select color map register
            out.push('#');
            write_number(&mut out, color_index);

            let mut x = 0;
            while x < width {
                // Build sixel value for this column and color
                // Only set bits for opaque pixels with matching color
                let mut bits: u8 = 0;
                for bit in 0..6 {
                    let y = y0 + bit;
                    if y >= y_max {
                        break;
                    }
                    let pixel_idx = y * width + x;
                    // Only draw if pixel is opaque AND has this color
                    if opacity_mask[pixel_idx] && indices[pixel_idx] as usize == color_index {
                        bits |= 1 << bit;
                    }
                }

                // Run-length encode consecutive identical sixel values
                let mut run_len = 1usize;
                while x + run_len < width {
                    let mut bits_next: u8 = 0;
                    for bit in 0..6 {
                        let y = y0 + bit;
                        if y >= y_max {
                            break;
                        }
                        let pixel_idx = y * width + (x + run_len);
                        if opacity_mask[pixel_idx] && indices[pixel_idx] as usize == color_index {
                            bits_next |= 1 << bit;
                        }
                    }
                    if bits_next != bits {
                        break;
                    }
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
    fn test_encode_simple() {
        let rgba = vec![255u8, 0, 0, 255]; // 1x1 red pixel
        let result = sixel_encode(&rgba, 1, 1, &EncodeOptions::default());
        assert!(result.is_ok());
        let sixel = result.unwrap();
        assert!(sixel.starts_with("\x1bPq"));
        assert!(sixel.ends_with("\x1b\\"));
    }

    #[test]
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

    #[test]
    fn test_invalid_dimensions() {
        let rgba = vec![0u8; 16];

        assert!(sixel_encode(&rgba, 0, 4, &EncodeOptions::default()).is_err());
        assert!(sixel_encode(&rgba, 4, 0, &EncodeOptions::default()).is_err());
        assert!(sixel_encode(&rgba, 10, 10, &EncodeOptions::default()).is_err());
    }
}
