use icy_sixel::*;

#[test]
fn test_decode_simple_sixel() {
    // Simple 2x2 black square
    let sixel_data = b"\x1bPq\"1;1;2;2#0;2;0;0;0#0~~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok(), "Decoding should succeed");

    let image = result.unwrap();
    let (pixels, width, height) = (image.pixels, image.width, image.height);
    assert!(width > 0, "Width should be positive");
    assert!(height > 0, "Height should be positive");
    assert_eq!(
        pixels.len(),
        width * height * 4,
        "Pixel buffer size should match dimensions * 4 (RGBA)"
    );
}

#[test]
fn test_decode_with_aspect_ratio() {
    // Test that aspect ratio is parsed from DCS params
    let sixel_data = b"\x1bP2q#0;2;100;0;0#0~~\x1b\\"; // P1=2 means aspect 5:1

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok(), "Decoding should succeed");

    let image = result.unwrap();
    assert!(image.width > 0);
    assert!(image.height >= 6);
    // Aspect ratio should be parsed (P1=2 means pad=5)
}

#[test]
fn test_decode_with_colors() {
    // SIXEL with color definition
    let sixel_data = b"\x1bPq#0;2;100;0;0#0~~@@~~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (pixels, width, height) = (image.pixels, image.width, image.height);
    assert!(width > 0);
    assert!(height >= 6); // At least one sixel row (6 pixels high)

    // Check that we have pixel data
    assert!(pixels.len() >= 3);
}

#[test]
fn test_decode_multicolor() {
    // Multiple colors
    let sixel_data = b"\x1bPq#0;2;100;0;0#1;2;0;100;0#0~#1~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (_pixels, width, height) = (image.pixels, image.width, image.height);
    assert!(width > 0);
    assert!(height > 0);
}

#[test]
fn test_decode_with_repeat() {
    // Test repeat count !
    let sixel_data = b"\x1bPq#0!5~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (_pixels, width, _height) = (image.pixels, image.width, image.height);
    assert_eq!(width, 5, "Width should be 5 (repeat count)");
}

#[test]
fn test_decode_carriage_return() {
    // Test $ (carriage return)
    let sixel_data = b"\x1bPq#0~~$~~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (_pixels, width, _height) = (image.pixels, image.width, image.height);
    assert_eq!(width, 2, "Width should be 2");
}

#[test]
fn test_decode_color_overlay_preserves_previous_pixels() {
    // First draw red across all six rows, then return and draw green only on the bottom row.
    // The previously drawn red pixels must survive in rows where the second pass has zero bits.
    let sixel_data = b"\x1bPq#2~$#3_\x1b\\";

    let image = sixel_decode(sixel_data).expect("Decoding overlay should work");
    let (pixels, width, height) = (image.pixels, image.width, image.height);
    assert_eq!(width, 1, "Overlay sample should be one column wide");
    assert!(
        height >= 6,
        "Single sixel column must span six pixels vertically"
    );

    let stride = width * 4;
    let top = &pixels[0..4];
    let bottom = &pixels[(height - 1) * stride..(height - 1) * stride + 4];

    assert_eq!(
        top,
        &[204, 33, 33, 255],
        "Top rows must keep the red color from the first pass"
    );
    assert_eq!(
        bottom,
        &[51, 204, 51, 255],
        "Bottom row must reflect the green overlay"
    );
}

#[test]
fn test_decode_newline() {
    // Test - (new line)
    let sixel_data = b"\x1bPq#0~~-~~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let height = image.height;
    assert!(
        height >= 12,
        "Height should be at least 12 (two sixel rows)"
    );
}

#[test]
fn test_decode_hls_color() {
    // HLS color space: #Pc;1;Ph;Pl;Ps
    let sixel_data = b"\x1bPq#0;1;120;50;100#0~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let pixels = image.pixels;
    // Color 0 should be defined with HLS values
    assert!(pixels.len() >= 3);
}

#[test]
fn test_decode_rgb_color() {
    // RGB color space: #Pc;2;Pr;Pg;Pb
    let sixel_data = b"\x1bPq#0;2;100;50;0#0~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let pixels = image.pixels;
    // Color 0 should be defined
    assert!(pixels.len() >= 4);
    // RGB 100,50,0 should map to approximately 255,127,0
    let r = pixels[0];
    let g = pixels[1];
    let b = pixels[2];
    let a = pixels[3];
    assert!(r > 200, "Red should be high");
    assert!(g > 100 && g < 150, "Green should be medium");
    assert!(b < 50, "Blue should be low");
    assert_eq!(a, 255, "Alpha should be 255");
}

