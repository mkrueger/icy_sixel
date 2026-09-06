use icy_sixel::{BackgroundMode, DcsSettings, PixelAspectRatio, SixelDecoder, SixelFeedStatus, SixelImage};

#[test]
fn normal_runs_preserve_all_masks_and_consume_repeat_once() {
    let settings = DcsSettings::default().with_background_mode(BackgroundMode::Transparent);
    let mut input = b"#1;2;100;0;0!3".to_vec();
    input.extend(b'?'..=b'~');
    input.extend_from_slice(b"\x1b\\ignored");
    let mut pixels = vec![0; 66 * 6 * 4];
    for mask in 1..64usize {
        for y in 0..6 {
            if mask & (1 << y) != 0 {
                let offset = (y * 66 + mask + 2) * 4;
                pixels[offset..offset + 4].copy_from_slice(&[255, 0, 0, 255]);
            }
        }
    }
    for size in 1..=input.len() {
        let image = incremental(input.chunks(size), settings).unwrap();
        assert_eq!(image.dimensions(), (66, 6));
        assert_eq!(image.pixels, pixels, "chunk size {size}");
    }
}

fn assert_image(actual: &SixelImage, expected: &SixelImage) {
    assert_eq!(actual.dimensions(), expected.dimensions());
    assert_eq!(actual.pixels, expected.pixels);
    assert_eq!(actual.aspect_ratio, expected.aspect_ratio);
    assert_eq!(actual.background_mode, expected.background_mode);
}

fn incremental<'a>(chunks: impl IntoIterator<Item = &'a [u8]>, settings: DcsSettings) -> icy_sixel::Result<SixelImage> {
    let mut decoder = SixelDecoder::new();
    let mut stream = decoder.begin_frame(settings)?;
    for chunk in chunks {
        let result = stream.feed(chunk)?;
        match result.status {
            SixelFeedStatus::NeedMoreData => assert_eq!(result.consumed, chunk.len()),
            SixelFeedStatus::Terminated(byte) => {
                assert_eq!(chunk[result.consumed], byte);
                break;
            }
        }
    }
    stream.finish()
}

#[test]
fn every_split_matches_one_shot_including_errors() {
    for payload in [
        b"".as_slice(),
        b"!",
        b"!0",
        b"!12",
        b"#",
        b"#1;2;100;;",
        b"\"3;1;12;6",
        b"#1;2;100;0;0!1\n2~$#2;1;120;50;100??@-!0~",
        b"\"6;2;16;12#1;2;100;0;0!12~-#;2;;100;~",
        b"#1;2;100;0;0;123;456~\"1;1;12;6;99~",
        b"#99999999999999999999;1;2147483647999;50;100~",
        b"#1;2;100;0;0@\x18#1;2;0;100;0!99999999~",
        b"@\x9b0m~~",
        b"@\x1b\\trailing text",
        b"\"4;1;1;6~",
        b"!65536",
        b"!655\n36~",
        b"!999999999999999999999999999999~",
        b"\"1;1;1000001;6",
        b"\"1;1;8192;8193",
        b"#1;2;100;0;0~!999999999~",
    ] {
        for settings in [
            DcsSettings::default(),
            DcsSettings::default()
                .with_background_mode(BackgroundMode::Transparent)
                .with_pixel_aspect_ratio(PixelAspectRatio::Ratio5To1),
        ] {
            let expected = SixelImage::decode_from_dcs(payload, settings);
            for split in 0..=payload.len() {
                let actual = incremental([&payload[..split], &[], &payload[split..]], settings);
                match (&expected, actual) {
                    (Ok(expected), Ok(actual)) => assert_image(&actual, expected),
                    (Err(expected), Err(actual)) => assert_eq!(actual.to_string(), expected.to_string()),
                    (expected, actual) => panic!("split {split}, payload {payload:?}: expected {expected:?}, got {actual:?}"),
                }
            }
        }
    }
}

#[test]
fn bytewise_and_random_chunks_preserve_encoded_image() {
    let pixels: Vec<_> = (0..64 * 24)
        .flat_map(|i| [(i % 256) as u8, (i / 7 % 256) as u8, 80, if i % 5 == 0 { 0 } else { 255 }])
        .collect();
    let encoded = SixelImage::try_from_rgba(pixels, 64, 24).unwrap().encode().unwrap();
    let payload = &encoded.as_bytes()[encoded.find('q').unwrap() + 1..];
    let settings = DcsSettings::default().with_background_mode(BackgroundMode::Transparent);
    let expected = SixelImage::decode(encoded.as_bytes()).unwrap();
    assert_image(&incremental(payload.chunks(1), settings).unwrap(), &expected);
    for seed in 1..=16u32 {
        let mut state = seed;
        let mut chunks = Vec::new();
        let mut offset = 0;
        while offset < payload.len() {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let end = (offset + 1 + state as usize % 97).min(payload.len());
            chunks.push(&payload[offset..end]);
            chunks.push(&[]);
            offset = end;
        }
        assert_image(&incremental(chunks, settings).unwrap(), &expected);
    }
}

