use assert_cmd::Command;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, RgbaImage};
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