#[test]
fn test_decode_raster_attributes() {
    // Test raster attributes "Pan;Pad;Ph;Pv
    let sixel_data = b"\x1bPq\"1;1;10;20#0~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (_pixels, width, height) = (image.pixels, image.width, image.height);
    // Dimensions should be at least the specified Ph;Pv
    assert!(width >= 10, "Width should be at least 10");
    assert!(height >= 20, "Height should be at least 20");
}

#[test]
fn test_decode_empty() {
    // Empty SIXEL
    let sixel_data = b"\x1bPq\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (_pixels, width, height) = (image.pixels, image.width, image.height);
    assert!(width > 0);
    assert!(height > 0);
}

#[test]
fn test_decode_all_sixel_chars() {
    // Test all sixel character values ? to ~
    let sixel_data =
        b"\x1bPq#0?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (_pixels, width, _height) = (image.pixels, image.width, image.height);
    // Should decode the entire range of sixel characters
    assert!(width > 60, "Should have decoded many characters");
}

#[test]
fn test_decode_roundtrip_simple() {
    // Create a simple RGBA image, encode it, decode it, check dimensions
    let original_pixels = vec![
        255, 0, 0, 255, // red
        0, 255, 0, 255, // green
        0, 0, 255, 255, // blue
        255, 255, 0, 255, // yellow
    ];

    let encoded = sixel_encode(&original_pixels, 2, 2, &EncodeOptions::default());

    assert!(encoded.is_ok());
    let sixel_str = encoded.unwrap();

    let decoded = sixel_decode(sixel_str.as_bytes());
    assert!(decoded.is_ok());

    let image = decoded.unwrap();
    let (pixels, width, height) = (image.pixels, image.width, image.height);

    // Check dimensions - note that SIXEL works in bands of 6 pixels high,
    // so height will be rounded up to the nearest multiple of 6
    // Also the encoder might add some padding
    assert_eq!(width, 2, "Width should match");
    assert!(height >= 2, "Height should be at least 2");

    // Check pixel buffer size (RGBA: 4 bytes per pixel)
    assert_eq!(pixels.len(), width * height * 4);
}

#[test]
fn test_decode_vertical_patterns() {
    // Test different vertical bit patterns
    // ? = 0b000000, @ = 0b000001, A = 0b000010, etc.
    let sixel_data = b"\x1bPq#0?@A~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (_pixels, width, height) = (image.pixels, image.width, image.height);
    assert_eq!(width, 4);
    assert!(height >= 6);

    // Column 0 should be empty (?)
    // Column 1 should have pixel at y=0 (@)
    // Column 2 should have pixel at y=1 (A)
    // etc.
}

#[test]
fn test_decode_large_repeat() {
    // Test large repeat count
    let sixel_data = b"\x1bPq#0!100~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (_pixels, width, _height) = (image.pixels, image.width, image.height);
    assert_eq!(width, 100);
}

#[test]
fn test_decode_palette_bounds() {
    // Test palette color index at boundary
    let sixel_data = b"\x1bPq#255;2;50;50;50#255~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let _ = result.unwrap();
    // Should handle color 255 correctly
}

#[test]
fn test_decode_escape_sequences() {
    // Test various escape sequence forms
    // ESC P ... ESC \ (7-bit)
    let sixel_data = b"\x1bP0;0;0q#0~\x1b\\";
    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    // DCS ... ST (8-bit)
    let sixel_data = b"\x90q#0~\x9c";
    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());
}

#[test]
fn test_decode_rgb() {
    // Simple SIXEL with a red pixel
    let sixel_data = b"\x1bPq#2;2;100;0;0~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (pixels, width, height) = (image.pixels, image.width, image.height);
    assert_eq!(width, 1);
    assert_eq!(height, 6); // SIXEL always encodes 6 pixels high

    // Check first pixel is red
    assert_eq!(pixels[0], 255); // R
    assert_eq!(pixels[1], 0); // G
    assert_eq!(pixels[2], 0); // B
    assert_eq!(pixels[3], 255); // A
}

