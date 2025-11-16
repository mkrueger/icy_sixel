use icy_sixel::sixel_decode;
use std::fs;

fn main() {
    let files = ["tests/data/map8.six", "tests/data/snake.six"];

    for file in &files {
        println!("\n=== Decoding {} ===", file);

        match fs::read(file) {
            Ok(data) => {
                match sixel_decode(&data) {
                    Ok((pixels, width, height)) => {
                        println!("✓ Success!");
                        println!("  Dimensions: {}x{}", width, height);
                        println!(
                            "  Buffer size: {} bytes ({} pixels)",
                            pixels.len(),
                            pixels.len() / 3
                        );

                        // Show first few pixels
                        println!("  First 5 pixels (RGB):");
                        for i in 0..5.min(pixels.len() / 3) {
                            let idx = i * 3;
                            println!(
                                "    Pixel {}: ({}, {}, {})",
                                i,
                                pixels[idx],
                                pixels[idx + 1],
                                pixels[idx + 2]
                            );
                        }
                    }
                    Err(e) => {
                        println!("✗ Decode failed: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to read file: {}", e);
            }
        }
    }
}
