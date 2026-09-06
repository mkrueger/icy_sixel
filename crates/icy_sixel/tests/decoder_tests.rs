use icy_sixel::*;

#[test]
fn background_is_independent_of_raster_preallocation() {
    for p2 in [None, Some(0), Some(1), Some(2)] {
        let settings = DcsSettings::new(None, p2, None);
        let expected = if p2 == Some(1) { [0, 0, 0, 0] } else { [0, 0, 0, 255] };
        for payload in [b"#0;2;100;0;0?".as_slice(), b"\"1;1;1;6#0;2;100;0;0?", b"#0;2;100;0;0\"1;1;1;6?"] {
            let image = SixelImage::decode_from_dcs(payload, settings).unwrap();
            assert_eq!(image.dimensions(), (1, 6));
            assert_eq!(image.pixels, expected.repeat(6));
        }
    }
}

#[test]
fn palette_changes_preserve_background_during_canvas_growth() {
    let drawing = "#0;2;100;0;0@?#0;2;0;100;0?-??#0;2;0;0;100@";
    for mode in [BackgroundMode::Opaque, BackgroundMode::Transparent] {
        let settings = DcsSettings::default().with_background_mode(mode);
        let grown = SixelImage::decode_from_dcs(drawing.as_bytes(), settings).unwrap();
        let sized = SixelImage::decode_from_dcs(format!("\"1;1;3;12{drawing}").as_bytes(), settings).unwrap();
        assert_eq!(grown.dimensions(), (3, 12));
        assert_eq!(grown.pixels, sized.pixels);

        let background = if mode.is_transparent() { [0, 0, 0, 0] } else { [0, 0, 0, 255] };
        let mut expected = background.repeat(3 * 12);
        expected[..4].copy_from_slice(&[255, 0, 0, 255]);
        let blue = (6 * 3 + 2) * 4;
        expected[blue..blue + 4].copy_from_slice(&[0, 0, 255, 255]);
        assert_eq!(grown.pixels, expected);
    }
}

#[test]
fn background_snapshots_shared_palette_at_frame_start() {
    let mut decoder = SixelDecoder::new();
    let settings = DcsSettings::default();
    decoder.decode_from_dcs(b"#0;2;100;0;0~", settings).unwrap();
    assert!(decoder.decode_from_dcs(b"#0;2;0;0;100!65536~", settings).is_err());

    let red = decoder.decode_from_dcs(b"#0;2;0;100;0?", settings).unwrap();
    assert_eq!(red.pixels, [255, 0, 0, 255].repeat(6));
    let green = decoder.decode_from_dcs(b"?", settings).unwrap();
    assert_eq!(green.pixels, [0, 255, 0, 255].repeat(6));

    decoder.reset_palette();
    let black = decoder.decode_from_dcs(b"?", settings).unwrap();
    assert_eq!(black.pixels, [0, 0, 0, 255].repeat(6));
}

#[test]
fn large_hls_hues_are_normalized_without_overflow() {
    for hue in [0, 120, 240, 359, 360, 720, i32::MAX - 240, i32::MAX - 239, i32::MAX] {
        let data = format!("\x1bPq#0;1;{hue};50;100~\x1b\\");
        let normalized = format!("\x1bPq#0;1;{};50;100~\x1b\\", hue % 360);
        let decoded = SixelImage::decode(data.as_bytes()).unwrap();
        let expected = SixelImage::decode(normalized.as_bytes()).unwrap();
        assert_eq!(decoded.pixels, expected.pixels, "hue={hue}");
    }

    // The numeric parser saturates values beyond i32::MAX.
    let saturated = SixelImage::decode(b"\x1bPq#0;1;999999999999999999999999;50;100~\x1b\\").unwrap();
    let expected = format!("\x1bPq#0;1;{};50;100~\x1b\\", i32::MAX % 360);
    assert_eq!(saturated.pixels, SixelImage::decode(expected.as_bytes()).unwrap().pixels);
}

