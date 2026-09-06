use icy_sixel::{BackgroundMode, DcsSettings, PixelAspectRatio, SixelDcsFeedStatus, SixelDecoder, SixelImage};

fn assert_image(actual: &SixelImage, expected: &SixelImage) {
    assert_eq!(actual.dimensions(), expected.dimensions());
    assert_eq!(actual.pixels, expected.pixels);
    assert_eq!(actual.aspect_ratio, expected.aspect_ratio);
    assert_eq!(actual.background_mode, expected.background_mode);
}

fn decode_chunks<'a>(decoder: &mut SixelDecoder, chunks: impl IntoIterator<Item = &'a [u8]>) -> icy_sixel::Result<(SixelImage, usize, SixelDcsFeedStatus)> {
    let mut stream = decoder.begin_dcs();
    let mut consumed = 0;
    let mut status = SixelDcsFeedStatus::NeedMoreData;
    for chunk in chunks {
        let progress = stream.feed(chunk)?;
        assert!(progress.consumed <= chunk.len());
        consumed += progress.consumed;
        status = progress.status;
        if status != SixelDcsFeedStatus::NeedMoreData {
            for more in [b"".as_slice(), b"~\x1b\\text"] {
                let repeated = stream.feed(more)?;
                assert_eq!(repeated.consumed, 0);
                assert_eq!(repeated.status, status);
            }
            break;
        }
        assert_eq!(progress.consumed, chunk.len());
    }
    Ok((stream.finish()?, consumed, status))
}

#[test]
fn every_pair_of_splits_preserves_framing_and_pixels() {
    for sequence in [
        b"\x1bPq~\x1b\\".as_slice(),
        b"\x90q~\x9c",
        b"\x1bP3;1;0q#1;2;100;0;0!12~\x9c",
        b"\x90;1q\"6;2;12;6#1;2;100;0;0!1\n2~\x1b\\",
        b"\x1bP9\n;1;q#42;2;0;100;0\x1b\\",
        b"\x1bPq\x9c",
    ] {
        let input = [sequence, b"suffix\x1bPq~~~\x1b\\"].concat();
        let expected = SixelImage::decode(sequence).unwrap();
        for first in 0..=input.len() {
            for second in first..=input.len() {
                let (actual, consumed, status) =
                    decode_chunks(&mut SixelDecoder::new(), [&input[..first], &[], &input[first..second], &[], &input[second..]]).unwrap();
                assert_image(&actual, &expected);
                assert_eq!(consumed, sequence.len());
                assert_eq!(status, SixelDcsFeedStatus::Complete);
            }
        }
    }
}

#[test]
fn header_defaults_saturation_and_excess_parameters_match_settings() {
    for (header, settings) in [
        ("", DcsSettings::default()),
        (";", DcsSettings::new(Some(0), Some(0), None)),
        (";1;", DcsSettings::new(Some(0), Some(1), Some(0))),
        ("3;1;0", DcsSettings::new(Some(3), Some(1), Some(0))),
        ("9999999999999999999999999999999;1;0", DcsSettings::new(Some(65535), Some(1), Some(0))),
        ("9;1;0;1;2;3;4;5;6;7;8;9;10;11;12;13;14;15;16;17;", DcsSettings::new(Some(9), Some(1), Some(0))),
    ] {
        let input = format!("\x1bP{header}q?\x1b\\");
        let expected = SixelImage::decode_from_dcs(b"?", settings).unwrap();
        let (actual, consumed, status) = decode_chunks(&mut SixelDecoder::new(), input.as_bytes().chunks(1)).unwrap();
        assert_image(&actual, &expected);
        assert_eq!(consumed, input.len());
        assert_eq!(status, SixelDcsFeedStatus::Complete);
    }
}

