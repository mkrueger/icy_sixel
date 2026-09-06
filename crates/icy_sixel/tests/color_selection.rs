use icy_sixel::{DcsSettings, SixelDecoder, SixelImage};

#[test]
fn omitted_color_index_matches_explicit_zero() {
    for prefix in ["", "#0;2;100;0;0", "#0;1;120;50;100"] {
        let implicit = format!("{prefix}#1~#~");
        let explicit = format!("{prefix}#1~#0~");
        let implicit = SixelImage::decode_from_dcs(implicit.as_bytes(), DcsSettings::default()).unwrap();
        let explicit = SixelImage::decode_from_dcs(explicit.as_bytes(), DcsSettings::default()).unwrap();
        assert_eq!(implicit.dimensions(), (2, 6));
        assert_eq!(implicit.pixels, explicit.pixels);
    }
}

#[test]
fn omitted_color_index_uses_persistent_register_zero() {
    let mut decoder = SixelDecoder::new();
    decoder.decode_from_dcs(b"#0;2;100;0;0~", DcsSettings::default()).unwrap();
    let image = decoder.decode_from_dcs(b"#1~#~", DcsSettings::default()).unwrap();
    assert_eq!(&image.pixels[4..8], &[255, 0, 0, 255]);
    decoder.reset_palette();
    let image = decoder.decode_from_dcs(b"#1~#~", DcsSettings::default()).unwrap();
    assert_eq!(&image.pixels[4..8], &[0, 0, 0, 255]);
}
