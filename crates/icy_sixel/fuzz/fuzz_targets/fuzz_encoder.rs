#![no_main]

use arbitrary::Arbitrary;
use icy_sixel::{sixel_encode, EncodeOptions, QuantizeMethod};
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    width: u8,
    height: u8,
    pixels: Vec<u8>,
    max_colors: u16,
    diffusion: f32,
    kmeans: bool,
}

fuzz_target!(|input: FuzzInput| {
    let width = usize::from(input.width).max(1);
    let height = usize::from(input.height).max(1);

    // Ensure we have enough pixels (RGBA = 4 bytes per pixel)
    let expected_size = width * height * 4;
    if input.pixels.len() < expected_size {
        return;
    }

    let pixels = &input.pixels[..expected_size];
    let opts = EncodeOptions {
        max_colors: input.max_colors.clamp(2, 256),
        // Include non-finite and out-of-range values to exercise sanitization.
        diffusion: input.diffusion,
        quantize_method: if input.kmeans { QuantizeMethod::kmeans() } else { QuantizeMethod::Wu },
    };

    // The encoder should never panic
    let _ = sixel_encode(pixels, width, height, &opts);
});
