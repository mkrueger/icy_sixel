//! Encode PNG images to SIXEL format.
//!
//! Usage:
//!   cargo run --example encode -- input.png -o output.six
//!   cargo run --example encode -- input.png  # outputs to stdout
//!
//! Options:
//!   -o, --output <FILE>   Output SIXEL file (default: stdout)
//!   -c, --colors <N>      Maximum number of colors (2-256, default: 256)
//!   -q, --quality <N>     Quality level (0-100, default: 100)

use clap::Parser;
use icy_sixel::{sixel_encode, EncodeOptions};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "encode")]
#[command(about = "Encode PNG images to SIXEL format", long_about = None)]
struct Args {
    /// Input PNG file
    #[arg(required = true)]
    input: PathBuf,

    /// Output SIXEL file (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Maximum number of colors (2-256)
    #[arg(short, long, default_value = "256")]
    colors: u16,

    /// Quality level (0-100, higher = better quality but slower)
    #[arg(short, long, default_value = "100")]
    quality: u8,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load the image
    let img = image::open(&args.input)
        .map_err(|e| format!("Failed to open '{}': {}", args.input.display(), e))?;
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let pixels = rgba_img.into_raw();

    eprintln!(
        "Encoding '{}' ({}x{}) with {} colors, quality={}",
        args.input.display(),
        width,
        height,
        args.colors,
        args.quality
    );

    // Set up encoding options
    let opts = EncodeOptions {
        max_colors: args.colors.clamp(2, 256),
        quality: args.quality.clamp(0, 100),
    };

    // Encode to SIXEL
    let sixel = sixel_encode(&pixels, width as usize, height as usize, &opts)?;

    // Output
    match args.output {
        Some(path) => {
            fs::write(&path, &sixel)?;
            eprintln!("Written {} bytes to '{}'", sixel.len(), path.display());
        }
        None => {
            io::stdout().write_all(sixel.as_bytes())?;
        }
    }

    Ok(())
}
