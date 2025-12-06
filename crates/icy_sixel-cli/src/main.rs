//! sixel - Encode and decode SIXEL graphics
//!
//! A command-line tool for converting images to/from SIXEL format.

use clap::{Parser, Subcommand};
use icy_sixel::{sixel_decode, sixel_encode, EncodeOptions};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sixel")]
#[command(author = "Mike Krüger <mkrueger@posteo.de>")]
#[command(version)]
#[command(about = "Encode and decode SIXEL graphics", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode an image to SIXEL format
    Encode {
        /// Input image file (PNG, JPEG, GIF, WebP)
        input: PathBuf,

        /// Output SIXEL file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Maximum number of colors (2-256)
        #[arg(short, long, default_value = "256")]
        colors: u16,

        /// Quality level (0-100, higher = better quality but larger output)
        #[arg(short, long, default_value = "100")]
        quality: u8,
    },

    /// Decode a SIXEL file to PNG
    Decode {
        /// Input SIXEL file (use - for stdin)
        input: PathBuf,

        /// Output PNG file (default: input with .png extension)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Display an image as SIXEL in the terminal
    Show {
        /// Input image file (PNG, JPEG, GIF, WebP)
        input: PathBuf,

        /// Maximum number of colors (2-256)
        #[arg(short, long, default_value = "256")]
        colors: u16,

        /// Quality level (0-100)
        #[arg(short, long, default_value = "100")]
        quality: u8,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode {
            input,
            output,
            colors,
            quality,
        } => {
            let img = image::open(&input)
                .map_err(|e| format!("Failed to open '{}': {}", input.display(), e))?;
            let rgba_img = img.to_rgba8();
            let (width, height) = rgba_img.dimensions();
            let pixels = rgba_img.into_raw();

            eprintln!(
                "Encoding '{}' ({}x{}) with {} colors, quality={}",
                input.display(),
                width,
                height,
                colors.clamp(2, 256),
                quality.clamp(0, 100)
            );

            let opts = EncodeOptions {
                max_colors: colors.clamp(2, 256),
                quality: quality.clamp(0, 100),
            };

            let sixel = sixel_encode(&pixels, width as usize, height as usize, &opts)?;

            match output {
                Some(path) => {
                    fs::write(&path, &sixel)?;
                    eprintln!("Written {} bytes to '{}'", sixel.len(), path.display());
                }
                None => {
                    io::stdout().write_all(sixel.as_bytes())?;
                }
            }
        }

        Commands::Decode { input, output } => {
            let sixel_data = if input.to_string_lossy() == "-" {
                let mut buf = Vec::new();
                io::stdin().read_to_end(&mut buf)?;
                buf
            } else {
                fs::read(&input)
                    .map_err(|e| format!("Failed to read '{}': {}", input.display(), e))?
            };

            eprintln!("Decoding ({} bytes)", sixel_data.len());

            let (rgba, width, height) = sixel_decode(&sixel_data)?;

            let output_path = output.unwrap_or_else(|| {
                let mut p = input.clone();
                p.set_extension("png");
                p
            });

            let img = image::RgbaImage::from_raw(width as u32, height as u32, rgba)
                .ok_or("Failed to create image from decoded data")?;
            img.save(&output_path)?;

            eprintln!(
                "Decoded: {}x{} pixels -> '{}'",
                width,
                height,
                output_path.display()
            );
        }

        Commands::Show {
            input,
            colors,
            quality,
        } => {
            let img = image::open(&input)
                .map_err(|e| format!("Failed to open '{}': {}", input.display(), e))?;
            let rgba_img = img.to_rgba8();
            let (width, height) = rgba_img.dimensions();
            let pixels = rgba_img.into_raw();

            let opts = EncodeOptions {
                max_colors: colors.clamp(2, 256),
                quality: quality.clamp(0, 100),
            };

            let sixel = sixel_encode(&pixels, width as usize, height as usize, &opts)?;
            print!("{}", sixel);
        }
    }

    Ok(())
}