#[test]
fn test_decode_color_redefinition() {
    // SIXEL that redefines the same color index multiple times
    let sixel_data = b"\x1bPq\
        #0;2;100;0;0~$-\
        #0;2;0;100;0~$-\
        #0;2;0;0;100~\
        \x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (pixels, width, height) = (image.pixels, image.width, image.height);
    assert_eq!(width, 1);
    assert_eq!(height, 18); // 3 lines of 6 pixels

    // First line should be red
    assert_eq!(pixels[0], 255); // R
    assert_eq!(pixels[1], 0); // G
    assert_eq!(pixels[2], 0); // B
    assert_eq!(pixels[3], 255); // A

    // Second line should be green
    let offset = width * 6 * 4; // Second sixel line
    assert_eq!(pixels[offset], 0); // R
    assert_eq!(pixels[offset + 1], 255); // G
    assert_eq!(pixels[offset + 2], 0); // B
    assert_eq!(pixels[offset + 3], 255); // A

    // Third line should be blue
    let offset = width * 12 * 4; // Third sixel line
    assert_eq!(pixels[offset], 0); // R
    assert_eq!(pixels[offset + 1], 0); // G
    assert_eq!(pixels[offset + 2], 255); // B
    assert_eq!(pixels[offset + 3], 255); // A
}

#[test]
fn test_decode_rgb_output() {
    // Test that RGB decoder works with multiple colors
    let sixel_data = b"\x1bPq#1;2;50;50;0#2;2;0;50;50#1~#2~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (pixels, width, height) = (image.pixels, image.width, image.height);
    assert_eq!(width, 2);
    assert_eq!(height, 6);
    assert_eq!(pixels.len(), width * height * 4); // RGBA: 4 bytes per pixel
}

// ============================================================================
// Roundtrip tests with real PNG images
// ============================================================================

/// Helper function to calculate average and max pixel difference
fn compare_images(original: &[u8], decoded: &[u8], width: usize, height: usize) -> (f64, u8) {
    let compare_len = (width * height * 4).min(original.len()).min(decoded.len());
    let mut total_diff: u64 = 0;
    let mut max_diff: u8 = 0;

    for i in 0..compare_len {
        // Skip alpha channel comparison (every 4th byte starting at index 3)
        if i % 4 == 3 {
            continue;
        }
        let diff = (original[i] as i32 - decoded[i] as i32)
            .unsigned_abs()
            .min(255) as u8;
        total_diff += diff as u64;
        max_diff = max_diff.max(diff);
    }

    // Only count RGB channels (3 out of every 4 bytes)
    let rgb_count = (compare_len / 4) * 3;
    let avg_diff = if rgb_count > 0 {
        total_diff as f64 / rgb_count as f64
    } else {
        0.0
    };

    (avg_diff, max_diff)
}

#[test]
fn test_roundtrip_test_page_png() {
    // Load test_page.png
    let img = image::open("tests/data/test_page.png").expect("Failed to open test_page.png");
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let original_pixels = rgba_img.into_raw();

    // Encode to SIXEL
    let sixel = sixel_encode(
        &original_pixels,
        width as usize,
        height as usize,
        &EncodeOptions::default(),
    )
    .expect("Failed to encode test_page.png");

    assert!(!sixel.is_empty(), "SIXEL output should not be empty");
    assert!(
        sixel.starts_with("\x1bPq"),
        "SIXEL should start with DCS introducer"
    );
    assert!(
        sixel.ends_with("\x1b\\"),
        "SIXEL should end with string terminator"
    );

    // Decode back
    let decoded = sixel_decode(sixel.as_bytes()).expect("Failed to decode test_page.png sixel");
    let (decoded_pixels, decoded_width, decoded_height) =
        (decoded.pixels, decoded.width, decoded.height);

    // Check dimensions (height may be rounded up to multiple of 6)
    assert_eq!(decoded_width, width as usize, "Width should match");
    assert!(
        decoded_height >= height as usize,
        "Decoded height should be >= original"
    );

    // Compare quality
    let (avg_diff, max_diff) = compare_images(
        &original_pixels,
        &decoded_pixels,
        width as usize,
        height as usize,
    );

    println!(
        "test_page.png roundtrip: avg_diff={:.2}, max_diff={}",
        avg_diff, max_diff
    );

    // With imagequant, we expect good quality
    assert!(
        avg_diff < 15.0,
        "Average pixel difference should be < 15, got {:.2}",
        avg_diff
    );
}

