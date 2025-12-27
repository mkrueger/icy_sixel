//! sixel - Encode and decode SIXEL graphics
//!
//! A command-line tool for converting images to/from SIXEL format.

use clap::{Parser, Subcommand, ValueEnum};
use icy_sixel::{BackgroundMode, EncodeOptions, PixelAspectRatio, QuantizeMethod, SixelImage};
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, ImageDecoder};
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::PathBuf;
use std::time::Duration;
use std::{fs, thread};

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

/// CLI argument wrapper for PixelAspectRatio
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum AspectRatioArg {
    /// 1:1 square pixels (default, best for modern terminals)
    #[default]
    Square,
    /// 2:1 aspect ratio (VT240/VT340 native)
    Ratio2to1,
    /// 3:1 aspect ratio (tall pixels)
    Ratio3to1,
    /// 5:1 aspect ratio (very tall pixels)
    Ratio5to1,
}

impl From<AspectRatioArg> for PixelAspectRatio {
    fn from(arg: AspectRatioArg) -> Self {
        match arg {
            AspectRatioArg::Square => PixelAspectRatio::Square,
            AspectRatioArg::Ratio2to1 => PixelAspectRatio::Ratio2To1,
            AspectRatioArg::Ratio3to1 => PixelAspectRatio::Ratio3To1,
            AspectRatioArg::Ratio5to1 => PixelAspectRatio::Ratio5To1,
        }
    }
}

/// CLI argument wrapper for BackgroundMode
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum BackgroundArg {
    /// Transparent - undrawn pixels keep their current color (default)
    #[default]
    Transparent,
    /// Opaque - undrawn pixels are set to background color
    Opaque,
}

impl From<BackgroundArg> for BackgroundMode {
    fn from(arg: BackgroundArg) -> Self {
        match arg {
            BackgroundArg::Transparent => BackgroundMode::Transparent,
            BackgroundArg::Opaque => BackgroundMode::Opaque,
        }
    }
}

#[derive(Parser)]
#[command(name = "sixel")]
#[command(author = "Mike Krüger <mkrueger@posteo.de>")]
#[command(version)]
#[command(about = "Encode and decode SIXEL graphics", long_about = None)]
struct Cli {
    /// Suppress informational messages (errors are still shown)
    #[arg(short, long, global = true)]
    quiet: bool,

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

        /// Pixel aspect ratio (how terminals interpret pixel dimensions)
        #[arg(short = 'a', long, default_value = "square", value_enum)]
        aspect_ratio: AspectRatioArg,

