// Example: Decode SIXEL from already-parsed ANSI DCS parameters
//
// This demonstrates how to use sixel_decode_from_dcs when you've already
// parsed the DCS sequence: ESC P p1;p2;p3 q <sixel_data> ST
//
// Usage from an ANSI parser:
// 1. Parse DCS introducer: ESC P
// 2. Parse optional parameters p1, p2, p3 followed by 'q'
// 3. Collect sixel data until ST (ESC \ or 0x9c)
// 4. Call sixel_decode_from_dcs with the extracted values

use icy_sixel::sixel_decode_from_dcs;

fn main() {
    // Simulate what an ANSI parser would provide:
    // ESC P 2 ; 1 ; 10 q <sixel_data> ST
    //       ^   ^   ^
    //       |   |   grid_size (zoom)
    //       |   zero_color (background)
    //       aspect_ratio (5:1 pixel aspect)

    let aspect_ratio = Some(2); // P1: aspect ratio 5:1
    let zero_color = Some(1); // P2: background color index (currently unused)
    let grid_size = Some(10); // P3: grid size/zoom = 10 (1:1)

    // Raw SIXEL data (after 'q', before ST)
    // This creates a simple pattern with color definitions
    let sixel_data = b"#0;2;100;0;0#1;2;0;100;0#0~~~#1~~~\x1b\\";

    match sixel_decode_from_dcs(aspect_ratio, zero_color, grid_size, sixel_data) {
        Ok((pixels, width, height)) => {
            println!("Decoded SIXEL image:");
            println!("  Width:  {} pixels", width);
            println!("  Height: {} pixels", height);
            println!("  Size:   {} bytes (RGB)", pixels.len());
            println!(
                "\nFirst pixel (R,G,B): ({}, {}, {})",
                pixels[0], pixels[1], pixels[2]
            );
        }
        Err(e) => {
            eprintln!("Failed to decode: {}", e);
        }
    }

    // Example with no parameters (default aspect ratio and grid)
    println!("\n---\n");
    let sixel_data_simple = b"#0~@?-#1~@?\x1b\\";

    match sixel_decode_from_dcs(None, None, None, sixel_data_simple) {
        Ok((pixels, width, height)) => {
            println!("Decoded simple SIXEL (no params):");
            println!("  Width:  {} pixels", width);
            println!("  Height: {} pixels", height);
            println!("  Size:   {} bytes (RGB)", pixels.len());
        }
        Err(e) => {
            eprintln!("Failed to decode: {}", e);
        }
    }
}