#[test]
fn test_roundtrip_transparency_png() {
    // Load transparency.png
    let img = image::open("tests/data/transparency.png").expect("Failed to open transparency.png");
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let original_pixels = rgba_img.into_raw();

    // Encode to SIXEL
    let sixel = sixel_encode(
        &original_pixels,
        width as usize,
        height as usize,
        &EncodeOptions::default(),
    )
    .expect("Failed to encode transparency.png");

    assert!(!sixel.is_empty(), "SIXEL output should not be empty");
    // Note: With transparency, the DCS header includes P2=1 parameter: ESC P 0;1;0 q
    assert!(
        sixel.starts_with("\x1bP"),
        "SIXEL should start with DCS introducer"
    );
    assert!(sixel.contains('q'), "SIXEL should contain 'q' command");
    assert!(
        sixel.ends_with("\x1b\\"),
        "SIXEL should end with string terminator"
    );

    // Decode back
    let decoded_image =
        sixel_decode(sixel.as_bytes()).expect("Failed to decode transparency.png sixel");
    let (decoded_pixels, decoded_width, decoded_height) = (
        decoded_image.pixels,
        decoded_image.width,
        decoded_image.height,
    );

    // Check dimensions
    // SIXEL works in 6-pixel bands, so height may be different
    // Also, if the bottom rows are all transparent, they may not be encoded
    assert_eq!(decoded_width, width as usize, "Width should match");
    // Height can be smaller if trailing rows are transparent, or larger if padded to 6-pixel boundary
    println!(
        "transparency.png: original {}x{}, decoded {}x{}",
        width, height, decoded_width, decoded_height
    );

    // Compare quality - only compare opaque pixels within the decoded area
    let mut total_diff: u64 = 0;
    let mut max_diff: u8 = 0;
    let mut opaque_pixel_count = 0u64;
    let mut transparent_match_count = 0u64;

    let compare_height = height.min(decoded_height as u32);

    for y in 0..compare_height {
        for x in 0..width {
            let orig_idx = ((y * width + x) * 4) as usize;
            let dec_idx = ((y * decoded_width as u32 + x) * 4) as usize;

            let orig_alpha = original_pixels[orig_idx + 3];
            let dec_alpha = decoded_pixels[dec_idx + 3];

            if orig_alpha >= 128 {
                // Original pixel is opaque, compare RGB
                opaque_pixel_count += 1;
                for c in 0..3 {
                    let diff = (original_pixels[orig_idx + c] as i32
                        - decoded_pixels[dec_idx + c] as i32)
                        .unsigned_abs()
                        .min(255) as u8;
                    total_diff += diff as u64;
                    max_diff = max_diff.max(diff);
                }
            } else {
                // Original pixel is transparent, decoded should also be transparent
                if dec_alpha < 128 {
                    transparent_match_count += 1;
                }
            }
        }
    }

    let avg_diff = if opaque_pixel_count > 0 {
        total_diff as f64 / (opaque_pixel_count * 3) as f64
    } else {
        0.0
    };

    println!("transparency.png roundtrip: avg_diff={:.2}, max_diff={}, opaque_pixels={}, transparent_matches={}", 
             avg_diff, max_diff, opaque_pixel_count, transparent_match_count);

    // With imagequant, we expect good quality for opaque pixels
    assert!(
        avg_diff < 15.0,
        "Average pixel difference should be < 15, got {:.2}",
        avg_diff
    );

    // Verify that we have some opaque pixels that were compared
    assert!(
        opaque_pixel_count > 0,
        "Should have some opaque pixels to compare"
    );
}

#[test]
fn test_encode_beelitz_heilstaetten_png() {
    // Load beelitz_heilstätten.png (larger, more complex image)
    let img = image::open("tests/data/beelitz_heilstätten.png")
        .expect("Failed to open beelitz_heilstätten.png");
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let original_pixels = rgba_img.into_raw();

    println!("beelitz_heilstätten.png: {}x{}", width, height);

    // Encode to SIXEL - this should just work without errors
    let sixel = sixel_encode(
        &original_pixels,
        width as usize,
        height as usize,
        &EncodeOptions::default(),
    )
    .expect("Failed to encode beelitz_heilstätten.png");

    assert!(!sixel.is_empty(), "SIXEL output should not be empty");
    assert!(
        sixel.starts_with("\x1bPq"),
        "SIXEL should start with DCS introducer"
    );
    assert!(
        sixel.ends_with("\x1b\\"),
        "SIXEL should end with string terminator"
    );

    println!(
        "beelitz_heilstätten.png encoded to {} bytes of SIXEL",
        sixel.len()
    );

    // Optionally decode to verify it's valid SIXEL
    let result = sixel_decode(sixel.as_bytes());
    assert!(result.is_ok(), "Encoded SIXEL should be decodable");

    let decoded_image = result.unwrap();
    let (_, decoded_width, decoded_height) = (
        decoded_image.pixels,
        decoded_image.width,
        decoded_image.height,
    );
    assert_eq!(decoded_width, width as usize, "Decoded width should match");
    assert!(
        decoded_height >= height as usize,
        "Decoded height should be >= original"
    );
}
