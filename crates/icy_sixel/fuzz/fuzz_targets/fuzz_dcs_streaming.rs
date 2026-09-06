#![no_main]

use icy_sixel::{DcsSettings, SixelDcsFeedStatus, SixelDecoder, SixelImage};
use libfuzzer_sys::fuzz_target;

fn assert_image(actual: &SixelImage, expected: &SixelImage) {
    assert_eq!(actual.dimensions(), expected.dimensions());
    assert_eq!(actual.pixels, expected.pixels);
    assert_eq!(actual.aspect_ratio, expected.aspect_ratio);
    assert_eq!(actual.background_mode, expected.background_mode);
}

fn decode(decoder: &mut SixelDecoder, input: &[u8], size: usize) -> icy_sixel::Result<(SixelImage, usize, SixelDcsFeedStatus)> {
    let mut stream = decoder.begin_dcs();
    let mut consumed = 0;
    let mut status = SixelDcsFeedStatus::NeedMoreData;
    for chunk in input.chunks(size) {
        let empty = stream.feed(b"")?;
        assert_eq!(empty.consumed, 0);
        assert_eq!(empty.status, SixelDcsFeedStatus::NeedMoreData);
        let progress = stream.feed(chunk)?;
        assert!(progress.consumed <= chunk.len());
        consumed += progress.consumed;
        status = progress.status;
        match status {
            SixelDcsFeedStatus::NeedMoreData => assert_eq!(progress.consumed, chunk.len()),
            _ => {
                match status {
                    SixelDcsFeedStatus::Complete => assert!(input[..consumed].ends_with(b"\x1b\\") || input[consumed - 1] == 0x9c),
                    SixelDcsFeedStatus::Cancelled(byte) | SixelDcsFeedStatus::Interrupted(byte @ 0x1b) => assert_eq!(input[consumed - 1], byte),
                    SixelDcsFeedStatus::Interrupted(byte) => assert_eq!(input[consumed], byte),
                    SixelDcsFeedStatus::NeedMoreData => unreachable!(),
                }
                let repeated = stream.feed(b"trailing data")?;
                assert_eq!(repeated.consumed, 0);
                assert_eq!(repeated.status, status);
                break;
            }
        }
    }
    Ok((stream.finish()?, consumed, status))
}

fn compare(input: &[u8], size: usize, legacy: bool) {
    let mut reference = SixelDecoder::new();
    let mut decoder = SixelDecoder::new();
    let expected = decode(&mut reference, input, input.len().max(1));
    let actual = decode(&mut decoder, input, size);
    match (&actual, &expected) {
        (Ok((actual, actual_count, actual_status)), Ok((expected, expected_count, expected_status))) => {
            assert_image(actual, expected);
            assert_eq!(actual_count, expected_count);
            assert_eq!(actual_status, expected_status);
        }
        (Err(actual), Err(expected)) => assert_eq!(actual.to_string(), expected.to_string()),
        (actual, expected) => panic!("chunked: {actual:?}; single chunk: {expected:?}"),
    }
    if legacy {
        match (&actual, SixelImage::decode(input)) {
            (Ok((actual, _, _)), Ok(expected)) => assert_image(actual, &expected),
            (Err(actual), Err(expected)) => assert_eq!(actual.to_string(), expected.to_string()),
            (actual, expected) => panic!("DCS streaming: {actual:?}; legacy: {expected:?}"),
        }
    }
    let probe = b"#0@#1@#42@#255@";
    assert_image(
        &decoder.decode_from_dcs(probe, DcsSettings::default()).unwrap(),
        &reference.decode_from_dcs(probe, DcsSettings::default()).unwrap(),
    );
}

fuzz_target!(|data: &[u8]| {
    let selector = data.first().copied().unwrap_or(0);
    let size = 1 + usize::from(selector) % 32;
    // Arbitrary framing, including malformed/truncated headers and non-SIXEL input.
    compare(data, size, false);
    // Guarantee payload coverage and compare against the legacy batch entry point.
    let header = if selector & 1 == 0 { b"\x1bP3;1q".as_slice() } else { b"\x909;1q" };
    let terminator = if selector & 2 == 0 { b"\x1b\\".as_slice() } else { b"\x9c" };
    let framed = [header, data, terminator, b"suffix"].concat();
    compare(&framed, size, true);
});