#[test]
fn chunk_boundaries_do_not_finalize_commands() {
    let mut decoder = SixelDecoder::new();
    let mut frame = decoder.begin_frame(DcsSettings::default()).unwrap();
    for chunk in [b"#1;2;1".as_slice(), b"00;0;", b"", b"0!1", b"", b"2~"] {
        let result = frame.feed(chunk).unwrap();
        assert_eq!(result.status, SixelFeedStatus::NeedMoreData);
        assert_eq!(result.consumed, chunk.len());
    }
    let image = frame.finish().unwrap();
    assert_eq!(image.dimensions(), (12, 6));
    assert!(image.pixels.as_chunks::<4>().0.iter().all(|p| *p == [255, 0, 0, 255]));
}

#[test]
fn all_terminators_remain_unconsumed_and_stop_future_feeds() {
    for byte in [0x18, 0x1a, 0x1b].into_iter().chain(0x80..=0x9f) {
        let mut decoder = SixelDecoder::new();
        let mut frame = decoder.begin_frame(DcsSettings::default()).unwrap();
        assert_eq!(frame.feed(b"#1;2;100;0;0@!").unwrap().status, SixelFeedStatus::NeedMoreData);
        let tail = [b"12".as_slice(), &[byte], b"!999999999~"].concat();
        let result = frame.feed(&tail).unwrap();
        assert_eq!(result.consumed, 2);
        assert_eq!(result.status, SixelFeedStatus::Terminated(byte));
        for input in [b"".as_slice(), b"\\text", b"#1;2;0;100;0~"] {
            let repeated = frame.feed(input).unwrap();
            assert_eq!(repeated.consumed, 0);
            assert_eq!(repeated.status, result.status);
        }
        let image = frame.finish().unwrap();
        assert_eq!(image.dimensions(), (1, 6));
        assert_eq!(&image.pixels[..4], &[255, 0, 0, 255]);
    }
}

#[test]
fn split_st_and_following_dcs_belong_to_outer_parser() {
    let mut decoder = SixelDecoder::new();
    let mut frame = decoder.begin_frame(DcsSettings::default()).unwrap();
    let chunk = b"#1;2;100;0;0~\x1b";
    let result = frame.feed(chunk).unwrap();
    assert_eq!(&chunk[result.consumed..], b"\x1b");
    assert_eq!(result.status, SixelFeedStatus::Terminated(0x1b));
    assert_eq!(frame.finish().unwrap().dimensions(), (1, 6));
    let next = b"\\text\x1bP9;1q#1~~\x1b\\suffix";
    let payload_start = next.iter().position(|&b| b == b'q').unwrap() + 1;
    let mut frame = decoder.begin_frame(DcsSettings::new(Some(9), Some(1), None)).unwrap();
    let result = frame.feed(&next[payload_start..]).unwrap();
    assert_eq!(&next[payload_start + result.consumed..], b"\x1b\\suffix");
    let image = frame.finish().unwrap();
    assert_eq!(image.dimensions(), (2, 6));
    assert_eq!(&image.pixels[..4], &[255, 0, 0, 255]);
}

#[test]
fn explicit_eof_flushes_pending_palette_and_raster() {
    let mut decoder = SixelDecoder::new();
    let mut frame = decoder.begin_frame(DcsSettings::default()).unwrap();
    let _ = frame.feed(b"\"3;1;12;6").unwrap();
    let image = frame.finish().unwrap();
    assert_eq!(image.dimensions(), (12, 6));
    assert_eq!(image.aspect_ratio, PixelAspectRatio::Ratio3To1);
    let mut frame = decoder.begin_frame(DcsSettings::default()).unwrap();
    let _ = frame.feed(b"#42;2;100;0;0").unwrap();
    frame.finish().unwrap();
    let next = decoder.decode_from_dcs(b"#42@", DcsSettings::default()).unwrap();
    assert_eq!(&next.pixels[..4], &[255, 0, 0, 255]);
}

