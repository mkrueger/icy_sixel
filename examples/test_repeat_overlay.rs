use icy_sixel::sixel_decode;

fn main() {
    // Test pattern: draw with repeats then overstrike
    // Pattern: 10 full columns (~), 5 partial (^), 10 full (~)
    // Then carriage return and overstrike with different color
    let test_data = b"\x1bPq\"1;1;30;12#0;2;0;0;0#1;2;100;0;0#0!10~!5^!10~$#1!10?!5_!10?\x1b\\";

    match sixel_decode(test_data) {
        Ok((pixels, width, height)) => {
            println!("Decoded: {}x{} pixels", width, height);

            // Check dimensions
            if width != 25 {
                eprintln!("Width wrong: expected 25, got {}", width);
            }
            if height < 6 {
                eprintln!("Height too small: expected at least 6, got {}", height);
            }

            // Sample some pixels to verify overlay worked

            // First pixel should be from color 1 (red overlay over black)
            let pixel0 = &pixels[0..3];
            println!(
                "First pixel (should be red): R={}, G={}, B={}",
                pixel0[0], pixel0[1], pixel0[2]
            );

            // Pixel at x=12 (in the middle section) should also be red from overlay
            let pixel12 = &pixels[12 * 3..(12 * 3 + 3)];
            println!(
                "Pixel 12 (should be red): R={}, G={}, B={}",
                pixel12[0], pixel12[1], pixel12[2]
            );

            // Last pixel of first row should be red
            let pixel_last = &pixels[(width - 1) * 3..width * 3];
            println!(
                "Last pixel (should be red): R={}, G={}, B={}",
                pixel_last[0], pixel_last[1], pixel_last[2]
            );
        }
        Err(e) => {
            eprintln!("Decoding failed: {}", e);
        }
    }
}