#[test]
fn raster_fill_reaches_allocation_end() {
    // A 16x16 raster uses the complete power-of-two allocation. Filling its last
    // row exercises the end-of-allocation case in the SIMD loop.
    for p2 in [0, 1, 2] {
        let data = format!("\x1bP9;{p2}q\"1;1;16;16\x1b\\");
        let image = SixelImage::decode(data.as_bytes()).unwrap();
        let expected = if p2 == 1 { [0, 0, 0, 0] } else { [0, 0, 0, 255] };
        assert_eq!(image.dimensions(), (16, 16));
        assert_eq!(image.pixels, expected.repeat(16 * 16));
    }
}

#[test]
fn raster_aspect_ratio_overrides_dcs_metadata() {
    for (raster, expected, corrected_height) in [
        ("1;1", PixelAspectRatio::Square, 6),
        ("0;0", PixelAspectRatio::Square, 6),
        ("2147483647;2147483647", PixelAspectRatio::Square, 6),
        ("2;1", PixelAspectRatio::Ratio2To1, 12),
        ("6;2", PixelAspectRatio::Ratio3To1, 18),
        ("10;2", PixelAspectRatio::Ratio5To1, 30),
    ] {
        for p1 in ["", "0", "9"] {
            let data = format!("\x1bP{p1}q\"{raster};1;6~\x1b\\");
            let image = SixelImage::decode(data.as_bytes()).unwrap();
            assert_eq!(image.aspect_ratio, expected);
            assert_eq!(image.corrected_dimensions(), (1, corrected_height));

            let encoded = image.encode().unwrap();
            assert_eq!(SixelImage::decode(encoded.as_bytes()).unwrap().aspect_ratio, expected);
        }
    }
}

#[test]
fn raster_aspect_ratio_is_frame_local() {
    let mut decoder = SixelDecoder::new();
    let settings = DcsSettings::default().with_pixel_aspect_ratio(PixelAspectRatio::Ratio5To1);
    let first = decoder.decode_from_dcs(b"\"1;1;1;6~", settings).unwrap();
    assert_eq!(first.aspect_ratio, PixelAspectRatio::Square);
    let second = decoder.decode_from_dcs(b"~", settings).unwrap();
    assert_eq!(second.aspect_ratio, PixelAspectRatio::Ratio5To1);
}

#[test]
fn unsupported_raster_aspect_ratio_falls_back_to_dcs() {
    // The public enum cannot represent arbitrary ratios without an API change.
    let settings = DcsSettings::default().with_pixel_aspect_ratio(PixelAspectRatio::Ratio3To1);
    for payload in [b"\"4;1;1;6~".as_slice(), b"\"1;2;1;6~", b"\"2~"] {
        let image = SixelImage::decode_from_dcs(payload, settings).unwrap();
        assert_eq!(image.aspect_ratio, PixelAspectRatio::Ratio3To1);
    }
}

#[test]
fn shared_palette_survives_between_images() {
    let mut decoder = SixelDecoder::new();

    decoder.decode_from_dcs(b"#42;2;100;0;0#42~", DcsSettings::default()).unwrap();
    let image = decoder.decode_from_dcs(b"#42~", DcsSettings::default()).unwrap();

    assert_eq!(&image.pixels[..4], &[255, 0, 0, 255]);
}

#[test]
fn decoders_do_not_share_palettes() {
    let mut first = SixelDecoder::new();
    let mut second = SixelDecoder::new();

    first.decode_from_dcs(b"#42;2;100;0;0#42~", DcsSettings::default()).unwrap();
    let image = second.decode_from_dcs(b"#42~", DcsSettings::default()).unwrap();

    assert_ne!(&image.pixels[..4], &[255, 0, 0, 255]);
}

