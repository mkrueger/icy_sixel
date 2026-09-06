use assert_cmd::Command;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{AnimationDecoder, Delay, Frame, RgbaImage};
use predicates::prelude::*;
use std::{fs::File, path::Path, time::Duration};

fn sixel_cmd() -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sixel"));
    cmd.timeout(Duration::from_secs(5));
    cmd
}

fn write_gif(path: &Path, repeat: Repeat) {
    let mut encoder = GifEncoder::new(File::create(path).unwrap());
    encoder.set_repeat(repeat).unwrap();
    for color in [[255, 0, 0, 255], [0, 0, 255, 255]] {
        let buffer = RgbaImage::from_raw(1, 1, color.to_vec()).unwrap();
        encoder
            .encode_frame(Frame::from_parts(buffer, 0, 0, Delay::from_numer_denom_ms(10, 1)))
            .unwrap();
    }
}

#[test]
fn animation_rejects_invalid_speed_and_loop_options() {
    for speed in ["0", "-1", "NaN", "inf", "-inf"] {
        sixel_cmd()
            .args(["animate", "not-opened.gif", "--speed", speed])
            .assert()
            .code(2)
            .stderr(predicate::str::contains("speed must be a finite number greater than zero"));
    }
    sixel_cmd().args(["animate", "not-opened.gif", "--loops", "-2"]).assert().code(2);
}

#[test]
fn animation_uses_gif_loop_count_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("finite.gif");
    write_gif(&path, Repeat::Finite(2));
    let output = sixel_cmd().args(["-q", "animate"]).arg(&path).assert().success().get_output().stdout.clone();
    assert_eq!(output.windows(2).filter(|bytes| *bytes == b"\x1bP").count(), 4);
    assert_eq!(output.windows(3).filter(|bytes| *bytes == b"\x1b[s").count(), 1);
    assert_eq!(output.windows(3).filter(|bytes| *bytes == b"\x1b[u").count(), 3);
}

#[test]
fn explicit_loop_count_overrides_infinite_gif() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("infinite.gif");
    write_gif(&path, Repeat::Infinite);
    let output = sixel_cmd()
        .args(["-q", "animate", "--loops", "1"])
        .arg(&path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(output.windows(2).filter(|bytes| *bytes == b"\x1bP").count(), 2);
}

#[test]
fn unrepresentable_frame_delay_returns_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("finite.gif");
    write_gif(&path, Repeat::Finite(1));
    sixel_cmd()
        .args(["-q", "animate", "--speed", "1e-38"])
        .arg(&path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("frame delay is too large"));
}

#[test]
fn extracting_first_frame_ignores_corrupt_later_frame() {
    use image::codecs::gif::GifDecoder;
    use std::io::Cursor;

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("broken.gif");
    write_gif(&input, Repeat::Finite(1));
    let bytes = std::fs::read(&input).unwrap();
    // Find a truncation that leaves frame 0 valid and frame 1 incomplete.
    let end = (1..bytes.len())
        .find(|&end| {
            let Ok(decoder) = GifDecoder::new(Cursor::new(&bytes[..end])) else {
                return false;
            };
            let mut frames = decoder.into_frames();
            matches!(frames.next(), Some(Ok(_))) && matches!(frames.next(), Some(Err(_)))
        })
        .expect("truncated second frame");
    std::fs::write(&input, &bytes[..end]).unwrap();
    sixel_cmd().args(["-q", "animate", "--frame", "0"]).arg(&input).assert().success();
    sixel_cmd().args(["-q", "animate", "--frame", "1"]).arg(&input).assert().failure();
}

#[test]
fn frame_extraction_and_file_output_are_consistent() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("two.gif");
    let output = dir.path().join("animation.six");
    write_gif(&input, Repeat::Finite(1));
    let second = sixel_cmd()
        .args(["-q", "animate", "--frame", "1"])
        .arg(&input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let image = icy_sixel::SixelImage::decode(&second).unwrap();
    assert_eq!(&image.pixels[..4], &[0, 0, 255, 255]);
    sixel_cmd()
        .args(["-q", "animate", "--frame", "2"])
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("GIF has 2 frames"));
    sixel_cmd().args(["-q", "animate", "-o"]).arg(&output).arg(&input).assert().success();
    let bytes = std::fs::read(output).unwrap();
    assert_eq!(bytes.windows(2).filter(|bytes| *bytes == b"\x1bP").count(), 2);
    assert!(bytes.ends_with(&second));
}

#[test]
fn gif_without_loop_extension_plays_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no-loop.gif");
    {
        let mut encoder = GifEncoder::new(File::create(&path).unwrap());
        for _ in 0..2 {
            encoder
                .encode_frame(Frame::new(RgbaImage::from_raw(1, 1, vec![255, 0, 0, 255]).unwrap()))
                .unwrap();
        }
    }
    let bytes = std::fs::read(&path).unwrap();
    assert!(!bytes.windows(8).any(|bytes| bytes == b"NETSCAPE"));
    let output = sixel_cmd().args(["-q", "animate"]).arg(&path).assert().success().get_output().stdout.clone();
    assert_eq!(output.windows(2).filter(|bytes| *bytes == b"\x1bP").count(), 2);
    let repeated = sixel_cmd()
        .args(["-q", "animate", "--loops=2"])
        .arg(&path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(repeated.windows(2).filter(|bytes| *bytes == b"\x1bP").count(), 4);
}

#[test]
fn oversized_gif_screen_is_rejected_in_every_mode() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("huge.gif");
    let output = dir.path().join("out.six");
    write_gif(&input, Repeat::Finite(1));
    let mut bytes = std::fs::read(&input).unwrap();
    for (width, height) in [(65535u16, 65535u16), (8192, 8192)] {
        bytes[6..8].copy_from_slice(&width.to_le_bytes());
        bytes[8..10].copy_from_slice(&height.to_le_bytes());
        std::fs::write(&input, &bytes).unwrap();
        for args in [vec![], vec!["--frame=0"], vec!["-o", output.to_str().unwrap()]] {
            sixel_cmd()
                .args(["-q", "animate"])
                .arg(&input)
                .args(args)
                .assert()
                .code(1)
                .stdout(predicate::str::is_empty())
                .stderr(predicate::str::contains("GIF canvas exceeds supported dimensions"));
        }
        assert!(!output.exists());
    }
}

#[test]
fn file_export_does_not_compute_unused_frame_delays() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("finite.gif");
    let normal = dir.path().join("normal.six");
    let slow = dir.path().join("slow.six");
    write_gif(&input, Repeat::Finite(1));
    for (speed, output) in [("1", &normal), ("1e-38", &slow)] {
        sixel_cmd()
            .args(["-q", "animate", "--speed", speed, "-o"])
            .arg(output)
            .arg(&input)
            .assert()
            .success();
    }
    assert_eq!(std::fs::read(normal).unwrap(), std::fs::read(slow).unwrap());
    sixel_cmd().args(["-q", "animate", "--speed=1e-38", "--frame=0"]).arg(&input).assert().success();
}
