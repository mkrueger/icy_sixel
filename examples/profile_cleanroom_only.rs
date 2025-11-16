use icy_sixel::decoder;
use std::fs;

fn main() {
    let snake_data = fs::read("tests/data/snake.six").expect("Failed to read snake.six");

    // Run many iterations for better profiling data
    for _ in 0..1000 {
        let _ = decoder::sixel_decode(&snake_data).expect("Decode failed");
    }
}
