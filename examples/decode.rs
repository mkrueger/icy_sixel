//! Decode SIXEL images to PNG format.
//!
//! Usage:
//!   cargo run --example decode -- input.six -o output.png
//!   cargo run --example decode -- input.six  # outputs to input.png
//!
//! Options:
//!   -o, --output <FILE>   Output PNG file (default: derived from input)

use clap::Parser;
use icy_sixel::sixel_decode;
use image::{ImageBuffer, Rgba};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "decode")]
#[command(about = "Decode SIXEL images to PNG format", long_about = None)]
struct Args {
    /// Input SIXEL file
    #[arg(required = true)]
    input: PathBuf,

    /// Output PNG file (default: input with .png extension)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Determine output path
    let output_path = args.output.unwrap_or_else(|| {
        let mut path = args.input.clone();
        path.set_extension("png");
        path
    });

    // Read SIXEL data
    let sixel_data = fs::read(&args.input)
        .map_err(|e| format!("Failed to read '{}': {}", args.input.display(), e))?;

    eprintln!(
        "Decoding '{}' ({} bytes)",
        args.input.display(),
        sixel_data.len()
    );

    // Decode SIXEL
    let (rgba, width, height) = sixel_decode(&sixel_data)?;

    eprintln!("Decoded: {}x{} pixels", width, height);

    // Create image and save
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width as u32, height as u32, rgba)
            .ok_or("Failed to create image buffer")?;

    img.save(&output_path)?;
    eprintln!("Saved to '{}'", output_path.display());

    Ok(())
}
