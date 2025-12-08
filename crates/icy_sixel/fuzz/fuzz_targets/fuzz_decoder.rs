#![no_main]

use libfuzzer_sys::fuzz_target;
use icy_sixel::sixel_decode;

fuzz_target!(|data: &[u8]| {
    // The decoder should never panic, regardless of input
    let _ = sixel_decode(data);
});