        /// Background mode (how undrawn pixels are handled)
        #[arg(short = 'b', long, default_value = "transparent", value_enum)]
        background: BackgroundArg,
    },

    /// Play an animated GIF in the terminal using SIXEL
    Animate {
        /// Input GIF file
        input: PathBuf,

        /// Output SIXEL file (default: stdout/terminal playback)
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

        /// Pixel aspect ratio (how terminals interpret pixel dimensions)
        #[arg(short = 'a', long, default_value = "square", value_enum)]
        aspect_ratio: AspectRatioArg,

        /// Background mode (how undrawn pixels are handled)
        #[arg(short = 'b', long, default_value = "opaque", value_enum)]
        background: BackgroundArg,

        /// Number of times to loop (0 = use GIF's loop count, -1 = infinite)
        #[arg(short, long, default_value = "0")]
        loops: i32,

        /// Speed multiplier (e.g., 2.0 = twice as fast, 0.5 = half speed)
        #[arg(short, long, default_value = "1.0")]
        speed: f32,

        /// Extract a single frame (0-indexed) instead of animating
        #[arg(short = 'f', long)]
        frame: Option<usize>,
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
    let quiet = cli.quiet;

    // Helper macro for conditional stderr output
    macro_rules! info {
        ($($arg:tt)*) => {
            if !quiet {
                eprintln!($($arg)*);
            }
        };
    }
    macro_rules! info_no_nl {
        ($($arg:tt)*) => {
            if !quiet {
                eprint!($($arg)*);
            }
        };
    }

    match cli.command {
        Commands::Encode {
            input,
            output,
            colors,
            diffusion,
            method,
            aspect_ratio,
            background,
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

            info!(
                "Encoding '{}' ({}x{}) with {} colors, diffusion={:.3}, method={:?}, aspect={:?}, bg={:?}",
                source_name,
                width,
                height,
                colors.clamp(2, 256),
                diffusion.clamp(0.0, 1.0),
                method,
                aspect_ratio,
                background
            );

            let opts = EncodeOptions {
                max_colors: colors.clamp(2, 256),
                diffusion: diffusion.clamp(0.0, 1.0),
                quantize_method: method.into(),
            };

            let image = SixelImage::try_from_rgba(pixels, width as usize, height as usize)?
                .with_aspect_ratio(aspect_ratio.into())
                .with_background_mode(background.into());
            let sixel = image.encode_with(&opts)?;

            match output {
                Some(path) => {
                    fs::write(&path, &sixel)?;
                    info!("Written {} bytes to '{}'", sixel.len(), path.display());
                }
                None => {
                    io::stdout().write_all(sixel.as_bytes())?;
                    io::stdout().flush()?;
                }
            }
        }

        Commands::Animate {
            input,
            output,
            colors,
            diffusion,
            method,
            aspect_ratio,
            background,
            loops,
            speed,
            frame,
        } => {
            // Open GIF file
            let file = File::open(&input)
                .map_err(|e| format!("Failed to open '{}': {}", input.display(), e))?;
            let reader = BufReader::new(file);

            // Decode GIF
            let decoder = GifDecoder::new(reader)
                .map_err(|e| format!("Failed to decode GIF '{}': {}", input.display(), e))?;

            let (width, height) = decoder.dimensions();

            // Get frames
            let frames: Vec<_> = decoder
                .into_frames()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to decode GIF frames: {}", e))?;

            if frames.is_empty() {
                return Err("GIF has no frames".into());
            }

            let opts = EncodeOptions {
                max_colors: colors.clamp(2, 256),
                diffusion: diffusion.clamp(0.0, 1.0),
                quantize_method: method.into(),
            };

            // Single frame extraction mode
            if let Some(frame_idx) = frame {
                if frame_idx >= frames.len() {
                    return Err(format!(
                        "Frame {} does not exist (GIF has {} frames, 0-indexed)",
                        frame_idx,
                        frames.len()
                    )
                    .into());
                }

                info!(
                    "Extracting frame {} from '{}' ({}x{}, {} frames)",
                    frame_idx,
                    input.display(),
                    width,
                    height,
                    frames.len()
                );

                let frame_data = &frames[frame_idx];
                let rgba = frame_data.buffer();
                let (w, h) = rgba.dimensions();
                let pixels = rgba.as_raw();
                let image = SixelImage::try_from_rgba(pixels.to_vec(), w as usize, h as usize)?
                    .with_aspect_ratio(aspect_ratio.into())
                    .with_background_mode(background.into());
                let sixel = image.encode_with(&opts)?;

                match output {
                    Some(path) => {
                        fs::write(&path, &sixel)?;
                        info!("Written frame {} to '{}'", frame_idx, path.display());
                    }
                    None => {
                        io::stdout().write_all(sixel.as_bytes())?;
                        io::stdout().flush()?;
                    }
                }
                return Ok(());
            }

            info!(
                "Animating '{}' ({}x{}, {} frames) with {} colors, speed={:.1}x",
                input.display(),
                width,
                height,
                frames.len(),
                colors.clamp(2, 256),
                speed
            );

            // Pre-encode all frames to SIXEL
            let total_frames = frames.len();
            let encoded_frames: Vec<(String, Duration)> = frames
                .iter()
                .enumerate()
                .map(|(i, frame)| {
                    info_no_nl!("\rEncoding frame {}/{}...", i + 1, total_frames);
                    let rgba = frame.buffer();
                    let (w, h) = rgba.dimensions();
                    let pixels = rgba.as_raw();
                    let image = SixelImage::try_from_rgba(pixels.to_vec(), w as usize, h as usize)?
                        .with_aspect_ratio(aspect_ratio.into())
                        .with_background_mode(background.into());
                    let sixel = image.encode_with(&opts)?;

                    // Get frame delay (in milliseconds, apply speed multiplier)
                    let delay = frame.delay().numer_denom_ms();
                    let delay_ms = (delay.0 as f32 / delay.1 as f32) / speed;
                    let duration = Duration::from_millis(delay_ms.max(1.0) as u64);

                    Ok((sixel, duration))
                })
                .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;

            info!("\rEncoded {} frames.           ", encoded_frames.len());

            // ANSI escape sequences for cursor save/restore
            const SAVE_CURSOR: &str = "\x1b[s";
            const RESTORE_CURSOR: &str = "\x1b[u";

            // Determine if we're writing to file or terminal
            if let Some(ref path) = output {
                // File output mode - write all frames with cursor sequences (single pass)
                let mut file_output = String::new();

                for (i, (sixel, _delay)) in encoded_frames.iter().enumerate() {
                    if i > 0 {
                        file_output.push_str(RESTORE_CURSOR);
                    } else {
                        file_output.push_str(SAVE_CURSOR);
                    }
                    file_output.push_str(sixel);
                }

                fs::write(path, &file_output)?;
                info!(
                    "Written {} bytes ({} frames) to '{}'",
                    file_output.len(),
                    encoded_frames.len(),
                    path.display()
                );
            } else {
                // Terminal playback mode
                // Determine loop count (-1 or 0 means infinite)
                let loop_count = if loops <= 0 {
                    usize::MAX
                } else {
                    loops as usize
                };

                info!("Starting animation (Ctrl+C to stop)...");

                let mut stdout = io::stdout();

                for loop_num in 0..loop_count {
                    for (i, (sixel, delay)) in encoded_frames.iter().enumerate() {
                        if loop_num > 0 || i > 0 {
                            stdout.write_all(RESTORE_CURSOR.as_bytes())?;
                        } else {
                            stdout.write_all(SAVE_CURSOR.as_bytes())?;
                        }

                        stdout.write_all(sixel.as_bytes())?;
                        stdout.flush()?;

                        thread::sleep(*delay);
                    }
                }

                info!("\nAnimation complete.");
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

            info!("Decoding ({} bytes)", sixel_data.len());

            let image = SixelImage::decode(&sixel_data)?;

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

            info!(
                "Decoded: {}x{} pixels -> '{}'",
                image.width,
                image.height,
                output_path.display()
            );
        }
    }

    Ok(())
}
