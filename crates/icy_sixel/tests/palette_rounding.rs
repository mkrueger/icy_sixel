use icy_sixel::{EncodeOptions, SixelImage};

#[test]
fn solid_colors_round_to_nearest_sixel_percentage() {
    for color in [[0, 0, 0], [255, 255, 255], [127, 127, 127], [1, 2, 3], [64, 128, 192], [254, 253, 252]] {
        let image = SixelImage::try_from_rgba(vec![color[0], color[1], color[2], 255], 1, 1).unwrap();
        let encoded = image
            .encode_with(&EncodeOptions {
                diffusion: 0.0,
                ..Default::default()
            })
            .unwrap();
        let decoded = SixelImage::decode(encoded.as_bytes()).unwrap();
        for (channel, value) in color.into_iter().enumerate() {
            let percentage = (u32::from(value) * 100 + 127) / 255;
            let expected = ((percentage * 255 + 50) / 100) as u8;
            assert_eq!(decoded.pixels[channel], expected, "color={color:?}, channel={channel}");
        }
    }
}

#[test]
fn mid_gray_roundtrip_does_not_have_a_downward_bias() {
    let image = SixelImage::try_from_rgba(vec![127, 127, 127, 255], 1, 1).unwrap();
    let encoded = image.encode().unwrap();
    let decoded = SixelImage::decode(encoded.as_bytes()).unwrap();
    assert_eq!(&decoded.pixels[..4], &[128, 128, 128, 255]);
    let repeated = SixelImage::decode(decoded.encode().unwrap().as_bytes()).unwrap();
    assert_eq!(repeated.pixels, decoded.pixels);
}
