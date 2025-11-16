use icy_sixel::*;

#[test]
fn test_decode_simple_sixel() {
    // Simple 2x2 black square
    let sixel_data = b"\x1bPq\"1;1;2;2#0;2;0;0;0#0~~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok(), "Decoding should succeed");

    let (pixels, width, height) = result.unwrap();
    assert!(width > 0, "Width should be positive");
    assert!(height > 0, "Height should be positive");
    assert_eq!(
        pixels.len(),
        width * height * 4,
        "Pixel buffer size should match dimensions * 4 (RGBA)"
    );
}

#[test]
fn test_decode_from_dcs_simple() {
    // Test the new DCS parameter API with just sixel data (no DCS header)
    // Simple sixel data: color 0 (black), one sixel character
    let sixel_data = b"#0~\x1b\\";

    let result = sixel_decode_from_dcs(None, None, None, sixel_data);
    assert!(result.is_ok(), "Decoding should succeed");

    let (pixels, width, height) = result.unwrap();
    assert!(width > 0, "Width should be positive");
    assert!(height >= 6, "Height should be at least 6 (one sixel row)");
    assert_eq!(
        pixels.len(),
        width * height * 4,
        "Pixel buffer size should match dimensions * 4 (RGBA)"
    );
}

#[test]
fn test_decode_from_dcs_with_params() {
    // Test with DCS parameters: aspect_ratio=2 (5:1), grid_size=10
    let sixel_data = b"#0~#1;2;0;100;0~\x1b\\";

    let result = sixel_decode_from_dcs(Some(2), None, Some(10), sixel_data);
    assert!(result.is_ok(), "Decoding with parameters should succeed");

    let (pixels, width, height) = result.unwrap();
    assert!(width > 0);
    assert!(height > 0);
    assert_eq!(pixels.len(), width * height * 4);
}

#[test]
fn test_decode_with_colors() {
    // SIXEL with color definition
    let sixel_data = b"\x1bPq#0;2;100;0;0#0~~@@~~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let (pixels, width, height) = result.unwrap();
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

    let (_pixels, width, height) = result.unwrap();
    assert!(width > 0);
    assert!(height > 0);
}

#[test]
fn test_decode_with_repeat() {
    // Test repeat count !
    let sixel_data = b"\x1bPq#0!5~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let (_pixels, width, _height) = result.unwrap();
    assert_eq!(width, 5, "Width should be 5 (repeat count)");
}

#[test]
fn test_decode_carriage_return() {
    // Test $ (carriage return)
    let sixel_data = b"\x1bPq#0~~$~~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let (_pixels, width, _height) = result.unwrap();
    assert_eq!(width, 2, "Width should be 2");
}

#[test]
fn test_decode_color_overlay_preserves_previous_pixels() {
    // First draw red across all six rows, then return and draw green only on the bottom row.
    // The previously drawn red pixels must survive in rows where the second pass has zero bits.
    let sixel_data = b"\x1bPq#2~$#3_\x1b\\";

    let (pixels, width, height) = sixel_decode(sixel_data).expect("Decoding overlay should work");
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

    let (_pixels, _width, height) = result.unwrap();
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

    let (pixels, _width, _height) = result.unwrap();
    // Color 0 should be defined with HLS values
    assert!(pixels.len() >= 3);
}

#[test]
fn test_decode_rgb_color() {
    // RGB color space: #Pc;2;Pr;Pg;Pb
    let sixel_data = b"\x1bPq#0;2;100;50;0#0~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let (pixels, _width, _height) = result.unwrap();
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

    let (_pixels, width, height) = result.unwrap();
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

    let (_pixels, width, height) = result.unwrap();
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

    let (_pixels, width, _height) = result.unwrap();
    // Should decode the entire range of sixel characters
    assert!(width > 60, "Should have decoded many characters");
}

#[test]
fn test_decode_roundtrip_simple() {
    // Create a simple image, encode it, decode it, check dimensions
    let original_pixels = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0];

    let encoded = sixel_string(
        &original_pixels,
        2,
        2,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::LOW,
    );

    assert!(encoded.is_ok());
    let sixel_str = encoded.unwrap();

    let decoded = sixel_decode(sixel_str.as_bytes());
    assert!(decoded.is_ok());

    let (pixels, width, height) = decoded.unwrap();

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

    let (_pixels, width, height) = result.unwrap();
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

    let (_pixels, width, _height) = result.unwrap();
    assert_eq!(width, 100);
}

#[test]
fn test_decode_palette_bounds() {
    // Test palette color index at boundary
    let sixel_data = b"\x1bPq#255;2;50;50;50#255~\x1b\\";

    let result = sixel_decode(sixel_data);
    assert!(result.is_ok());

    let (_pixels, _width, _height) = result.unwrap();
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

    let (pixels, width, height) = result.unwrap();
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

    let (pixels, width, height) = result.unwrap();
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

    let (pixels, width, height) = result.unwrap();
    assert_eq!(width, 2);
    assert_eq!(height, 6);
    assert_eq!(pixels.len(), width * height * 4); // RGBA: 4 bytes per pixel
}
