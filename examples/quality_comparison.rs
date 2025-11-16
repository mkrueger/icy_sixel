use icy_sixel::*;

fn main() {
    // Load a simple test image
    let img = image::open("tests/data/snake.png").expect("Failed to open snake.png");
    let img = img.to_rgb8();
    let (width, height) = img.dimensions();
    let pixels = img.into_raw();

    println!("Testing SIXEL encoding quality settings for {}x{} image\n", width, height);

    // Test different combinations
    let test_cases = vec![
        ("High Quality + Floyd-Steinberg", Quality::HIGH, DiffusionMethod::FS),
        ("High Quality + No Dithering", Quality::HIGH, DiffusionMethod::None),
        ("High Quality + Atkinson", Quality::HIGH, DiffusionMethod::Atkinson),
        ("High Quality + Stucki", Quality::HIGH, DiffusionMethod::Stucki),
        ("Low Quality + Floyd-Steinberg", Quality::LOW, DiffusionMethod::FS),
        ("Auto Quality + Auto Dithering", Quality::AUTO, DiffusionMethod::Auto),
    ];

    for (name, quality, diffusion) in test_cases {
        let sixel_data = sixel_string(
            &pixels,
            width as i32,
            height as i32,
            PixelFormat::RGB888,
            diffusion,
            MethodForLargest::Auto,
            MethodForRep::Auto,
            quality,
        )
        .expect("Failed to encode");

        let (decoded, dec_w, dec_h) = sixel_decode(sixel_data.as_bytes())
            .expect("Failed to decode");

        // Calculate MSE (Mean Squared Error) for quality comparison
        let mut total_error = 0u64;
        let mut pixel_count = 0;
        
        for y in 0..height.min(dec_h as u32) {
            for x in 0..width.min(dec_w as u32) {
                let orig_idx = ((y * width + x) * 3) as usize;
                let dec_idx = ((y * dec_w as u32 + x) * 4) as usize;
                
                if orig_idx + 2 < pixels.len() && dec_idx + 3 < decoded.len() {
                    let r_err = (pixels[orig_idx] as i32 - decoded[dec_idx] as i32).abs() as u64;
                    let g_err = (pixels[orig_idx + 1] as i32 - decoded[dec_idx + 1] as i32).abs() as u64;
                    let b_err = (pixels[orig_idx + 2] as i32 - decoded[dec_idx + 2] as i32).abs() as u64;
                    
                    total_error += r_err * r_err + g_err * g_err + b_err * b_err;
                    pixel_count += 1;
                }
            }
        }
        
        let mse = if pixel_count > 0 { total_error / pixel_count } else { 0 };
        let psnr = if mse > 0 {
            let max_val = 255.0;
            20.0 * (max_val / (mse as f64).sqrt()).log10()
        } else {
            f64::INFINITY
        };

        println!("{:40} | Size: {:7} bytes | MSE: {:6} | PSNR: {:5.2} dB", 
                 name, sixel_data.len(), mse, psnr);
    }

    println!("\nNote: SIXEL format is limited to 256 colors.");
    println!("Higher PSNR values indicate better quality (>30 dB is good).");
    println!("Lower MSE values indicate less error (0 would be perfect).");
}
