use icy_sixel::decoder;
use std::fs;

fn main() {
    // Load snake.six file
    let snake_data = fs::read("tests/data/snake.six").expect("Failed to read snake.six");
    println!("Loaded snake.six: {} bytes", snake_data.len());

    // Run many iterations for profiling
    println!("Running clean-room decoder 1000 times...");
    for i in 0..1000 {
        let _ = decoder::sixel_decode(&snake_data).expect("Decode failed");
        if i % 100 == 0 {
            println!("Progress: {}/1000", i);
        }
    }
    println!("Done!");
}
