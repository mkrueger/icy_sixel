#![no_main]

use libfuzzer_sys::fuzz_target;
use icy_sixel::{sixel_encode, EncodeOptions};
use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    width: u8,
    height: u8,
    pixels: Vec<u8>,
    max_colors: u8,
    quality: u8,
}

fuzz_target!(|input: FuzzInput| {
    // Skip invalid dimensions
    let width = (input.width as usize).max(1).min(256);
    let height = (input.height as usize).max(1).min(256);
    
    // Ensure we have enough pixels (RGBA = 4 bytes per pixel)
    let expected_size = width * height * 4;
    if input.pixels.len() < expected_size {
        return;
    }
    
    let pixels = &input.pixels[..expected_size];
    let opts = EncodeOptions {
        max_colors: (input.max_colors as u16).max(2).min(256),
        quality: input.quality.min(100),
    };
    
    // The encoder should never panic
    let _ = sixel_encode(pixels, width, height, &opts);
});