#[test]
fn finalization_preserves_all_registers_and_packed_pixels() {
    let mut decoder = SixelDecoder::new();
    let settings = DcsSettings::default().with_background_mode(BackgroundMode::Transparent);
    let mut frame = decoder.begin_frame(settings).unwrap();
    let mut expected_row = Vec::new();
    let mut drawing = String::new();
    for index in 0..256 {
        let channels = [index % 101, (index * 3) % 101, (index * 7) % 101];
        let [r, g, b] = channels;
        assert_eq!(
            frame.feed(format!("#{index};2;{r};{g};{b}").as_bytes()).unwrap().status,
            SixelFeedStatus::NeedMoreData
        );
        expected_row.extend(channels.map(|value| ((value * 255 + 50) / 100) as u8));
        expected_row.push(255);
        drawing.push_str(&format!("#{index}~"));
    }
    // The last register is still pending until finish; no pixels were drawn.
    assert_eq!(frame.finish().unwrap().pixels, vec![0; 4]);

    // Exercise both direct buffer extraction and packing a padded canvas.
    let mut images = Vec::new();
    for extra_blank in [false, true] {
        let mut frame = decoder.begin_frame(settings).unwrap();
        assert_eq!(frame.feed(drawing.as_bytes()).unwrap().status, SixelFeedStatus::NeedMoreData);
        if extra_blank {
            assert_eq!(frame.feed(b"?").unwrap().status, SixelFeedStatus::NeedMoreData);
        }
        images.push(frame.finish().unwrap());
    }
    assert_eq!(images[0].dimensions(), (256, 6));
    assert_eq!(images[0].pixels, expected_row.repeat(6));
    expected_row.extend([0; 4]);
    assert_eq!(images[1].dimensions(), (257, 6));
    assert_eq!(images[1].pixels, expected_row.repeat(6));
}

#[test]
fn dropped_aborted_and_unfinished_terminated_frames_do_not_commit() {
    for mode in 0..3 {
        let mut decoder = SixelDecoder::new();
        let expected = decoder.decode_from_dcs(b"#42@", DcsSettings::default()).unwrap();
        let mut frame = decoder.begin_frame(DcsSettings::default()).unwrap();
        let _ = frame.feed(b"#42;2;100;0;0@").unwrap();
        match mode {
            0 => frame.abort(),
            1 => drop(frame),
            _ => {
                assert_eq!(frame.feed(b"\x18").unwrap().status, SixelFeedStatus::Terminated(0x18));
                drop(frame);
            }
        }
        assert_image(&decoder.decode_from_dcs(b"#42@", DcsSettings::default()).unwrap(), &expected);
    }
}

#[test]
fn feed_errors_poison_session_without_committing_palette() {
    let mut decoder = SixelDecoder::new();
    let expected = decoder.decode_from_dcs(b"#42@", DcsSettings::default()).unwrap();
    let mut frame = decoder.begin_frame(DcsSettings::default()).unwrap();
    let _ = frame.feed(b"#42;2;100;0;0@!655").unwrap();
    assert!(frame.feed(b"36~").is_err());
    assert!(frame.feed(b"").is_err());
    assert!(frame.feed(b"~").is_err());
    assert!(frame.finish().is_err());
    assert_image(&decoder.decode_from_dcs(b"#42@", DcsSettings::default()).unwrap(), &expected);
}

#[test]
fn errors_discovered_at_eof_do_not_commit() {
    for tail in [b"!65536".as_slice(), b"\"1;1;1000001;6"] {
        let mut decoder = SixelDecoder::new();
        let expected = decoder.decode_from_dcs(b"#42@", DcsSettings::default()).unwrap();
        let mut frame = decoder.begin_frame(DcsSettings::default()).unwrap();
        let _ = frame.feed(b"#42;2;100;0;0@").unwrap();
        let _ = frame.feed(tail).unwrap();
        assert!(frame.finish().is_err());
        assert_image(&decoder.decode_from_dcs(b"#42@", DcsSettings::default()).unwrap(), &expected);
    }
}

#[test]
fn sequential_frames_preserve_palette_and_reset_frame_state() {
    let mut reference = SixelDecoder::new();
    let mut decoder = SixelDecoder::new();
    for (payload, settings) in [
        (b"#0;2;100;0;0@#42;2;0;100;0~".as_slice(), DcsSettings::default()),
        (b"\"3;1;12;6#42@\x1b\\ignored", DcsSettings::default()),
        (b"?", DcsSettings::default()),
        (b"?", DcsSettings::default().with_background_mode(BackgroundMode::Transparent)),
        (b"#42;2;0;0;100@\x18#42;2;100;0;0", DcsSettings::default()),
        (b"#42@", DcsSettings::default()),
    ] {
        let expected = reference.decode_from_dcs(payload, settings).unwrap();
        let mut frame = decoder.begin_frame(settings).unwrap();
        for chunk in payload.chunks(1) {
            if matches!(frame.feed(chunk).unwrap().status, SixelFeedStatus::Terminated(_)) {
                break;
            }
        }
        assert_image(&frame.finish().unwrap(), &expected);
    }
}
