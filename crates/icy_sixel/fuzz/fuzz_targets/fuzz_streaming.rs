#![no_main]

use icy_sixel::{DcsSettings, SixelDecoder, SixelFeedStatus, SixelImage};
use libfuzzer_sys::fuzz_target;

fn assert_image(actual: &SixelImage, expected: &SixelImage) {
    assert_eq!(actual.dimensions(), expected.dimensions());
    assert_eq!(actual.pixels, expected.pixels);
    assert_eq!(actual.aspect_ratio, expected.aspect_ratio);
    assert_eq!(actual.background_mode, expected.background_mode);
}

fuzz_target!(|data: &[u8]| {
    let settings = DcsSettings::new(Some(9), Some(1), None);
    let mut reference = SixelDecoder::new();
    let mut decoder = SixelDecoder::new();
    let expected = reference.decode_from_dcs(data, settings);
    let actual = (|| {
        let mut frame = decoder.begin_frame(settings)?;
        let size = 1 + usize::from(data.first().copied().unwrap_or(0)) % 32;
        for chunk in data.chunks(size) {
            let empty = frame.feed(&[])?;
            assert_eq!(empty.consumed, 0);
            assert_eq!(empty.status, SixelFeedStatus::NeedMoreData);
            let progress = frame.feed(chunk)?;
            match progress.status {
                SixelFeedStatus::NeedMoreData => assert_eq!(progress.consumed, chunk.len()),
                SixelFeedStatus::Terminated(byte) => {
                    assert_eq!(chunk[progress.consumed], byte);
                    break;
                }
            }
        }
        frame.finish()
    })();
    match (actual, expected) {
        (Ok(actual), Ok(expected)) => assert_image(&actual, &expected),
        (Err(actual), Err(expected)) => assert_eq!(actual.to_string(), expected.to_string()),
        (actual, expected) => panic!("streaming: {actual:?}; one-shot: {expected:?}"),
    }
    let actual = decoder.decode_from_dcs(b"#0@#1@#42@#255@", settings).unwrap();
    let expected = reference.decode_from_dcs(b"#0@#1@#42@#255@", settings).unwrap();
    assert_image(&actual, &expected);
});