#[test]
fn reset_palette_restores_defaults() {
    let mut decoder = SixelDecoder::new();

    decoder.decode_from_dcs(b"#42;2;100;0;0#42~", DcsSettings::default()).unwrap();
    decoder.reset_palette();
    let image = decoder.decode_from_dcs(b"#42~", DcsSettings::default()).unwrap();

    assert_ne!(&image.pixels[..4], &[255, 0, 0, 255]);
}

#[test]
fn failed_frame_does_not_mutate_shared_palette() {
    let mut decoder = SixelDecoder::new();

    decoder.decode_from_dcs(b"#42;2;100;0;0#42~", DcsSettings::default()).unwrap();
    assert!(decoder.decode_from_dcs(b"#42;2;0;100;0!65536~", DcsSettings::default()).is_err());
    let image = decoder.decode_from_dcs(b"#42~", DcsSettings::default()).unwrap();

    assert_eq!(&image.pixels[..4], &[255, 0, 0, 255]);
}

#[test]
fn frame_state_is_reset_between_images() {
    let mut decoder = SixelDecoder::new();

    decoder
        .decode_from_dcs(b"\"1;1;10;20!5~", DcsSettings::default().with_background_mode(BackgroundMode::Transparent))
        .unwrap();
    let image = decoder.decode_from_dcs(b"~", DcsSettings::default()).unwrap();

    assert_eq!((image.width, image.height), (1, 6));
}

#[test]
fn unsized_growth_matches_pre_sized_decode() {
    // Without raster attributes the canvas grows column by column; the result must be
    // byte-identical to the pre-sized path and carry no capacity padding.
    let width = 1500;
    let mut unsized_data = Vec::from(*b"\x1bPq#1;2;100;0;0");
    unsized_data.resize(unsized_data.len() + width, b'~');
    unsized_data.extend_from_slice(b"\x1b\\");

    let mut raster_data = Vec::from(*b"\x1bPq");
    raster_data.extend_from_slice(format!("\"1;1;{};6", width).as_bytes());
    raster_data.extend_from_slice(b"#1;2;100;0;0");
    raster_data.resize(raster_data.len() + width, b'~');
    raster_data.extend_from_slice(b"\x1b\\");

    let grown = SixelImage::decode(&unsized_data).unwrap();
    let pre_sized = SixelImage::decode(&raster_data).unwrap();

    assert_eq!((grown.width, grown.height), (width, 6));
    assert_eq!((grown.width, grown.height), (pre_sized.width, pre_sized.height));
    assert_eq!(grown.pixels.len(), width * 6 * 4);
    assert_eq!(grown.pixels, pre_sized.pixels);
    assert!(grown.pixels.chunks_exact(4).all(|px| px == [255, 0, 0, 255]));
}

#[test]
fn partial_rows_survive_canvas_growth() {
    // Draw a short first band, then a much wider second band: the older, narrower rows must
    // keep their pixels and be padded with background rather than shifted by the new stride.
    let mut data = Vec::from(*b"\x1bPq#1;2;100;0;0#1!3~-#2;2;0;0;100#2!300~");
    data.extend_from_slice(b"\x1b\\");

    let image = SixelImage::decode(&data).unwrap();
    assert_eq!((image.width, image.height), (300, 12));

    let px = |x: usize, y: usize| &image.pixels[(y * image.width + x) * 4..(y * image.width + x) * 4 + 4];
    assert_eq!(px(0, 0), &[255, 0, 0, 255], "first band keeps its red pixels");
    assert_eq!(px(2, 5), &[255, 0, 0, 255], "first band spans all six rows");
    assert_eq!(px(3, 0), &[0, 0, 0, 255], "area beyond the first band is background");
    assert_eq!(px(299, 6), &[0, 0, 255, 255], "second band reaches the full width");
}