#[test]
fn all_ignored_controls_preserve_split_headers() {
    let expected = SixelImage::decode(b"\x1bP3;1;0q?\x1b\\").unwrap();
    assert_eq!(expected.aspect_ratio, PixelAspectRatio::Ratio3To1);
    assert_eq!(expected.background_mode, BackgroundMode::Transparent);
    for byte in (0..=0x17).chain([0x19, 0x1c, 0x1d, 0x1e, 0x1f, 0x7f]) {
        for split in 0..=5 {
            let header = b"3;1;0";
            let input = [b"\x1bP".as_slice(), &header[..split], &[byte], &header[split..], b"q?\x1b\\"].concat();
            let (actual, _, _) = decode_chunks(&mut SixelDecoder::new(), input.chunks(1)).unwrap();
            assert_image(&actual, &expected);
        }
    }
}

#[test]
fn every_terminator_has_explicit_byte_ownership() {
    let prefix = b"\x1bPq#1;2;100;0;0~!12";
    let expected = SixelImage::decode(prefix).unwrap();
    for byte in [0x18, 0x1a, 0x1b].into_iter().chain(0x80..=0x9f) {
        let input = [prefix.as_slice(), &[byte], b"[31mTAIL"].concat();
        let (expected_consumed, expected_status) = match byte {
            0x18 | 0x1a => (prefix.len() + 1, SixelDcsFeedStatus::Cancelled(byte)),
            0x9c => (prefix.len() + 1, SixelDcsFeedStatus::Complete),
            0x1b => (prefix.len() + 1, SixelDcsFeedStatus::Interrupted(byte)),
            _ => (prefix.len(), SixelDcsFeedStatus::Interrupted(byte)),
        };
        for split in 0..=input.len() {
            let (actual, consumed, status) = decode_chunks(&mut SixelDecoder::new(), [&input[..split], &[], &input[split..]]).unwrap();
            assert_image(&actual, &expected);
            assert_eq!(consumed, expected_consumed);
            assert_eq!(status, expected_status);
        }
    }
}

#[test]
fn trailing_escape_waits_for_lookahead_without_consuming_next_command() {
    for tail in [b"[31m".as_slice(), b"Pq~\x1b\\", b"\x1bPq~\x1b\\", b"\x9c", b"\x18", b"\n\\"] {
        let mut decoder = SixelDecoder::new();
        let mut stream = decoder.begin_dcs();
        let first = b"\x1bPq~\x1b";
        let progress = stream.feed(first).unwrap();
        assert_eq!(progress.consumed, first.len());
        assert_eq!(progress.status, SixelDcsFeedStatus::NeedMoreData);
        assert_eq!(stream.feed(b"").unwrap().status, SixelDcsFeedStatus::NeedMoreData);
        let progress = stream.feed(tail).unwrap();
        assert_eq!(progress.consumed, 0);
        assert_eq!(progress.status, SixelDcsFeedStatus::Interrupted(0x1b));
        assert_eq!(stream.finish().unwrap().dimensions(), (1, 6));
    }
}

#[test]
fn truncated_input_is_rejected_without_palette_commit() {
    let complete = b"\x1bP3;1q#42;2;100;0;0~\x1b\\";
    for end in 0..complete.len() {
        let mut decoder = SixelDecoder::new();
        let expected = decoder.decode_from_dcs(b"#42~", DcsSettings::default()).unwrap();
        let mut stream = decoder.begin_dcs();
        let progress = stream.feed(&complete[..end]).unwrap();
        assert_eq!(progress.consumed, end);
        assert_eq!(progress.status, SixelDcsFeedStatus::NeedMoreData);
        assert!(stream.finish().unwrap_err().to_string().contains("incomplete"));
        assert_image(&decoder.decode_from_dcs(b"#42~", DcsSettings::default()).unwrap(), &expected);
    }
}

