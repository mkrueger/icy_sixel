use icy_sixel::{EncodeOptions, QuantizeMethod, SixelError, SixelImage};

#[test]
fn long_runs_roundtrip_across_repeat_limit() {
    for width in [65_535, 65_536, 131_073] {
        let pixels = [255, 0, 0, 255].repeat(width);
        let image = SixelImage::try_from_rgba(pixels.clone(), width, 1).unwrap();
        let encoded = image.encode().unwrap();
        let decoded = SixelImage::decode(encoded.as_bytes()).unwrap();

        assert_eq!(decoded.width, width);
        assert_eq!(&decoded.pixels[..pixels.len()], pixels.as_slice());
        for run in encoded.split('!').skip(1) {
            let count: usize = run.chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap();
            assert!(count <= 65_535);
        }
    }
}

#[test]
fn overflowing_dimensions_return_errors() {
    // Exercise both multiplication steps, even through the unchecked constructor.
    for (width, height) in [(usize::MAX, 2), (usize::MAX / 4 + 1, 1)] {
        let image = SixelImage::from_rgba(Vec::new(), width, height);
        assert!(matches!(image.encode(), Err(SixelError::IntegerOverflow)));
        assert!(matches!(image.encode_with(&EncodeOptions::default()), Err(SixelError::IntegerOverflow)));
        assert!(matches!(
            icy_sixel::sixel_encode(&[], width, height, &EncodeOptions::default()),
            Err(SixelError::IntegerOverflow)
        ));
    }
}

#[test]
fn transparent_pixels_do_not_consume_palette_colors() {
    for quantize_method in [QuantizeMethod::Wu, QuantizeMethod::kmeans()] {
        for diffusion in [0.0, 0.875, 1.0] {
            let mut pixels = [0, 255, 0, 127].repeat(1000);
            pixels.extend_from_slice(&[255, 0, 0, 128]);
            pixels.extend_from_slice(&[0, 0, 255, 255]);
            let image = SixelImage::try_from_rgba(pixels, 1002, 1).unwrap();
            let options = EncodeOptions {
                max_colors: 2,
                diffusion,
                quantize_method: quantize_method.clone(),
            };
            let encoded = image.encode_with(&options).unwrap();
            let decoded = SixelImage::decode(encoded.as_bytes()).unwrap();

            assert!(decoded.pixels[..4000].chunks_exact(4).all(|pixel| pixel[3] == 0));
            assert_eq!(&decoded.pixels[4000..4004], &[255, 0, 0, 255]);
            assert_eq!(&decoded.pixels[4004..4008], &[0, 0, 255, 255]);
        }
    }
}

#[test]
fn hidden_rgb_does_not_change_encoded_output() {
    let (width, height) = (17, 12);
    let mut first = Vec::new();
    let mut second = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if (x + y) % 3 == 0 {
                first.extend_from_slice(&[0, 255, 0, 0]);
                second.extend_from_slice(&[255, 0, 255, 0]);
            } else {
                let pixel = [(x * 15) as u8, (y * 20) as u8, ((x + y) * 9) as u8, 255];
                first.extend_from_slice(&pixel);
                second.extend_from_slice(&pixel);
            }
        }
    }
    let first = SixelImage::try_from_rgba(first, width, height).unwrap();
    let second = SixelImage::try_from_rgba(second, width, height).unwrap();
    for diffusion in [0.0, 0.5, 1.0] {
        let options = EncodeOptions {
            max_colors: 4,
            diffusion,
            ..Default::default()
        };
        assert_eq!(first.encode_with(&options).unwrap(), second.encode_with(&options).unwrap());
    }
}

#[test]
fn fully_transparent_images_roundtrip() {
    for (width, height) in [(1, 1), (65, 7)] {
        let image = SixelImage::try_from_rgba([255, 0, 255, 0].repeat(width * height), width, height).unwrap();
        let encoded = image.encode().unwrap();
        let decoded = SixelImage::decode(encoded.as_bytes()).unwrap();
        assert_eq!(decoded.dimensions(), (width, height));
        assert!(decoded.pixels.chunks_exact(4).all(|pixel| pixel[3] == 0));
    }
}
