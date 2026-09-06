use icy_sixel::{DcsSettings, SixelDecoder, SixelError, SixelImage};

fn assert_same_image(actual: &SixelImage, expected: &SixelImage) {
    assert_eq!(actual.dimensions(), expected.dimensions());
    assert_eq!(actual.pixels, expected.pixels);
    assert_eq!(actual.aspect_ratio, expected.aspect_ratio);
    assert_eq!(actual.background_mode, expected.background_mode);
}

#[test]
fn cancellation_returns_partial_image_in_all_entry_points() {
    for cancel in [0x18, 0x1a] {
        let payload = [b"#1;2;100;0;0@".as_slice(), &[cancel], b"~~#1;2;0;100;0~"].concat();
        for prefix in [b"".as_slice(), b"\x1bPq", b"\x90q"] {
            let expected = SixelImage::decode(&[prefix, b"#1;2;100;0;0@\x1b\\"].concat()).unwrap();
            for suffix in [b"".as_slice(), b"\x1b\\", b"\x9c"] {
                let actual = SixelImage::decode(&[prefix, &payload, suffix].concat()).unwrap();
                assert_same_image(&actual, &expected);
            }
        }
        let expected = SixelImage::decode_from_dcs(b"#1;2;100;0;0@", DcsSettings::default()).unwrap();
        let actual = SixelImage::decode_from_dcs(&payload, DcsSettings::default()).unwrap();
        assert_same_image(&actual, &expected);
    }
}

#[test]
fn cancellation_ignores_following_commands_and_data() {
    for cancel in [0x18, 0x1a] {
        for prefix in [b"".as_slice(), b"@!3", b"@#1", b"@\"1;1"] {
            let expected = SixelImage::decode_from_dcs(prefix, DcsSettings::default()).unwrap();
            let payload = [prefix, &[cancel], b"!999999999~\"1;1;999999999;999999999"].concat();
            let actual = SixelImage::decode_from_dcs(&payload, DcsSettings::default()).unwrap();
            assert_same_image(&actual, &expected);
            let actual = SixelImage::decode(&[b"\x1bPq".as_slice(), &payload].concat()).unwrap();
            assert_same_image(&actual, &expected);
        }
    }
}

#[test]
fn cancellation_preserves_only_preceding_palette_changes() {
    for cancel in [0x18, 0x1a] {
        for wrapped in [false, true] {
            let mut decoder = SixelDecoder::new();
            let payload = [b"#1;2;100;0;0@".as_slice(), &[cancel], b"#1;2;0;100;0~~"].concat();
            if wrapped {
                decoder.decode(&[b"\x1bPq".as_slice(), &payload, b"\x1b\\"].concat()).unwrap();
            } else {
                decoder.decode_from_dcs(&payload, DcsSettings::default()).unwrap();
            }
            let next = decoder.decode(b"\x1bPq#1@\x1b\\").unwrap();
            assert_eq!(&next.pixels[..4], &[255, 0, 0, 255]);
            assert_eq!(next.dimensions(), (1, 6));
        }
    }
}

#[test]
fn cancellation_before_sixel_command_still_rejects_header() {
    for cancel in [0x18, 0x1a] {
        for prefix in [b"\x1bP9;1".as_slice(), b"\x90"] {
            let mut decoder = SixelDecoder::new();
            decoder.decode(b"\x1bPq#1;2;100;0;0@\x1b\\").unwrap();
            let data = [prefix, &[cancel], b"q#1;2;0;100;0@\x1b\\"].concat();
            assert!(matches!(decoder.decode(&data), Err(SixelError::InvalidData(_))));
            let next = decoder.decode(b"\x1bPq#1@\x1b\\").unwrap();
            assert_eq!(&next.pixels[..4], &[255, 0, 0, 255]);
        }
    }
}
