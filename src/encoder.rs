//! Clean-room SIXEL encoder using imagequant for high-quality color quantization.
//!
//! This encoder uses the imagequant library for optimal color palette generation
//! and dithering, then encodes the result to SIXEL format.

use crate::SixelResult;
use imagequant::{Attributes, RGBA};

/// Options for the imagequant-based SIXEL encoder.
#[derive(Clone, Debug)]
pub struct EncodeOptions {
    /// Maximum number of colors in the palette (2-256).
    /// Fewer colors = smaller SIXEL output but less accurate colors.
    pub max_colors: u16,

    /// Quality setting (0-100). Affects both visual quality and output size.
    ///
    /// Higher quality allows the encoder to use more colors from the palette
    /// and spend more effort on optimal dithering, which typically results
    /// in larger SIXEL output but better visual fidelity.
    ///
    /// - **100**: Best quality, largest output (recommended for final output)
    /// - **80**: Good quality/size balance (good default for most uses)
    /// - **50**: Medium quality, smaller output
    /// - **20**: Lower quality, smallest output (for previews or thumbnails)
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

/// Encode RGBA image data into a SIXEL string using imagequant.
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
pub fn sixel_encode(
    rgba: &[u8],
    width: usize,
    height: usize,
    opts: &EncodeOptions,
) -> SixelResult<String> {
    if width == 0 || height == 0 {
        return Err("width and height must be > 0".into());
    }
    if rgba.len() != width * height * 4 {
        return Err("rgba buffer size must be width*height*4".into());
    }

    // Check if image has any transparency
    let has_transparency = rgba.chunks_exact(4).any(|c| c[3] < 128);

    // Create transparency mask (true = opaque, false = transparent)
    let opacity_mask: Vec<bool> = rgba.chunks_exact(4).map(|c| c[3] >= 128).collect();

    // Convert to imagequant RGBA format
    // For transparent pixels, we still need to provide a color, but we'll skip them during encoding
    let pixels: Vec<RGBA> = rgba
        .chunks_exact(4)
        .map(|c| RGBA::new(c[0], c[1], c[2], c[3]))
        .collect();

    // Set up imagequant
    // Speed is derived from quality: high quality = low speed (more effort)
    let speed = match opts.quality {
        90..=100 => 1, // Best quality: slowest
        70..=89 => 3,  // High quality
        50..=69 => 5,  // Medium quality
        30..=49 => 7,  // Lower quality
        _ => 10,       // Fast mode for previews
    };

    let mut attr = Attributes::new();
    attr.set_max_colors(opts.max_colors.min(256) as u32)?;
    attr.set_quality(0, opts.quality)?;
    attr.set_speed(speed)?;

    // Create image and quantize
    let mut img = attr.new_image(pixels, width, height, 0.0)?;
    let mut result = attr.quantize(&mut img)?;

    // Enable dithering for better quality
    result.set_dithering_level(1.0)?;

    // Remap pixels to palette indices
    let (palette, indices) = result.remapped(&mut img)?;

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
pub fn sixel_encode_default(rgba: &[u8], width: usize, height: usize) -> SixelResult<String> {
    sixel_encode(rgba, width, height, &EncodeOptions::default())
}

fn encode_indexed_to_sixel(
    palette: &[RGBA],
    indices: &[u8],
    opacity_mask: &[bool],
    width: usize,
    height: usize,
    has_transparency: bool,
) -> SixelResult<String> {
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

    let bands = (height + 5) / 6;

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
        for color_index in 0..palette.len() {
            if !colors_used[color_index] {
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
