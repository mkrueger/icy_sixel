use icy_sixel::SixelImage;
use std::hint::black_box;

fn main() {
    let mut args = std::env::args().skip(1);
    let fixture = args.next().unwrap_or_else(|| "simple".into());
    let iterations: usize = args.next().map(|value| value.parse().expect("invalid iteration count")).unwrap_or(100_000);
    let data: &[u8] = match fixture.as_str() {
        "simple" => b"\x1bPq#0;2;100;0;0#0~~~\x1b\\",
        "beelitz" => include_bytes!("../tests/data/beelitz_heilstätten.six"),
        "transparency" => include_bytes!("../tests/data/transparency.six"),
        _ => panic!("expected simple, beelitz or transparency"),
    };
    for _ in 0..iterations {
        black_box(SixelImage::decode(black_box(data)).unwrap());
    }
}
