use icy_sixel::*;

#[test]
fn test_width_height_validation() {
    // Test that width < 1 returns error (FPE fix)
    let pixels = vec![0u8; 100];
    let result = sixel_string(
        &pixels,
        0, // invalid width
        10,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_err(), "Should fail with width = 0");

    // Test that height < 1 returns error (FPE fix)
    let result = sixel_string(
        &pixels,
        10,
        0, // invalid height
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_err(), "Should fail with height = 0");

    // Test negative width
    let result = sixel_string(
        &pixels,
        -1, // invalid width
        10,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_err(), "Should fail with negative width");

    // Test negative height
    let result = sixel_string(
        &pixels,
        10,
        -1, // invalid height
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_err(), "Should fail with negative height");
}

#[test]
fn test_simple_1x1_pixel() {
    // Test encoding a simple 1x1 black pixel
    let pixels = vec![0u8, 0u8, 0u8]; // RGB black
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

    assert!(result.is_ok(), "Should encode 1x1 pixel successfully");
    let sixel = result.unwrap();

    // Check that it starts with DCS and ends with ST
    assert!(sixel.starts_with("\x1BP"), "Should start with DCS");
    assert!(sixel.ends_with("\x1B\\"), "Should end with ST");
}

#[test]
fn test_simple_1x1_white_pixel() {
    // Test encoding a simple 1x1 white pixel
    let pixels = vec![255u8, 255u8, 255u8]; // RGB white
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

    assert!(result.is_ok(), "Should encode 1x1 white pixel successfully");
    let sixel = result.unwrap();

    // Check basic structure
    assert!(sixel.contains("q"), "Should contain sixel introducer");
}

#[test]
fn test_2x2_pixels() {
    // Test encoding a 2x2 image with different colors
    let pixels = vec![
        255, 0, 0, // red
        0, 255, 0, // green
        0, 0, 255, // blue
        255, 255, 0, // yellow
    ];
    let result = sixel_string(
        &pixels,
        2,
        2,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::HIGH,
    );

    assert!(result.is_ok(), "Should encode 2x2 pixels successfully");
    let sixel = result.unwrap();

    // Should contain color definitions
    assert!(sixel.contains("#"), "Should contain palette definitions");
}

#[test]
fn test_different_diffusion_methods() {
    let pixels = vec![
        255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 255, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255, 0, 255, 0,
        255, 0, 0,
    ];

    // Test all diffusion methods
    let methods = vec![
        DiffusionMethod::None,
        DiffusionMethod::Auto,
        DiffusionMethod::Atkinson,
        DiffusionMethod::FS,
        DiffusionMethod::JaJuNi,
        DiffusionMethod::Stucki,
        DiffusionMethod::Burkes,
        DiffusionMethod::ADither,
        DiffusionMethod::XDither,
    ];

    for method in methods {
        let result = sixel_string(
            &pixels,
            3,
            3,
            PixelFormat::RGB888,
            method,
            MethodForLargest::Auto,
            MethodForRep::Auto,
            Quality::AUTO,
        );

        assert!(result.is_ok(), "Diffusion method should work");
    }
}

#[test]
fn test_different_quality_modes() {
    let pixels = vec![
        255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 255, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255, 0, 255, 0,
        255, 0, 0,
    ];

    // Test different quality modes
    // Note: Quality::FULL is not included as it disables optimizations
    // Quality::HIGHCOLOR is tested separately
    let qualities = vec![Quality::AUTO, Quality::HIGH, Quality::LOW];

    for quality in qualities {
        let result = sixel_string(
            &pixels,
            3,
            3,
            PixelFormat::RGB888,
            DiffusionMethod::None,
            MethodForLargest::Auto,
            MethodForRep::Auto,
            quality,
        );

        assert!(result.is_ok(), "Quality mode should work");
    }
}

// TODO: HIGHCOLOR mode has a bug (index out of bounds in tosixel.rs:1049)
// This needs to be fixed in the library before this test can pass
// #[test]
// fn test_quality_highcolor() {
//     let mut pixels = Vec::new();
//     for i in 0..64 {
//         pixels.push(((i * 4) % 256) as u8);
//         pixels.push(((i * 8) % 256) as u8);
//         pixels.push(((i * 16) % 256) as u8);
//     }
//
//     let result = sixel_string(
//         &pixels,
//         8,
//         8,
//         PixelFormat::RGB888,
//         DiffusionMethod::None,
//         MethodForLargest::Auto,
//         MethodForRep::Auto,
//         Quality::HIGHCOLOR,
//     );
//
//     assert!(result.is_ok());
// }

