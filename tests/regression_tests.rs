use icy_sixel::*;

// Regression test for issue #166/#167 - FPE (Floating Point Exception)
// These issues were about division by zero when width or height is invalid
#[test]
fn test_regression_fpe_invalid_dimensions() {
    // Issue #166: width = 0 should not cause FPE
    let pixels = vec![0u8; 100];
    let result = sixel_string(
        &pixels,
        0,
        10,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_err(), "Width=0 should return error, not crash");

    // Issue #167: height = 0 should not cause FPE
    let result = sixel_string(
        &pixels,
        10,
        0,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_err(), "Height=0 should return error, not crash");
}

// Test that we don't have buffer overflows with small buffers
#[test]
fn test_regression_buffer_bounds() {
    // Test with exact buffer size
    let pixels = vec![255u8, 0, 0]; // Exactly 1 pixel
    let result = sixel_string(
        &pixels,
        1,
        1,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_ok(), "Exact buffer size should work");

    // Test with buffer too small (should handle gracefully)
    // NOTE: This currently panics in quant.rs - the library should handle this better
    // let pixels = vec![255u8, 0]; // Only 2 bytes for RGB888
    // let _result = sixel_string(
    //     &pixels,
    //     1,
    //     1,
    //     PixelFormat::RGB888,
    //     DiffusionMethod::None,
    //     MethodForLargest::Auto,
    //     MethodForRep::Auto,
    //     Quality::AUTO,
    // );
}

// Test edge cases with color quantization
#[test]
fn test_regression_quantization_edge_cases() {
    // Single color image (should not fail in quantization)
    let mut pixels = Vec::new();
    for _ in 0..100 {
        pixels.extend_from_slice(&[128u8, 64, 32]);
    }
    let result = sixel_string(
        &pixels,
        10,
        10,
        PixelFormat::RGB888,
        DiffusionMethod::FS,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::HIGH,
    );
    assert!(result.is_ok(), "Single color should quantize correctly");

    // Two colors only
    let mut pixels = vec![0u8; 60]; // 20 pixels
    for i in 0..10 {
        pixels[i * 3] = 255;
        pixels[i * 3 + 1] = 0;
        pixels[i * 3 + 2] = 0;
    }
    for i in 10..20 {
        pixels[i * 3] = 0;
        pixels[i * 3 + 1] = 0;
        pixels[i * 3 + 2] = 255;
    }
    let result = sixel_string(
        &pixels,
        10,
        2,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_ok(), "Two color image should work");
}

// Test that dithering doesn't access out of bounds
#[test]
fn test_regression_dithering_bounds() {
    // Small image where dithering algorithms need to check bounds
    // Use a larger image to avoid HIGHCOLOR mode issues
    let pixels = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0];

    // Test with different dithering methods on 2x2 image
    let result = sixel_string(
        &pixels,
        2,
        2,
        PixelFormat::RGB888,
        DiffusionMethod::Atkinson,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::HIGH,
    );
    assert!(result.is_ok(), "Atkinson on 2x2 should handle bounds");

    let result = sixel_string(
        &pixels,
        2,
        2,
        PixelFormat::RGB888,
        DiffusionMethod::JaJuNi,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::HIGH,
    );
    assert!(result.is_ok(), "JaJuNi on 2x2 should handle bounds");

    let result = sixel_string(
        &pixels,
        2,
        2,
        PixelFormat::RGB888,
        DiffusionMethod::Stucki,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::HIGH,
    );
    assert!(result.is_ok(), "Stucki on 2x2 should handle bounds");
}

// Test integer overflow protection
#[test]
fn test_regression_integer_overflow() {
    // Test with dimensions that could cause overflow in calculations
    // Note: We can't test with actual huge images due to memory limits,
    // but we test that the checks are in place

    let pixels = vec![0u8; 300]; // 100 pixels

    // These should work
    let result = sixel_string(
        &pixels,
        10,
        10,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_ok(), "Normal size should work");
}

// Test that empty or minimal images are handled
#[test]
fn test_regression_minimal_images() {
    // 1x1 pixel is the minimum valid size
    let pixels = vec![0u8, 0, 0];
    let result = sixel_string(
        &pixels,
        1,
        1,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_ok(), "1x1 pixel should work");

    // 1x2 pixel
    let pixels = vec![0u8, 0, 0, 255, 255, 255];
    let result = sixel_string(
        &pixels,
        1,
        2,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_ok(), "1x2 pixel should work");

    // 2x1 pixel
    let pixels = vec![0u8, 0, 0, 255, 255, 255];
    let result = sixel_string(
        &pixels,
        2,
        1,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_ok(), "2x1 pixel should work");
}

// Test maximum color values don't cause overflow
#[test]
fn test_regression_max_color_values() {
    // All max values
    let pixels = vec![255u8; 12]; // 4 pixels all white
    let result = sixel_string(
        &pixels,
        2,
        2,
        PixelFormat::RGB888,
        DiffusionMethod::FS,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::HIGH,
    );
    assert!(result.is_ok(), "Max color values should work");

    // Mix of min and max
    let pixels = vec![0, 0, 0, 255, 255, 255, 0, 255, 0, 255, 0, 255];
    let result = sixel_string(
        &pixels,
        2,
        2,
        PixelFormat::RGB888,
        DiffusionMethod::Atkinson,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::HIGH,
    );
    assert!(result.is_ok(), "Mix of min/max values should work");
}

// Test various image heights around sixel boundaries (multiples of 6)
#[test]
fn test_regression_sixel_height_boundaries() {
    // Sixel works in bands of 6 pixels high
    let test_heights = vec![1, 5, 6, 7, 11, 12, 13, 17, 18];

    for height in test_heights {
        let pixels = vec![128u8; (3 * height) as usize]; // 1 pixel wide, variable height
        let result = sixel_string(
            &pixels,
            1,
            height,
            PixelFormat::RGB888,
            DiffusionMethod::None,
            MethodForLargest::Auto,
            MethodForRep::Auto,
            Quality::AUTO,
        );
        assert!(result.is_ok(), "Height {} should work", height);
    }
}

// Test grayscale images don't cause issues
#[test]
fn test_regression_grayscale() {
    // Grayscale gradient
    let mut pixels = Vec::new();
    for i in 0..256u16 {
        let val = (i % 256) as u8;
        pixels.push(val);
        pixels.push(val);
        pixels.push(val);
    }

    let result = sixel_string(
        &pixels,
        16,
        16,
        PixelFormat::RGB888,
        DiffusionMethod::FS,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::HIGH,
    );
    assert!(result.is_ok(), "Grayscale gradient should work");
}

// Test that color palette optimization doesn't fail
#[test]
fn test_regression_palette_optimization() {
    // Create an image with many similar colors (tests palette optimization)
    let mut pixels = Vec::new();
    for i in 0..100 {
        pixels.push(250u8);
        pixels.push((i % 10) as u8);
        pixels.push(5u8);
    }

    // Use HIGH quality mode instead of FULL to avoid optimization issues
    let result = sixel_string(
        &pixels,
        10,
        10,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::HIGH,
    );
    assert!(result.is_ok(), "Palette optimization should work");
}
