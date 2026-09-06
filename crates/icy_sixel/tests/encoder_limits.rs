use icy_sixel::{EncodeOptions, SixelError, SixelImage};

#[test]
fn unsupported_raster_sizes_are_rejected_before_encoding() {
    for (width, height) in [(1_000_001, 1), (1, 1_000_000)] {
        for alpha in [0, 255] {
            let pixels = [0, 0, 0, alpha].repeat(width * height);
            let image = SixelImage::try_from_rgba(pixels, width, height).unwrap();
            assert!(matches!(image.encode(), Err(SixelError::InvalidDimensions { .. })));
            assert!(matches!(
                image.encode_with(&EncodeOptions::default()),
                Err(SixelError::InvalidDimensions { .. })
            ));
        }
    }
}

#[test]
fn decoder_area_limit_is_checked_before_buffer_allocation() {
    // No enormous input allocation needed: dimensions themselves are unsupported.
    assert!(matches!(
        icy_sixel::sixel_encode(&[], 8192, 8192, &EncodeOptions::default()),
        Err(SixelError::InvalidDimensions { .. })
    ));
}

#[test]
fn supported_height_boundary_roundtrips() {
    // Sparse, tall image exercises the maximum complete band without large RLEs.
    let height = 999_996;
    let mut pixels = vec![0; height * 4];
    pixels[height * 4 - 4..].copy_from_slice(&[255, 0, 0, 255]);
    let image = SixelImage::try_from_rgba(pixels, 1, height).unwrap();
    let encoded = image.encode().unwrap();
    let decoded = SixelImage::decode(encoded.as_bytes()).unwrap();
    assert_eq!(decoded.dimensions(), (1, height));
    assert_eq!(&decoded.pixels[height * 4 - 4..], &[255, 0, 0, 255]);
}