#[test]
fn invalid_headers_and_payloads_poison_the_session() {
    for input in [
        b"text\x1bPq~\x1b\\".as_slice(),
        b"\x1b[31m",
        b"\x1bPp~\x1b\\",
        b"\x90?q~\x9c",
        b"\x90 q~\x9c",
        b"\x90:1q~\x9c",
        b"\x1bP1\x18q~\x1b\\",
        b"\x1bP1\x1aq~\x1b\\",
        b"\x1bP1\x1bq~\x1b\\",
        b"\x1bP1\x9cq~\x1b\\",
        b"\x1bPq#42;2;100;0;0~!65536~\x1b\\",
        b"\x1bPq#42;2;100;0;0~\"1;1;1000001;6\x9c",
    ] {
        for size in 1..=input.len() {
            let mut decoder = SixelDecoder::new();
            let expected = decoder.decode_from_dcs(b"#42~", DcsSettings::default()).unwrap();
            let mut stream = decoder.begin_dcs();
            assert!(input.chunks(size).any(|chunk| stream.feed(chunk).is_err()), "input: {input:?}");
            assert!(stream.feed(b"").is_err());
            assert!(stream.feed(b"\x1bPq~\x1b\\").is_err());
            assert!(stream.finish().is_err());
            assert_image(&decoder.decode_from_dcs(b"#42~", DcsSettings::default()).unwrap(), &expected);
        }
    }
}

#[test]
fn abort_and_drop_discard_even_completed_or_cancelled_palettes() {
    for suffix in [b"".as_slice(), b"\x1b", b"\x1b\\", b"\x9c", b"\x18", b"\x1a", b"\x1b[", b"\x9b"] {
        for abort in [false, true] {
            let mut decoder = SixelDecoder::new();
            let expected = decoder.decode_from_dcs(b"#42~", DcsSettings::default()).unwrap();
            let mut stream = decoder.begin_dcs();
            let _ = stream.feed(b"\x1bPq#42;2;100;0;0~").unwrap();
            let _ = stream.feed(suffix).unwrap();
            if abort {
                stream.abort();
            } else {
                drop(stream);
            }
            assert_image(&decoder.decode_from_dcs(b"#42~", DcsSettings::default()).unwrap(), &expected);
        }
    }
}

#[test]
fn consecutive_frames_share_palettes_but_not_frame_settings() {
    let input = b"\x1bP3;1q#42;2;100;0;0~\x1b\\\x90q#42~\x9c\x1bPq#42;2;0;100;0~\x18\x1bPq#42~\x1b\\tail";
    let mut decoder = SixelDecoder::new();
    let mut reference = SixelDecoder::new();
    let mut offset = 0;
    for _ in 0..4 {
        let expected = reference.decode(&input[offset..]).unwrap();
        let (actual, consumed, _) = decode_chunks(&mut decoder, input[offset..].chunks(1)).unwrap();
        assert_image(&actual, &expected);
        offset += consumed;
    }
    assert_eq!(&input[offset..], b"tail");
}

#[test]
fn encoded_image_survives_bytewise_and_random_chunks() {
    let pixels = (0..32 * 18)
        .flat_map(|i| [(i % 256) as u8, 80, (i / 3 % 256) as u8, if i % 5 == 0 { 0 } else { 255 }])
        .collect();
    let encoded = SixelImage::try_from_rgba(pixels, 32, 18).unwrap().encode().unwrap();
    let expected = SixelImage::decode(encoded.as_bytes()).unwrap();
    assert_image(&decode_chunks(&mut SixelDecoder::new(), encoded.as_bytes().chunks(1)).unwrap().0, &expected);
    for seed in 1..=16u32 {
        let mut random = seed;
        let mut chunks = Vec::new();
        let mut offset = 0;
        while offset < encoded.len() {
            random = random.wrapping_mul(1664525).wrapping_add(1013904223);
            let end = (offset + 1 + random as usize % 97).min(encoded.len());
            chunks.push(&encoded.as_bytes()[offset..end]);
            chunks.push(&[]);
            offset = end;
        }
        let (actual, consumed, status) = decode_chunks(&mut SixelDecoder::new(), chunks).unwrap();
        assert_image(&actual, &expected);
        assert_eq!(consumed, encoded.len());
        assert_eq!(status, SixelDcsFeedStatus::Complete);
    }
}
