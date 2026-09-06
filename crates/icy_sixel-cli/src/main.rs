//! sixel - Encode and decode SIXEL graphics
//!
//! A command-line tool for converting images to/from SIXEL format.

use clap::{Parser, Subcommand, ValueEnum};
use icy_sixel::{BackgroundMode, EncodeOptions, PixelAspectRatio, QuantizeMethod, SixelImage};
use image::codecs::gif::GifDecoder;
use image::metadata::LoopCount;
use image::{AnimationDecoder, ImageDecoder};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
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
        #[arg(short, long, default_value = "0", allow_hyphen_values = true, value_parser = clap::value_parser!(i32).range(-1..))]
        loops: i32,

        /// Speed multiplier (e.g., 2.0 = twice as fast, 0.5 = half speed)
        #[arg(short, long, default_value = "1.0", allow_hyphen_values = true, value_parser = parse_speed)]
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

fn parse_speed(value: &str) -> Result<f32, String> {
    let speed: f32 = value.parse().map_err(|_| "speed must be a finite number greater than zero".to_string())?;
    if !speed.is_finite() || speed <= 0.0 {
        return Err("speed must be a finite number greater than zero".to_string());
    }
    Ok(speed)
}

fn frame_duration(delay: image::Delay, speed: f32) -> Result<Duration, &'static str> {
    let (numerator, denominator) = delay.numer_denom_ms();
    let seconds = f64::from(numerator) / f64::from(denominator) / f64::from(speed) / 1000.0;
    Duration::try_from_secs_f64(seconds.max(0.001)).map_err(|_| "frame delay is too large for the requested speed")
}

fn playback_loops(requested: i32, gif_loops: LoopCount) -> Option<u32> {
    match requested {
        -1 => None,
        0 => match gif_loops {
            LoopCount::Infinite => None,
            LoopCount::Finite(count) => Some(count.get()),
        },
        count => Some(count as u32), // clap validates count >= -1
    }
}

const MAX_ANIMATION_CACHE_BYTES: usize = 256 * 1024 * 1024;

