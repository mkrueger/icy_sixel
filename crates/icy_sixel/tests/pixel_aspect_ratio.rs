use icy_sixel::{DcsSettings, PixelAspectRatio, SixelImage};

#[test]
fn p1_values_follow_dec_macro_table() {
    // Expectations come from the DEC table, not from the inverse conversion.
    for (p1, ratio, scale) in [
        (0, PixelAspectRatio::Ratio2To1, 2),
        (1, PixelAspectRatio::Ratio2To1, 2),
        (2, PixelAspectRatio::Ratio5To1, 5),
        (3, PixelAspectRatio::Ratio3To1, 3),
        (4, PixelAspectRatio::Ratio3To1, 3),
        (5, PixelAspectRatio::Ratio2To1, 2),
        (6, PixelAspectRatio::Ratio2To1, 2),
        (7, PixelAspectRatio::Square, 1),
        (8, PixelAspectRatio::Square, 1),
        (9, PixelAspectRatio::Square, 1),
    ] {
        assert_eq!(PixelAspectRatio::from_p1(p1), ratio);
        for introducer in [b"\x1bP".as_slice(), b"\x90"] {
            let data = [introducer, format!("{p1};1q@\x1b\\").as_bytes()].concat();
            let image = SixelImage::decode(&data).unwrap();
            assert_eq!(image.aspect_ratio, ratio, "P1={p1}");
            assert_eq!(image.corrected_dimensions(), (1, 6 * scale));
        }
        let image = SixelImage::decode_from_dcs(b"@", DcsSettings::new(Some(p1), Some(1), None)).unwrap();
        assert_eq!(image.aspect_ratio, ratio);
    }
}

#[test]
fn encoded_p1_matches_raster_attributes() {
    for (ratio, p1, vertical) in [
        (PixelAspectRatio::Ratio2To1, 0, 2),
        (PixelAspectRatio::Ratio3To1, 3, 3),
        (PixelAspectRatio::Ratio5To1, 2, 5),
        (PixelAspectRatio::Square, 9, 1),
    ] {
        assert_eq!(ratio.to_p1_value(), p1);
        assert_eq!(PixelAspectRatio::from_p1(u16::from(p1)), ratio);
        let image = SixelImage::try_from_rgba(vec![255, 0, 0, 255], 1, 1).unwrap().with_aspect_ratio(ratio);
        let encoded = image.encode().unwrap();
        assert!(encoded.starts_with(&format!("\x1bP{p1};1;0q\"{vertical};1;1;1")));
        assert_eq!(SixelImage::decode(encoded.as_bytes()).unwrap().aspect_ratio, ratio);

        // Remove raster attributes so they cannot mask a wrong P1 parameter.
        let header_end = encoded.find('q').unwrap() + 1;
        let palette_start = encoded.find('#').unwrap();
        let without_raster = format!("{}{}", &encoded[..header_end], &encoded[palette_start..]);
        assert_eq!(SixelImage::decode(without_raster.as_bytes()).unwrap().aspect_ratio, ratio);

        let settings = DcsSettings::default().with_pixel_aspect_ratio(ratio);
        assert_eq!(SixelImage::decode_from_dcs(b"@", settings).unwrap().aspect_ratio, ratio);
    }
}

#[test]
fn explicit_raster_overrides_corrected_p1_mapping() {
    // P1=2 is 5:1, but the explicit raster requests 3:1.
    let image = SixelImage::decode(b"\x1bP2;1q\"3;1;1;6@\x1b\\").unwrap();
    assert_eq!(image.aspect_ratio, PixelAspectRatio::Ratio3To1);
    assert_eq!(image.corrected_dimensions(), (1, 18));
    // An unsupported raster ratio falls back to the corrected P1 mapping.
    let image = SixelImage::decode(b"\x1bP2;1q\"4;1;1;6@\x1b\\").unwrap();
    assert_eq!(image.aspect_ratio, PixelAspectRatio::Ratio5To1);
}

#[test]
fn absent_and_unknown_p1_keep_square_fallback() {
    // Preserve the library's documented fallback, distinct from explicit P1=0.
    for data in [b"@".as_slice(), b"\x1bPq@\x1b\\", b"\x1bP10q@\x1b\\", b"\x1bP65535q@\x1b\\"] {
        assert_eq!(SixelImage::decode(data).unwrap().aspect_ratio, PixelAspectRatio::Square);
    }
    for p1 in [10, u16::MAX] {
        assert_eq!(PixelAspectRatio::from_p1(p1), PixelAspectRatio::Square);
    }
}
