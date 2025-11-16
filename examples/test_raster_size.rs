use icy_sixel::sixel_decode;

fn main() {
    // Simplified version of the sixel.ans pattern:
    // Raster attributes set 268x380, then draw 268 sixels with color 0
    let test_data = b"\x1bPq\"1;1;268;380#0;2;0;0;0#1;2;3;3;3#0!268~-\x1b\\";

    match sixel_decode(test_data) {
        Ok((_pixels, width, height)) => {
            println!("Decoded: {}x{} pixels", width, height);
            println!("Expected width: 268, got: {}", width);
            println!("Expected height: at least 380, got: {}", height);

            if width != 268 {
                eprintln!("ERROR: Width mismatch! Expected 268, got {}", width);
            }
            if height < 380 {
                eprintln!(
                    "ERROR: Height too small! Expected at least 380, got {}",
                    height
                );
            }
        }
        Err(e) => {
            eprintln!("Decoding failed: {}", e);
        }
    }
}
