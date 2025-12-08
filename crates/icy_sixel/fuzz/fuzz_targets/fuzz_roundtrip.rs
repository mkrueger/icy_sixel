#![no_main]

use libfuzzer_sys::fuzz_target;
use icy_sixel::{sixel_decode, sixel_encode, EncodeOptions};
use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    width: u8,
    height: u8,
    pixels: Vec<u8>,
}

fuzz_target!(|input: FuzzInput| {
    // Skip invalid dimensions
    let width = (input.width as usize).max(1).min(64);
    let height = (input.height as usize).max(1).min(64);
    
    // Ensure we have enough pixels (RGBA = 4 bytes per pixel)
    let expected_size = width * height * 4;
    if input.pixels.len() < expected_size {
        return;
    }
    
    let pixels = &input.pixels[..expected_size];
    let opts = EncodeOptions::default();
    
    // Encode
    let sixel = match sixel_encode(pixels, width, height, &opts) {
        Ok(s) => s,
        Err(_) => return,
    };
    
    // Decode - should never panic
    let decoded = match sixel_decode(sixel.as_bytes()) {
        Ok(img) => img,
        Err(_) => return,
    };
    
    // Basic sanity checks
    assert!(decoded.width >= width, "Decoded width should be >= original");
    // Height might differ due to 6-pixel band alignment
});
