use icy_sixel::{DcsSettings, SixelDecoder, SixelImage};

fn assert_same_image(actual: &SixelImage, expected: &SixelImage) {
    assert_eq!(actual.dimensions(), expected.dimensions());
    assert_eq!(actual.pixels, expected.pixels);
    assert_eq!(actual.aspect_ratio, expected.aspect_ratio);
    assert_eq!(actual.background_mode, expected.background_mode);
}

#[test]
fn ignored_controls_preserve_parameter_state() {
    for command in [b"!12".as_slice(), b"#1;2;100;0;0", b"#1;1;120;50;100", b"\"3;1;12;6", b"#;2;100;;"] {
        let expected = SixelImage::decode_from_dcs(&[command, b"~"].concat(), DcsSettings::default()).unwrap();
        for control in (0x00..=0x17).chain([0x19]).chain(0x1c..=0x1f).chain([0x7f]) {
            for offset in 1..=command.len() {
                let payload = [&command[..offset], &[control], &command[offset..], b"~"].concat();
                let actual = SixelImage::decode_from_dcs(&payload, DcsSettings::default()).unwrap();
                assert_same_image(&actual, &expected);
                let wrapped = [b"\x1bPq".as_slice(), &payload, b"\x1b\\"].concat();
                assert_same_image(&SixelImage::decode(&wrapped).unwrap(), &expected);
            }
        }
    }
}

#[test]
fn ignored_controls_preserve_header_parameters() {
    let expected = SixelImage::decode(b"\x1bP3;1q~\x1b\\").unwrap();
    for control in (0x00..=0x17).chain([0x19]).chain(0x1c..=0x1f).chain([0x7f]) {
        let data = [b"\x1bP3;".as_slice(), &[control], b"1q~\x1b\\"].concat();
        assert_same_image(&SixelImage::decode(&data).unwrap(), &expected);
    }
}

#[test]
fn controls_do_not_bypass_numeric_limits() {
    for payload in [b"!655\n36~".as_slice(), b"\"1;1;1000\r001;1", b"!99999999999999999999\t999999999999999999999~"] {
        assert!(SixelImage::decode_from_dcs(payload, DcsSettings::default()).is_err());
        assert!(SixelImage::decode(&[b"\x1bPq".as_slice(), payload].concat()).is_err());
    }
}

#[test]
fn c1_controls_match_seven_bit_cancellation() {
    for c1 in 0x80..=0x9f {
        let expected = SixelImage::decode_from_dcs(b"#1~", DcsSettings::default()).unwrap();
        for control in [vec![c1], vec![0x1b, c1 - 0x40]] {
            let payload = [b"#1~".as_slice(), &control, b"0m~~#1;2;100;0;0~"].concat();
            let actual = SixelImage::decode_from_dcs(&payload, DcsSettings::default()).unwrap();
            assert_same_image(&actual, &expected);
            for prefix in [b"\x1bPq".as_slice(), b"\x90q"] {
                let actual = SixelImage::decode(&[prefix, &payload, b"\x1b\\"].concat()).unwrap();
                assert_same_image(&actual, &expected);
            }
        }
    }
}

#[test]
fn cancellation_still_stops_inside_parameters() {
    for cancel in [0x18, 0x1a, 0x1b].into_iter().chain(0x80..=0x9f) {
        for prefix in [b"@!1\n2".as_slice(), b"@#1;2;100;\r0;0", b"@\"1;\t1;12;6"] {
            let expected = SixelImage::decode_from_dcs(prefix, DcsSettings::default()).unwrap();
            let payload = [prefix, &[cancel], b"!999999999~"].concat();
            let actual = SixelImage::decode_from_dcs(&payload, DcsSettings::default()).unwrap();
            assert_same_image(&actual, &expected);
            let actual = SixelImage::decode(&[b"\x1bPq".as_slice(), &payload].concat()).unwrap();
            assert_same_image(&actual, &expected);
        }
    }
}

#[test]
fn c1_cancellation_preserves_only_preceding_palette_changes() {
    for c1 in 0x80..=0x9f {
        for wrapped in [false, true] {
            let mut decoder = SixelDecoder::new();
            let payload = [b"#1;2;100;0;0@".as_slice(), &[c1], b"#1;2;0;100;0~"].concat();
            if wrapped {
                decoder.decode(&[b"\x1bPq".as_slice(), &payload].concat()).unwrap();
            } else {
                decoder.decode_from_dcs(&payload, DcsSettings::default()).unwrap();
            }
            let next = decoder.decode_from_dcs(b"#1@", DcsSettings::default()).unwrap();
            assert_eq!(&next.pixels[..4], &[255, 0, 0, 255]);
        }
    }
}
