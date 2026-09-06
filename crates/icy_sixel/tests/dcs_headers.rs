use icy_sixel::{DcsSettings, SixelDecoder, SixelImage};

#[test]
fn incomplete_or_non_sixel_dcs_headers_are_rejected() {
    for introducer in [b"\x1bP".as_slice(), b"\x90"] {
        for header in [b"".as_slice(), b"123", b"0;1;", b"xyzq~", b"$q~", b"\x1b\\", b"\x9c"] {
            let bytes = [introducer, header].concat();
            assert!(SixelImage::decode(&bytes).is_err(), "accepted {bytes:?}");
        }
    }
}

#[test]
fn complete_headers_and_raw_payloads_remain_supported() {
    let expected = SixelImage::decode_from_dcs(b"#1~", DcsSettings::default()).unwrap();
    for bytes in [
        b"\x1bPq#1~\x1b\\".as_slice(),
        b"\x90q#1~\x9c",
        b"prefix\x1bP9;0;0q#1~\x1b\\suffix",
        b"\x1bP\n9;0;0\tq#1~\x1b\\",
        b"#1~",
        b"\x1bPq#1~", // Missing terminators remain accepted for compatibility.
    ] {
        let image = SixelImage::decode(bytes).unwrap();
        assert_eq!(image.dimensions(), expected.dimensions());
        assert_eq!(image.pixels, expected.pixels);
    }
    assert!(SixelImage::decode(b"\x1bPq").is_ok());
}

#[test]
fn rejected_header_does_not_modify_shared_palette() {
    let mut decoder = SixelDecoder::new();
    decoder.decode(b"\x1bPq#42;2;100;0;0~\x1b\\").unwrap();
    assert!(decoder.decode(b"\x1bPbadq#42;2;0;100;0~\x1b\\").is_err());
    let image = decoder.decode_from_dcs(b"#42~", DcsSettings::default()).unwrap();
    assert_eq!(&image.pixels[..4], &[255, 0, 0, 255]);
}