fn animation_cache_size(used: usize, frame_capacity: usize) -> Result<usize, &'static str> {
    // Budget string allocations plus frame metadata, including geometric Vec slack.
    used.checked_add(frame_capacity)
        .and_then(|bytes| bytes.checked_add(2 * std::mem::size_of::<(String, Duration)>()))
        .filter(|&bytes| bytes <= MAX_ANIMATION_CACHE_BYTES)
        .ok_or("animation exceeds the 256 MiB SIXEL cache limit; reduce image size or extract a single frame")
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
                    let img = image::open(path).map_err(|e| format!("Failed to open '{}': {}", path.display(), e))?;
                    (img, path.display().to_string())
                }
                _ => {
                    let mut buf = Vec::new();
                    io::stdin().read_to_end(&mut buf)?;
                    let img = image::load_from_memory(&buf).map_err(|e| format!("Failed to decode image from stdin: {}", e))?;
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
            let file = File::open(&input).map_err(|e| format!("Failed to open '{}': {}", input.display(), e))?;
            let reader = BufReader::new(file);

            // Decode GIF
            let decoder = GifDecoder::new(reader).map_err(|e| format!("Failed to decode GIF '{}': {}", input.display(), e))?;

            let (width, height) = decoder.dimensions();
            let gif_loops = decoder.loop_count();

            let mut frames = decoder.into_frames();

            let opts = EncodeOptions {
                max_colors: colors.clamp(2, 256),
                diffusion: diffusion.clamp(0.0, 1.0),
                quantize_method: method.into(),
            };

            // Single frame extraction mode
            if let Some(frame_idx) = frame {
                let mut selected = None;
                // Decode preceding frames for GIF disposal/compositing, but never
                // inspect later frames or retain all preceding RGBA buffers.
                for index in 0..=frame_idx {
                    let decoded = frames
                        .next()
                        .ok_or_else(|| format!("Frame {frame_idx} does not exist (GIF has {index} frames, 0-indexed)"))?;
                    let decoded = decoded.map_err(|e| format!("Failed to decode GIF frame {index}: {e}"))?;
                    if index == frame_idx {
                        selected = Some(decoded);
                    }
                }

                info!("Extracting frame {} from '{}' ({}x{})", frame_idx, input.display(), width, height);

                let rgba = selected.ok_or("GIF has no frames")?.into_buffer();
                let (w, h) = rgba.dimensions();
                let image = SixelImage::try_from_rgba(rgba.into_raw(), w as usize, h as usize)?
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
                "Animating '{}' ({}x{}) with {} colors, speed={:.1}x",
                input.display(),
                width,
                height,
                colors.clamp(2, 256),
                speed
            );

            // Consume raw frames one at a time; only bounded encoded data is cached.
            let mut encoded_frames = Vec::new();
            let mut cache_bytes = 0;
            for (i, frame) in frames.enumerate() {
                let frame = frame.map_err(|e| format!("Failed to decode GIF frame {i}: {e}"))?;
                info_no_nl!("\rEncoding frame {}...", i + 1);
                let duration = frame_duration(frame.delay(), speed)?;
                let rgba = frame.into_buffer();
                let (w, h) = rgba.dimensions();
                let image = SixelImage::try_from_rgba(rgba.into_raw(), w as usize, h as usize)?
                    .with_aspect_ratio(aspect_ratio.into())
                    .with_background_mode(background.into());
                let sixel = image.encode_with(&opts)?;
                cache_bytes = animation_cache_size(cache_bytes, sixel.capacity())?;
                encoded_frames.push((sixel, duration));
            }
            if encoded_frames.is_empty() {
                return Err("GIF has no frames".into());
            }

            info!("\rEncoded {} frames.           ", encoded_frames.len());

            // ANSI escape sequences for cursor save/restore
            const SAVE_CURSOR: &str = "\x1b[s";
            const RESTORE_CURSOR: &str = "\x1b[u";

            // Determine if we're writing to file or terminal
            if let Some(ref path) = output {
                // File output mode - write all frames with cursor sequences (single pass)
                let mut file_output = BufWriter::new(File::create(path)?);
                let mut written = 0;

                for (i, (sixel, _delay)) in encoded_frames.iter().enumerate() {
                    let cursor = if i > 0 { RESTORE_CURSOR } else { SAVE_CURSOR };
                    file_output.write_all(cursor.as_bytes())?;
                    file_output.write_all(sixel.as_bytes())?;
                    written += cursor.len() + sixel.len();
                }

                file_output.flush()?;
                info!("Written {} bytes ({} frames) to '{}'", written, encoded_frames.len(), path.display());
            } else {
                // Terminal playback mode
                let mut remaining = playback_loops(loops, gif_loops);

                info!("Starting animation (Ctrl+C to stop)...");

                let mut stdout = io::stdout();
                let mut started = false;
                while remaining != Some(0) {
                    for (sixel, delay) in &encoded_frames {
                        if started {
                            stdout.write_all(RESTORE_CURSOR.as_bytes())?;
                        } else {
                            stdout.write_all(SAVE_CURSOR.as_bytes())?;
                            started = true;
                        }

                        stdout.write_all(sixel.as_bytes())?;
                        stdout.flush()?;

                        thread::sleep(*delay);
                    }
                    if let Some(count) = &mut remaining {
                        *count -= 1;
                    }
                }

                info!("\nAnimation complete.");
            }
        }

        Commands::Decode { input, output } => {
            let (sixel_data, from_stdin) = match &input {
                Some(path) if path.to_string_lossy() != "-" => {
                    let data = fs::read(path).map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
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

            let img = image::RgbaImage::from_raw(image.width as u32, image.height as u32, image.pixels).ok_or("Failed to create image from decoded data")?;
            img.save(&output_path)?;

            info!("Decoded: {}x{} pixels -> '{}'", image.width, image.height, output_path.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU32;

    #[test]
    fn animation_cache_budget_includes_metadata_and_checks_overflow() {
        let overhead = 2 * std::mem::size_of::<(String, Duration)>();
        assert_eq!(animation_cache_size(0, 10).unwrap(), 10 + overhead);
        assert_eq!(
            animation_cache_size(MAX_ANIMATION_CACHE_BYTES - overhead, 0).unwrap(),
            MAX_ANIMATION_CACHE_BYTES
        );
        assert!(animation_cache_size(MAX_ANIMATION_CACHE_BYTES - overhead, 1).is_err());
        assert!(animation_cache_size(usize::MAX, 1).is_err());
    }

    #[test]
    fn loop_selection_honors_metadata_and_overrides() {
        let finite = LoopCount::Finite(NonZeroU32::new(3).unwrap());
        assert_eq!(playback_loops(0, finite), Some(3));
        assert_eq!(playback_loops(0, LoopCount::Infinite), None);
        assert_eq!(playback_loops(-1, finite), None);
        assert_eq!(playback_loops(2, finite), Some(2));
        assert_eq!(playback_loops(2, LoopCount::Infinite), Some(2));
    }

    #[test]
    fn frame_delays_are_scaled_without_saturating_casts() {
        let delay = image::Delay::from_numer_denom_ms(100, 1);
        assert_eq!(frame_duration(delay, 2.0).unwrap(), Duration::from_millis(50));
        assert_eq!(frame_duration(delay, 0.5).unwrap(), Duration::from_millis(200));
        assert_eq!(frame_duration(image::Delay::from_numer_denom_ms(0, 1), 1.0).unwrap(), Duration::from_millis(1));
        assert!(frame_duration(delay, f32::MIN_POSITIVE).is_err());
    }
}
