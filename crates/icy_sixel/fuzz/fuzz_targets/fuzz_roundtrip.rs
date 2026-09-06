#![no_main]

use arbitrary::Arbitrary;
use icy_sixel::{sixel_encode, EncodeOptions, SixelImage};
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    width: u8,
    height: u8,
    pixels: Vec<u8>,
}

fuzz_target!(|input: FuzzInput| {
    // Keep quantization work bounded for repeated fuzz runs.
    let width = usize::from(input.width).clamp(1, 64);
    let height = usize::from(input.height).clamp(1, 64);

    // Ensure we have enough pixels (RGBA = 4 bytes per pixel)
    let expected_size = width * height * 4;
    if input.pixels.len() < expected_size {
        return;
    }

    let pixels = &input.pixels[..expected_size];
    let opts = EncodeOptions::default();

    // This is valid, size-bounded RGBA input; encoding must succeed, and the
    // decoder must accept every successful encoder output.
    let sixel = sixel_encode(pixels, width, height, &opts).expect("valid RGBA input must encode");
    let decoded = SixelImage::decode(sixel.as_bytes()).expect("encoder output must decode");

    assert_eq!(decoded.width, width);
    // A partially painted final band can extend the raster by up to five rows.
    assert!((height..=height.div_ceil(6) * 6).contains(&decoded.height));
    assert_eq!(decoded.pixels.len(), decoded.width * decoded.height * 4);
    // Quantization changes RGB, but the binary alpha mask must survive exactly.
    for (source, result) in pixels.as_chunks::<4>().0.iter().zip(decoded.pixels.as_chunks::<4>().0) {
        assert_eq!(result[3], if source[3] < 128 { 0 } else { 255 });
    }
    assert!(decoded.pixels[expected_size..].as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 0));
});
