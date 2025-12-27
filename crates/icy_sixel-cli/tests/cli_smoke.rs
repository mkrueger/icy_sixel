use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

fn sixel_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("sixel"))
}

#[test]
fn encode_png_to_stdout_emits_sixel_dcs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input_path = dir.path().join("in.png");

    // Create a tiny 2x1 RGBA PNG so the CLI has something deterministic to encode.
    let img = image::RgbaImage::from_raw(
        2,
        1,
        vec![255, 0, 0, 255, 0, 255, 0, 255], // red, green
    )
    .expect("create image");
    img.save(&input_path).expect("write png");

    let mut cmd = sixel_cmd();
    cmd.args(["encode"])
        .arg(&input_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1bP"))
        .stdout(predicate::str::contains("q"))
        .stdout(predicate::str::contains("\x1b\\"));
}

#[test]
fn encode_to_file_and_decode_to_png_roundtrips_dimensions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input_path = dir.path().join("in.png");
    let sixel_path = dir.path().join("out.six");
    let decoded_path = dir.path().join("out.png");

    // SIXEL is encoded in 6-pixel-high bands; using a 1x6 image avoids ambiguity
    // around how decoders represent partial bands.
    let img =
        image::RgbaImage::from_raw(1, 6, [0u8, 0, 255, 255].repeat(6)).expect("create image");
    img.save(&input_path).expect("write png");

    sixel_cmd()
        .args(["encode", "-o"])
        .arg(&sixel_path)
        .arg(&input_path)
        .assert()
        .success();

    let sixel_bytes = fs::read(&sixel_path).expect("read sixel output");
    assert!(!sixel_bytes.is_empty(), "sixel output file is empty");

    sixel_cmd()
        .args(["decode", "-o"])
        .arg(&decoded_path)
        .arg(&sixel_path)
        .assert()
        .success();

    let decoded = image::open(&decoded_path)
        .expect("load decoded png")
        .to_rgba8();
    assert_eq!(decoded.dimensions(), (1, 6));
}