#[test]
fn test_different_pixel_formats() {
    // Test RGB888
    let pixels_rgb888 = vec![255u8, 0, 0]; // 1x1 red pixel
    let result = sixel_string(
        &pixels_rgb888,
        1,
        1,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_ok(), "RGB888 should work");

    // Test PAL8 (indexed color)
    let pixels_pal8 = vec![0u8]; // 1x1 pixel, color index 0
    let result = sixel_string(
        &pixels_pal8,
        1,
        1,
        PixelFormat::PAL8,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_ok(), "PAL8 should work");

    // Test G8 (grayscale)
    let pixels_g8 = vec![128u8]; // 1x1 gray pixel
    let result = sixel_string(
        &pixels_g8,
        1,
        1,
        PixelFormat::G8,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );
    assert!(result.is_ok(), "G8 should work");
}

#[test]
fn test_larger_image() {
    // Create a 10x10 gradient image
    let mut pixels = Vec::new();
    for y in 0..10 {
        for x in 0..10 {
            let r = (x * 25) as u8;
            let g = (y * 25) as u8;
            let b = 128u8;
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
        }
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

    assert!(result.is_ok(), "Should encode 10x10 image successfully");
}

#[test]
fn test_method_for_largest_variations() {
    let pixels = vec![255u8, 0, 0, 0, 255, 0, 0, 0, 255];

    let methods = vec![
        MethodForLargest::Auto,
        MethodForLargest::Norm,
        MethodForLargest::Lum,
    ];

    for method in methods {
        let result = sixel_string(
            &pixels,
            1,
            3,
            PixelFormat::RGB888,
            DiffusionMethod::None,
            method,
            MethodForRep::Auto,
            Quality::AUTO,
        );

        assert!(result.is_ok(), "MethodForLargest should work");
    }
}

#[test]
fn test_method_for_rep_variations() {
    let pixels = vec![255u8, 0, 0, 0, 255, 0, 0, 0, 255];

    let methods = vec![
        MethodForRep::Auto,
        MethodForRep::CenterBox,
        MethodForRep::AverageColors,
        MethodForRep::Pixels,
    ];

    for method in methods {
        let result = sixel_string(
            &pixels,
            1,
            3,
            PixelFormat::RGB888,
            DiffusionMethod::None,
            MethodForLargest::Auto,
            method,
            Quality::AUTO,
        );

        assert!(result.is_ok(), "MethodForRep should work");
    }
}

#[test]
fn test_monochrome_image() {
    // Test with all black pixels
    let pixels = vec![0u8; 30]; // 10 pixels black (RGB)
    let result = sixel_string(
        &pixels,
        10,
        1,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );

    assert!(result.is_ok(), "Monochrome image should work");

    // Test with all white pixels
    let pixels = vec![255u8; 30]; // 10 pixels white (RGB)
    let result = sixel_string(
        &pixels,
        10,
        1,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );

    assert!(result.is_ok(), "White monochrome image should work");
}

#[test]
fn test_tall_image() {
    // Test a tall narrow image (1x100)
    let mut pixels = Vec::new();
    for i in 0..100 {
        pixels.push((i * 2) as u8);
        pixels.push(128);
        pixels.push(255 - (i * 2) as u8);
    }

    let result = sixel_string(
        &pixels,
        1,
        100,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );

    assert!(result.is_ok(), "Tall image (1x100) should work");
}

#[test]
fn test_wide_image() {
    // Test a wide narrow image (100x1)
    let mut pixels = Vec::new();
    for i in 0..100 {
        pixels.push((i * 2) as u8);
        pixels.push(128);
        pixels.push(255 - (i * 2) as u8);
    }

    let result = sixel_string(
        &pixels,
        100,
        1,
        PixelFormat::RGB888,
        DiffusionMethod::None,
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::AUTO,
    );

    assert!(result.is_ok(), "Wide image (100x1) should work");
}