#[test]
fn reported_background_mode_matches_pixels() {
    // No P2 parameter: undrawn pixels are filled opaquely, so the metadata must say Opaque.
    let image = SixelImage::decode(b"\x1bPq#1;2;100;0;0#1@\x1b\\").unwrap();
    let undrawn = &image.pixels[image.width * 4..image.width * 4 + 4];
    assert_eq!(image.background_mode, BackgroundMode::Opaque);
    assert_eq!(undrawn[3], 255);

    // P2=1 requests transparency and undrawn pixels keep alpha 0.
    let image = SixelImage::decode(b"\x1bP0;1;0q#1;2;100;0;0#1@\x1b\\").unwrap();
    let undrawn = &image.pixels[image.width * 4..image.width * 4 + 4];
    assert_eq!(image.background_mode, BackgroundMode::Transparent);
    assert_eq!(undrawn[3], 0);

    // P2=2 is an opaque variant.
    let image = SixelImage::decode(b"\x1bP0;2;0q#1;2;100;0;0#1@\x1b\\").unwrap();
    assert_eq!(image.background_mode, BackgroundMode::Opaque);
}

#[test]
fn test_decode_simple_sixel() {
    // Simple 2x2 black square
    let sixel_data = b"\x1bPq\"1;1;2;2#0;2;0;0;0#0~~\x1b\\";

    let result = SixelImage::decode(sixel_data);
    assert!(result.is_ok(), "Decoding should succeed");

    let image = result.unwrap();
    let (pixels, width, height) = (image.pixels, image.width, image.height);
    assert!(width > 0, "Width should be positive");
    assert!(height > 0, "Height should be positive");
    assert_eq!(pixels.len(), width * height * 4, "Pixel buffer size should match dimensions * 4 (RGBA)");
}

#[test]
fn test_decode_with_aspect_ratio() {
    // Test that aspect ratio is parsed from DCS params
    let sixel_data = b"\x1bP2q#0;2;100;0;0#0~~\x1b\\"; // P1=2 means aspect 5:1

    let result = SixelImage::decode(sixel_data);
    assert!(result.is_ok(), "Decoding should succeed");

    let image = result.unwrap();
    assert!(image.width > 0);
    assert!(image.height >= 6);
    assert_eq!(image.aspect_ratio, PixelAspectRatio::Ratio5To1);
}

#[test]
fn test_decode_with_colors() {
    // SIXEL with color definition
    let sixel_data = b"\x1bPq#0;2;100;0;0#0~~@@~~\x1b\\";

    let result = SixelImage::decode(sixel_data);
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

    let result = SixelImage::decode(sixel_data);
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

    let result = SixelImage::decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (_pixels, width, _height) = (image.pixels, image.width, image.height);
    assert_eq!(width, 5, "Width should be 5 (repeat count)");
}

#[test]
fn test_decode_carriage_return() {
    // Test $ (carriage return)
    let sixel_data = b"\x1bPq#0~~$~~\x1b\\";

    let result = SixelImage::decode(sixel_data);
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

    let image = SixelImage::decode(sixel_data).expect("Decoding overlay should work");
    let (pixels, width, height) = (image.pixels, image.width, image.height);
    assert_eq!(width, 1, "Overlay sample should be one column wide");
    assert!(height >= 6, "Single sixel column must span six pixels vertically");

    let stride = width * 4;
    let top = &pixels[0..4];
    let bottom = &pixels[(height - 1) * stride..(height - 1) * stride + 4];

    assert_eq!(top, &[204, 33, 33, 255], "Top rows must keep the red color from the first pass");
    assert_eq!(bottom, &[51, 204, 51, 255], "Bottom row must reflect the green overlay");
}

#[test]
fn test_decode_newline() {
    // Test - (new line)
    let sixel_data = b"\x1bPq#0~~-~~\x1b\\";

    let result = SixelImage::decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let height = image.height;
    assert!(height >= 12, "Height should be at least 12 (two sixel rows)");
}

