use icy_sixel::*;

fn main() {
    // Simple SIXEL test
    let sixel_data = b"\x1bPq#0;2;100;50;25#0~\x1b\\";

    let (pixels, width, height) = sixel_decode(sixel_data).expect("Decode failed");

    println!("Decoded: {}x{}", width, height);
    println!("Pixel format: RGBA (4 bytes per pixel)");
    println!(
        "First pixel: R={}, G={}, B={}, A={}",
        pixels[0], pixels[1], pixels[2], pixels[3]
    );

    assert_eq!(pixels[3], 0xFF, "Alpha channel should be 0xFF");
    println!("\n✓ Alpha channel is correctly set to 0xFF (255)");

    // Verify all alpha values
    for i in (0..pixels.len()).step_by(4) {
        assert_eq!(pixels[i + 3], 0xFF, "All alpha values should be 0xFF");
    }
    println!("✓ All {} pixels have alpha = 0xFF", pixels.len() / 4);
}
