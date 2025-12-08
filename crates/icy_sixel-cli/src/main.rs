//! sixel - Encode and decode SIXEL graphics
//!
//! A command-line tool for converting images to/from SIXEL format.

use clap::{Parser, Subcommand, ValueEnum};
use icy_sixel::{sixel_decode, sixel_encode, EncodeOptions, QuantizeMethod};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

/// CLI argument wrapper for QuantizeMethod
#[derive(Clone, Copy, Debug, ValueEnum)]
enum QuantizeMethodArg {
    /// Wu's color quantizer (fast and high quality)
    Wu,
    /// K-means clustering (slower but may be more accurate)
    Kmeans,
}

impl From<QuantizeMethodArg> for QuantizeMethod {
    fn from(arg: QuantizeMethodArg) -> Self {
        match arg {
            QuantizeMethodArg::Wu => QuantizeMethod::Wu,
            QuantizeMethodArg::Kmeans => QuantizeMethod::kmeans(),
        }
    }
}

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
        /// Input image file (PNG, JPEG, GIF, WebP), defaults to stdin
        input: Option<PathBuf>,

        /// Output SIXEL file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Maximum number of colors (2-256)
        #[arg(short, long, default_value = "256")]
        colors: u16,

        /// Floyd-Steinberg error diffusion strength (0.0-1.0, default: 0.875)
        #[arg(short, long, default_value = "0.875")]
        diffusion: f32,

        /// Color quantization method
        #[arg(short = 'm', long, default_value = "wu", value_enum)]
        method: QuantizeMethodArg,
    },

    /// Decode a SIXEL file to PNG
    Decode {
        /// Input SIXEL file, defaults to stdin
        input: Option<PathBuf>,

        /// Output PNG file (required when reading from stdin)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode {
            input,
            output,
            colors,
            diffusion,
            method,
        } => {
            // Read image data from file or stdin
            let (img, source_name) = match &input {
                Some(path) if path.to_string_lossy() != "-" => {
                    let img = image::open(path)
                        .map_err(|e| format!("Failed to open '{}': {}", path.display(), e))?;
                    (img, path.display().to_string())
                }
                _ => {
                    let mut buf = Vec::new();
                    io::stdin().read_to_end(&mut buf)?;
                    let img = image::load_from_memory(&buf)
                        .map_err(|e| format!("Failed to decode image from stdin: {}", e))?;
                    (img, "stdin".to_string())
                }
            };

            let rgba_img = img.to_rgba8();
            let (width, height) = rgba_img.dimensions();
            let pixels = rgba_img.into_raw();

            eprintln!(
                "Encoding '{}' ({}x{}) with {} colors, diffusion={:.3}, method={:?}",
                source_name,
                width,
                height,
                colors.clamp(2, 256),
                diffusion.clamp(0.0, 1.0),
                method
            );

            let opts = EncodeOptions {
                max_colors: colors.clamp(2, 256),
                diffusion: diffusion.clamp(0.0, 1.0),
                quantize_method: method.into(),
            };

            let sixel = sixel_encode(&pixels, width as usize, height as usize, &opts)?;

            match output {
                Some(path) => {
                    fs::write(&path, &sixel)?;
                    eprintln!("Written {} bytes to '{}'", sixel.len(), path.display());
                }
                None => {
                    io::stdout().write_all(sixel.as_bytes())?;
                    io::stdout().flush()?;
                }
            }
        }

        Commands::Decode { input, output } => {
            let (sixel_data, from_stdin) = match &input {
                Some(path) if path.to_string_lossy() != "-" => {
                    let data = fs::read(path)
                        .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
                    (data, false)
                }
                _ => {
                    let mut buf = Vec::new();
                    io::stdin().read_to_end(&mut buf)?;
                    (buf, true)
                }
            };

            eprintln!("Decoding ({} bytes)", sixel_data.len());

            let image = sixel_decode(&sixel_data)?;

            let output_path = match output {
                Some(path) => path,
                None => {
                    if from_stdin {
                        return Err("Output file (-o) is required when reading from stdin".into());
                    }
                    let mut p = input.unwrap();
                    p.set_extension("png");
                    p
                }
            };

            let img =
                image::RgbaImage::from_raw(image.width as u32, image.height as u32, image.pixels)
                    .ok_or("Failed to create image from decoded data")?;
            img.save(&output_path)?;

            eprintln!(
                "Decoded: {}x{} pixels -> '{}'",
                image.width,
                image.height,
                output_path.display()
            );
        }
    }

    Ok(())
}