#[test]
fn test_decode_hls_color() {
    // HLS color space: #Pc;1;Ph;Pl;Ps
    let sixel_data = b"\x1bPq#0;1;120;50;100#0~\x1b\\";

    let result = SixelImage::decode(sixel_data);
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

    let result = SixelImage::decode(sixel_data);
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

    let result = SixelImage::decode(sixel_data);
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

    let result = SixelImage::decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (_pixels, width, height) = (image.pixels, image.width, image.height);
    assert!(width > 0);
    assert!(height > 0);
}

#[test]
fn test_decode_all_sixel_chars() {
    // Test all sixel character values ? to ~
    let sixel_data = b"\x1bPq#0?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\x1b\\";

    let result = SixelImage::decode(sixel_data);
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

    let image = SixelImage::from_rgba(original_pixels.clone(), 2, 2);
    let encoded = image.encode();

    assert!(encoded.is_ok());
    let sixel_str = encoded.unwrap();

    let decoded = SixelImage::decode(sixel_str.as_bytes());
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

    let result = SixelImage::decode(sixel_data);
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

    let result = SixelImage::decode(sixel_data);
    assert!(result.is_ok());

    let image = result.unwrap();
    let (_pixels, width, _height) = (image.pixels, image.width, image.height);
    assert_eq!(width, 100);
}

#[test]
fn test_decode_palette_bounds() {
    // Test palette color index at boundary
    let sixel_data = b"\x1bPq#255;2;50;50;50#255~\x1b\\";

    let result = SixelImage::decode(sixel_data);
    assert!(result.is_ok());

    let _ = result.unwrap();
    // Should handle color 255 correctly
}

#[test]
fn test_decode_escape_sequences() {
    // Test various escape sequence forms
    // ESC P ... ESC \ (7-bit)
    let sixel_data = b"\x1bP0;0;0q#0~\x1b\\";
    let result = SixelImage::decode(sixel_data);
    assert!(result.is_ok());

    // DCS ... ST (8-bit)
    let sixel_data = b"\x90q#0~\x9c";
    let result = SixelImage::decode(sixel_data);
    assert!(result.is_ok());
}

#[test]
fn test_decode_rgb() {
    // Simple SIXEL with a red pixel
    let sixel_data = b"\x1bPq#2;2;100;0;0~\x1b\\";

    let result = SixelImage::decode(sixel_data);
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

    let result = SixelImage::decode(sixel_data);
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

    let result = SixelImage::decode(sixel_data);
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
        let diff = (original[i] as i32 - decoded[i] as i32).unsigned_abs().min(255) as u8;
        total_diff += diff as u64;
        max_diff = max_diff.max(diff);
    }

    // Only count RGB channels (3 out of every 4 bytes)
    let rgb_count = (compare_len / 4) * 3;
    let avg_diff = if rgb_count > 0 { total_diff as f64 / rgb_count as f64 } else { 0.0 };

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
    let image = SixelImage::from_rgba(original_pixels.clone(), width as usize, height as usize);
    let sixel = image.encode().expect("Failed to encode test_page.png");

    assert!(!sixel.is_empty(), "SIXEL output should not be empty");
    assert!(sixel.starts_with("\x1bP9;1;0q"), "SIXEL should start with DCS introducer");
    assert!(sixel.ends_with("\x1b\\"), "SIXEL should end with string terminator");

    // Decode back
    let decoded = SixelImage::decode(sixel.as_bytes()).expect("Failed to decode test_page.png sixel");
    let (decoded_pixels, decoded_width, decoded_height) = (decoded.pixels, decoded.width, decoded.height);

    // Check dimensions (height may be rounded up to multiple of 6)
    assert_eq!(decoded_width, width as usize, "Width should match");
    assert!(decoded_height >= height as usize, "Decoded height should be >= original");

    // Compare quality
    let (avg_diff, max_diff) = compare_images(&original_pixels, &decoded_pixels, width as usize, height as usize);

    println!("test_page.png roundtrip: avg_diff={:.2}, max_diff={}", avg_diff, max_diff);

    // With imagequant, we expect good quality
    assert!(avg_diff < 15.0, "Average pixel difference should be < 15, got {:.2}", avg_diff);
}

