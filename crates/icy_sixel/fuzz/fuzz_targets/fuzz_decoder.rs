#![no_main]

use icy_sixel::SixelImage;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The decoder should never panic, regardless of input
    let _ = SixelImage::decode(data);
});
