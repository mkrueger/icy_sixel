use icy_sixel::*;

fn main() {
    // Load the PNG
    let img = image::open("tests/data/snake.png").expect("Failed to open snake.png");
    let img = img.to_rgb8();
    let (width, height) = img.dimensions();
    let pixels = img.into_raw();

    println!("Loaded image: {}x{}", width, height);

    // Encode to SIXEL with best quality settings
    // Note: SIXEL is limited to 256 colors, so some quality loss is inevitable
    // for images with more colors. Using Floyd-Steinberg dithering for best results.
    let sixel_data: String = sixel_string(
        &pixels,
        width as i32,
        height as i32,
        PixelFormat::RGB888,
        DiffusionMethod::FS,  // Floyd-Steinberg dithering for smooth gradients
        MethodForLargest::Auto,
        MethodForRep::Auto,
        Quality::HIGH,  // HIGH often produces better results than FULL
    )
    .expect("Failed to encode");

    println!("Encoded to {} bytes of SIXEL", sixel_data.len());

    // Decode with the clean-room decoder
    let (decoded_pixels, decoded_width, decoded_height) =
        sixel_decode(sixel_data.as_bytes()).expect("Failed to decode");

    println!("Decoded: {}x{}", decoded_width, decoded_height);

    // Save as PNG (decoder returns RGBA, so use Rgba8)
    image::save_buffer(
        "tests/data/snake.decoded.png",
        &decoded_pixels,
        decoded_width as u32,
        decoded_height as u32,
        image::ColorType::Rgba8,
    )
    .expect("Failed to save decoded image");

    println!("Saved decoded image to tests/data/snake.decoded.png");
}
