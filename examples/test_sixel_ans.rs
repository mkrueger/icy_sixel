use icy_sixel::sixel_decode;
use std::fs;

fn main() {
    let path = "/home/mkrueger/work/icy_tools/crates/icy_engine/tests/output/ansi/files/sixel.ans";

    match fs::read(path) {
        Ok(data) => {
            println!("Read {} bytes from sixel.ans", data.len());

            match sixel_decode(&data) {
                Ok((_pixels, width, height)) => {
                    println!("Successfully decoded: {}x{} pixels", width, height);
                }
                Err(e) => {
                    eprintln!("Decoding failed: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
        }
    }
}