#[test]
fn test_roundtrip_transparency_png() {
    // Load transparency.png
    let img = image::open("tests/data/transparency.png").expect("Failed to open transparency.png");
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let original_pixels = rgba_img.into_raw();

    // Encode to SIXEL
    let image = SixelImage::from_rgba(original_pixels.clone(), width as usize, height as usize);
    let sixel = image.encode().expect("Failed to encode transparency.png");

    assert!(!sixel.is_empty(), "SIXEL output should not be empty");
    // Note: With transparency, the DCS header includes P2=1 parameter: ESC P 0;1;0 q
    assert!(sixel.starts_with("\x1bP"), "SIXEL should start with DCS introducer");
    assert!(sixel.contains('q'), "SIXEL should contain 'q' command");
    assert!(sixel.ends_with("\x1b\\"), "SIXEL should end with string terminator");

    // Decode back
    let decoded_image = SixelImage::decode(sixel.as_bytes()).expect("Failed to decode transparency.png sixel");
    let (decoded_pixels, decoded_width, decoded_height) = (decoded_image.pixels, decoded_image.width, decoded_image.height);

    // Check dimensions
    // SIXEL works in 6-pixel bands, so height may be different
    // Also, if the bottom rows are all transparent, they may not be encoded
    assert_eq!(decoded_width, width as usize, "Width should match");
    // Height can be smaller if trailing rows are transparent, or larger if padded to 6-pixel boundary
    println!("transparency.png: original {}x{}, decoded {}x{}", width, height, decoded_width, decoded_height);

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
                    let diff = (original_pixels[orig_idx + c] as i32 - decoded_pixels[dec_idx + c] as i32)
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

    println!(
        "transparency.png roundtrip: avg_diff={:.2}, max_diff={}, opaque_pixels={}, transparent_matches={}",
        avg_diff, max_diff, opaque_pixel_count, transparent_match_count
    );

    // With imagequant, we expect good quality for opaque pixels
    assert!(avg_diff < 15.0, "Average pixel difference should be < 15, got {:.2}", avg_diff);

    // Verify that we have some opaque pixels that were compared
    assert!(opaque_pixel_count > 0, "Should have some opaque pixels to compare");
}

#[test]
fn test_encode_beelitz_heilstaetten_png() {
    // Load beelitz_heilstätten.png (larger, more complex image)
    let img = image::open("tests/data/beelitz_heilstätten.png").expect("Failed to open beelitz_heilstätten.png");
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let original_pixels = rgba_img.into_raw();

    println!("beelitz_heilstätten.png: {}x{}", width, height);

    // Encode to SIXEL - this should just work without errors
    let image = SixelImage::from_rgba(original_pixels, width as usize, height as usize);
    let sixel = image.encode().expect("Failed to encode beelitz_heilstätten.png");

    assert!(!sixel.is_empty(), "SIXEL output should not be empty");
    assert!(sixel.starts_with("\x1bP9;1;0q"), "SIXEL should start with DCS introducer");
    assert!(sixel.ends_with("\x1b\\"), "SIXEL should end with string terminator");

    println!("beelitz_heilstätten.png encoded to {} bytes of SIXEL", sixel.len());

    // Optionally decode to verify it's valid SIXEL
    let result = SixelImage::decode(sixel.as_bytes());
    assert!(result.is_ok(), "Encoded SIXEL should be decodable");

    let decoded_image = result.unwrap();
    let (_, decoded_width, decoded_height) = (decoded_image.pixels, decoded_image.width, decoded_image.height);
    assert_eq!(decoded_width, width as usize, "Decoded width should match");
    assert!(decoded_height >= height as usize, "Decoded height should be >= original");
}
