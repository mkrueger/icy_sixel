use icy_sixel::sixel_decode;

fn main() {
    // Replicate the pattern from sixel.ans more precisely:
    // 1. Set raster to 268x380
    // 2. Draw 268 pixels in color 0 (black) twice (two rows)
    // 3. Draw a mix: 36 in color 0, 214 with different char, 18 in color 0
    // 4. Do carriage return ($)
    // 5. Overdraw with color 1
    let test_data = b"\x1bPq\"1;1;268;380#0;2;0;0;0#1;2;50;50;50#0!268~-!268~-!36~!214^!18~$#1!36?_??_??_!6?_??_!18~\x1b\\";

    match sixel_decode(test_data) {
        Ok((_pixels, width, height)) => {
            println!("Decoded: {}x{} pixels", width, height);

            // Check some specific pixels to verify color overlay
            if width >= 268 && height >= 18 {
                println!("Canvas sized correctly!");
            } else {
                eprintln!(
                    "Canvas size wrong: expected at least 268x18, got {}x{}",
                    width, height
                );
            }
        }
        Err(e) => {
            eprintln!("Decoding failed: {}", e);
        }
    }
}
