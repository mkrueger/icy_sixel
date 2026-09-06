use crate::{
    sixel_image::{BackgroundMode, PixelAspectRatio, SixelImage},
    Result, SixelError, SIXEL_HEIGHT_LIMIT, SIXEL_PALETTE_MAX, SIXEL_REPEAT_MAX, SIXEL_WIDTH_LIMIT,
};

use crate::{SIXEL_CELL_HEIGHT, SIXEL_MAX_PIXELS as MAX_PIXELS};

mod dcs_stream;
pub use dcs_stream::{SixelDcsFeedResult, SixelDcsFeedStatus, SixelDcsStreamDecoder};

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_storeu_si128};

#[cfg(target_arch = "x86")]
use core::arch::x86::{__m128i, _mm_loadu_si128, _mm_storeu_si128};

/// Internal decode function used by SixelImage::decode
pub(crate) fn decode_sixel(data: &[u8]) -> Result<SixelImage> {
    SixelDecoder::new().decode(data)
}

/// Internal decode function used by SixelImage::decode_from_dcs
pub(crate) fn decode_sixel_from_dcs(payload: &[u8], settings: DcsSettings) -> Result<SixelImage> {
    SixelDecoder::new().decode_from_dcs(payload, settings)
}

/// Decodes a complete ANSI SIXEL sequence.
///
/// This is the main entry point for decoding SIXEL graphics. It handles the full
/// ANSI DCS (Device Control String) sequence format.
///
/// # SIXEL Format
///
/// A complete SIXEL sequence has the format:
/// ```text
/// ESC P <params> q <sixel_data> ESC \
/// ```
/// Where:
/// - `ESC P` (0x1B 0x50): DCS introducer
/// - `<params>`: Optional parameters (aspect ratio, background color, etc.)
/// - `q`: SIXEL command
/// - `<sixel_data>`: The actual SIXEL graphics data
/// - `ESC \` (0x1B 0x5C) or 0x9C: String terminator
///
/// # Parameters
///
/// * `data` - Complete SIXEL sequence as bytes, including DCS introducer and terminator
///
/// # Returns
///
/// Returns a [`SixelImage`] on success.
///
/// Pixel data is returned as RGBA (4 bytes per pixel). When the SIXEL stream requests a
/// transparent background (P2=1), undrawn/background pixels may have `A=0`.
/// - `width`: Image width in pixels
/// - `height`: Image height in pixels
///
/// # Pixel Format
///
/// The returned pixel data is in RGBA format with 4 bytes per pixel:
/// ```text
/// [R₀, G₀, B₀, A₀, R₁, G₁, B₁, A₁, R₂, G₂, B₂, A₂, ...]
/// ```
/// - Total size: `width * height * 4` bytes
/// - Alpha channel is typically 0xFF (fully opaque), but may be 0x00 for undrawn pixels when the SIXEL stream requests transparent background (P2=1)
/// - Pixels are stored in row-major order (left to right, top to bottom)
///
/// To convert to other formats:
/// ```rust
/// # use icy_sixel::sixel_decode;
/// # let sixel_data = b"\x1bPq#0;2;100;0;0#0~~~\x1b\\";
/// let image = sixel_decode(sixel_data)?;
///
/// // Extract RGB (dropping alpha channel)
/// let rgb_pixels: Vec<u8> = image.pixels
///     .chunks(4)
///     .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]])
///     .collect();
///
/// // Access individual pixels
/// for y in 0..image.height {
///     for x in 0..image.width {
///         let idx = (y * image.width + x) * 4;
///         let r = image.pixels[idx];
///         let g = image.pixels[idx + 1];
///         let b = image.pixels[idx + 2];
///         let a = image.pixels[idx + 3];
///     }
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Example
///
/// ```rust
/// use icy_sixel::sixel_decode;
///
/// // Complete SIXEL sequence
/// let sixel_data = b"\x1bPq#0;2;100;0;0#0~~~\x1b\\";
///
/// match sixel_decode(sixel_data) {
///     Ok(image) => {
///         println!("Decoded {}x{} SIXEL image", image.width, image.height);
///         println!("Pixel data: {} bytes (RGBA format)", image.pixels.len());
///         assert_eq!(image.pixels.len(), image.width * image.height * 4);
///         
///         // First pixel color
///         println!("First pixel: R={}, G={}, B={}, A={}",
///                  image.pixels[0], image.pixels[1], image.pixels[2], image.pixels[3]);
///     }
///     Err(e) => eprintln!("Failed to decode: {}", e),
/// }
/// ```
///
/// # Saving as PNG
///
/// ```rust,no_run
/// use icy_sixel::sixel_decode;
/// use image;
///
/// let sixel_data = b"\x1bPq#0;2;100;0;0#0~~~\x1b\\";
/// let image_data = sixel_decode(sixel_data)?;
///
/// // Save as RGBA PNG
/// image::save_buffer(
///     "output.png",
///     &image_data.pixels,
///     image_data.width as u32,
///     image_data.height as u32,
///     image::ColorType::Rgba8,
/// )?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The SIXEL sequence is malformed (missing DCS introducer, invalid syntax)
/// - The resulting image dimensions exceed limits (1,000,000 x 1,000,000)
/// - Memory allocation fails
/// - Invalid color definitions or palette operations
/// - Malformed escape sequences
///
/// # Performance
///
/// This decoder is highly optimized with:
/// - SIMD-accelerated pixel filling (SSE2 on x86/x86_64)
/// - Zero-copy parsing where possible
/// - Minimal memory allocations
/// - Efficient palette caching
///
/// Typical performance: ~3ms to decode a 600x450 image on modern hardware.
///
/// # Example
///
/// ```rust
/// use icy_sixel::sixel_decode;
///
/// let sixel_data = b"\x1bPq#0;2;100;0;0#0~~~\x1b\\";
/// let image = sixel_decode(sixel_data)?;
///
/// println!("Image: {}x{}", image.width, image.height);
/// println!("Aspect ratio: {}:{}", image.aspect_ratio.pan(), image.aspect_ratio.pad());
///
/// // Access pixels (RGBA format, 4 bytes per pixel)
/// let first_pixel = &image.pixels[0..4];
/// # Ok::<(), icy_sixel::SixelError>(())
/// ```
#[deprecated(since = "0.5.0", note = "use SixelImage::decode() instead")]
#[must_use = "this returns the decoded SixelImage"]
pub fn sixel_decode(data: &[u8]) -> Result<SixelImage> {
    SixelImage::decode(data)
}

#[deprecated(since = "0.5.0", note = "use SixelImage::decode_from_dcs() instead")]
#[must_use = "this returns the decoded SixelImage"]
pub fn sixel_decode_from_dcs(payload: &[u8], settings: DcsSettings) -> Result<SixelImage> {
    SixelImage::decode_from_dcs(payload, settings)
}

struct AnsiPayload<'a> {
    aspect_ratio: Option<u16>,
    zero_color: Option<u16>,
    grid_size: Option<u16>,
    payload: &'a [u8],
}

impl<'a> AnsiPayload<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self> {
        let mut idx = 0;
        while idx < bytes.len() {
            match bytes[idx] {
                0x90 => {
                    return Self::parse_dcs(bytes, idx + 1);
                }
                0x1b => {
                    if idx + 1 < bytes.len() && bytes[idx + 1] == b'P' {
                        return Self::parse_dcs(bytes, idx + 2);
                    }
                    idx += 1;
                }
                _ => idx += 1,
            }
        }

        Ok(AnsiPayload {
            aspect_ratio: None,
            zero_color: None,
            grid_size: None,
            payload: bytes,
        })
    }

    fn parse_dcs(bytes: &'a [u8], mut idx: usize) -> Result<Self> {
        let mut header = DcsHeader::new();
        let settings = loop {
            let byte = *bytes.get(idx).ok_or_else(|| SixelError::InvalidData("missing SIXEL DCS command".into()))?;
            idx += 1;
            if let Some(settings) = header.push(byte)? {
                break settings;
            }
        };

        Ok(AnsiPayload {
            aspect_ratio: settings.aspect_ratio,
            zero_color: settings.zero_color,
            grid_size: settings.grid_size,
            // The shared payload parser stops at the first terminator. Avoid scanning
            // the complete image once here and then again while decoding it.
            payload: &bytes[idx..],
        })
    }
}

/// Shared, bounded header parser for both batch and incremental DCS decoding.
struct DcsHeader(Parameters<3>);

impl DcsHeader {
    fn new() -> Self {
        Self(Parameters::new())
    }

    fn push(&mut self, byte: u8) -> Result<Option<DcsSettings>> {
        if is_ignored_control(byte) || self.0.push(byte) {
            return Ok(None);
        }
        if byte == b'q' {
            let params = self.0.finish();
            let param = |index: usize| params.get(index).map(|&value| value.min(i32::from(u16::MAX)) as u16);
            return Ok(Some(DcsSettings::new(param(0), param(1), param(2))));
        }
        if terminates_sixel(byte) {
            return Err(SixelError::InvalidData("malformed SIXEL data".into()));
        }
        Err(SixelError::InvalidData("invalid SIXEL DCS header".into()))
    }
}

#[derive(Clone, Copy, Default)]
pub struct DcsSettings {
    aspect_ratio: Option<u16>,
    #[allow(dead_code)]
    zero_color: Option<u16>,
    grid_size: Option<u16>,
}

impl DcsSettings {
    pub fn new(aspect_ratio: Option<u16>, zero_color: Option<u16>, grid_size: Option<u16>) -> Self {
        Self {
            aspect_ratio,
            zero_color,
            grid_size,
        }
    }

    /// Sets the pixel aspect ratio (P1) using the typed enum.
    #[must_use]
    pub fn with_pixel_aspect_ratio(mut self, aspect_ratio: PixelAspectRatio) -> Self {
        self.aspect_ratio = Some(aspect_ratio.to_p1_value() as u16);
        self
    }

    /// Sets the background mode (P2) using the typed enum.
    #[must_use]
    pub fn with_background_mode(mut self, background_mode: BackgroundMode) -> Self {
        self.zero_color = Some(background_mode.to_p2_value() as u16);
        self
    }

    /// Sets the grid size (P3) raw value.
    #[must_use]
    pub fn with_grid_size(mut self, grid_size: u16) -> Self {
        self.grid_size = Some(grid_size);
        self
    }
}

/// Stateful SIXEL decoder whose color registers can be shared between images.
#[derive(Clone, Debug)]
pub struct SixelDecoder {
    palette: Palette,
}

impl Default for SixelDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl SixelDecoder {
    /// Creates a decoder initialized with the standard SIXEL palette.
    pub fn new() -> Self {
        Self { palette: Palette::new() }
    }

    /// Decodes a complete ANSI DCS sequence while preserving color registers.
    pub fn decode(&mut self, data: &[u8]) -> Result<SixelImage> {
        let parsed = AnsiPayload::parse(data)?;
        let settings = DcsSettings::new(parsed.aspect_ratio, parsed.zero_color, parsed.grid_size);
        let payload = strip_string_terminator(parsed.payload);
        self.decode_from_dcs(payload, settings)
    }

    /// Decodes one DCS payload while preserving color registers on success.
    ///
    /// Opaque undrawn pixels use register 0's color at frame start; later palette
    /// changes affect drawing and subsequent frames, not this frame's background.
    /// In transparent mode (P2=1), undrawn pixels remain transparent.
    /// CAN, SUB, ESC and C1 controls stop decoding, returning the partial image and preceding palette changes.
    pub fn decode_from_dcs(&mut self, payload: &[u8], settings: DcsSettings) -> Result<SixelImage> {
        let mut stream = self.begin_frame(settings)?;
        let _ = stream.feed(payload)?;
        stream.finish()
    }

    /// Starts an incremental SIXEL payload, after the ANSI parser has consumed the DCS header and `q`.
    ///
    /// The session borrows this decoder until finished, aborted or dropped. Only a successful
    /// [`SixelStreamDecoder::finish`] commits its palette; dropping it discards the entire frame.
    #[inline]
    pub fn begin_frame(&mut self, settings: DcsSettings) -> Result<SixelStreamDecoder<'_>> {
        let frame = FrameDecoder::new(settings, self.palette.clone())?;
        Ok(SixelStreamDecoder {
            decoder: self,
            frame: Some(frame),
            settings,
            terminator: None,
        })
    }

    /// Starts an incremental complete DCS sequence, including its introducer, header and ST.
    ///
    /// Input must start with `ESC P` or the 8-bit DCS byte (0x90), not arbitrary terminal text.
    /// Unlike payload streaming, this adapter consumes ST and rejects unfinished sequences
    /// at EOF. See [`SixelDcsStreamDecoder`] for interruption and remainder handling.
    pub fn begin_dcs(&mut self) -> SixelDcsStreamDecoder<'_> {
        SixelDcsStreamDecoder::new(self)
    }

    /// Restores all color registers to the standard SIXEL palette.
    pub fn reset_palette(&mut self) {
        self.palette = Palette::new();
    }
}

/// State returned after feeding a chunk of SIXEL payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SixelFeedStatus {
    /// All input was consumed; a chunk boundary does not finish pending commands.
    NeedMoreData,
    /// A CAN, SUB, ESC or C1 byte ended the payload; the byte remains unconsumed.
    Terminated(u8),
}

/// Progress within the input slice passed to [`SixelStreamDecoder::feed`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct SixelFeedResult {
    /// Bytes consumed from this call's slice, excluding any terminating byte.
    pub consumed: usize,
    /// Whether decoding needs more input or reached a terminating control.
    pub status: SixelFeedStatus,
}

/// Incremental decoder for one SIXEL payload, without a DCS header.
///
/// Input is processed immediately with fixed-size parser state; only the image canvas grows.
/// Existing decoder dimension and canvas limits apply. This is not a progressive image API.
/// A parse error invalidates the session and discards its canvas and uncommitted palette.
///
/// ```rust
/// use icy_sixel::{DcsSettings, SixelDecoder, SixelFeedStatus};
/// let mut decoder = SixelDecoder::new();
/// let mut frame = decoder.begin_frame(DcsSettings::default())?;
/// assert_eq!(frame.feed(b"#1;2;100;0;0!1")?.status, SixelFeedStatus::NeedMoreData);
/// let tail = b"2~\x1b\\remaining terminal data";
/// let progress = frame.feed(tail)?;
/// assert_eq!(progress.consumed, 2);
/// assert_eq!(progress.status, SixelFeedStatus::Terminated(0x1b));
/// let image = frame.finish()?;
/// assert_eq!(image.dimensions(), (12, 6));
/// // Give &tail[progress.consumed..] back to the ANSI parser, including ESC.
/// # Ok::<(), icy_sixel::SixelError>(())
/// ```
#[must_use = "finish the session to obtain an image and commit its palette"]
pub struct SixelStreamDecoder<'a> {
    decoder: &'a mut SixelDecoder,
    frame: Option<FrameDecoder>,
    settings: DcsSettings,
    terminator: Option<u8>,
}

impl SixelStreamDecoder<'_> {
    /// Feeds arbitrary payload chunks; empty chunks do not signal EOF.
    ///
    /// A terminator and all subsequent bytes remain owned by the caller. In particular, ESC
    /// ends the payload immediately, even if the following `\\` arrives in another chunk.
    /// Once terminated, further calls consume zero bytes and return the same status.
    /// On error, no consumed count is returned: discard this payload using the outer ANSI parser.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<SixelFeedResult> {
        let frame = self.frame.as_mut().ok_or_else(|| SixelError::InvalidData("stream decoder has failed".into()))?;
        if let Some(byte) = self.terminator {
            return Ok(SixelFeedResult {
                consumed: 0,
                status: SixelFeedStatus::Terminated(byte),
            });
        }
        match frame.process(bytes) {
            Ok(progress) => {
                if let SixelFeedStatus::Terminated(byte) = progress.status {
                    self.terminator = Some(byte);
                }
                Ok(progress)
            }
            Err(error) => {
                self.frame = None;
                Err(error)
            }
        }
    }

    /// Finishes the frame, including a pending command, and commits its palette on success.
    ///
    /// This also accepts explicit EOF without a terminator, matching the one-shot API.
    /// After CAN/SUB or another terminator it returns the partial image. Call [`Self::abort`]
    /// instead to discard it. Errors, including errors discovered at EOF, never commit a palette.
    pub fn finish(mut self) -> Result<SixelImage> {
        let frame = self.frame.as_mut().ok_or_else(|| SixelError::InvalidData("stream decoder has failed".into()))?;
        frame.finish_command()?;
        let aspect_ratio = frame
            .raster_aspect_ratio
            .unwrap_or_else(|| self.settings.aspect_ratio.map(PixelAspectRatio::from_p1).unwrap_or_default());
        let (pixels, width, height) = frame.finalize()?;

        // Mirrors the decoded pixels: only P2=1 leaves undrawn pixels transparent.
        let background_mode = BackgroundMode::from_p2(self.settings.zero_color.unwrap_or(0));
        // Commit only after finalization succeeds, without moving the whole frame.
        self.decoder.palette.clone_from(&frame.palette);

        Ok(SixelImage {
            pixels,
            width,
            height,
            aspect_ratio,
            background_mode,
        })
    }

    /// Discards this frame and its palette changes, equivalent to dropping the session.
    pub fn abort(self) {}
}

struct FrameDecoder {
    canvas: Canvas,
    palette: Palette,
    color_index: usize,
    current_color: [u8; 4], // RGBA with alpha channel
    repeat: usize,
    pos_x: usize,
    pos_y: usize,
    max_x: usize,
    max_y: usize,
    pan: usize,
    pad: usize,
    /// An explicit raster ratio takes precedence over the DCS P1 macro.
    raster_aspect_ratio: Option<PixelAspectRatio>,
    target_width: usize,
    target_height: usize,
    /// Frame-local background, independent of palette changes and canvas growth.
    background: [u8; 4],
    command: PayloadCommand,
}

enum PayloadCommand {
    Data,
    Repeat(usize),
    Color(Parameters<5>),
    Raster(Parameters<4>),
}

struct Parameters<const N: usize> {
    values: [i32; N],
    count: usize,
    current: i32,
    pending: bool,
}

impl<const N: usize> Parameters<N> {
    fn new() -> Self {
        Self {
            values: [0; N],
            count: 0,
            current: 0,
            pending: false,
        }
    }

    fn push(&mut self, byte: u8) -> bool {
        match byte {
            b'0'..=b'9' => {
                self.current = self.current.saturating_mul(10).saturating_add(i32::from(byte - b'0'));
                self.pending = true;
            }
            b';' => {
                self.store();
                self.pending = true;
            }
            _ => return false,
        }
        true
    }

    fn store(&mut self) {
        if self.count < N {
            self.values[self.count] = self.current;
            self.count += 1;
        }
        self.current = 0;
    }

    fn finish(&mut self) -> &[i32] {
        if self.pending {
            self.store();
        }
        &self.values[..self.count]
    }
}

impl FrameDecoder {
    #[inline]
    fn new(settings: DcsSettings, palette: Palette) -> Result<Self> {
        let repeat = 1usize;
        let current_color = palette.rgb_bytes(0);

        // P2=1 means transparent mode
        let transparent_mode = settings.zero_color == Some(1);

        // In transparent mode, background has alpha=0; otherwise alpha=255
        let background = if transparent_mode {
            [0, 0, 0, 0] // Transparent
        } else {
            palette.rgb_bytes(0)
        };

        let mut decoder = Self {
            canvas: Canvas::new(background),
            palette,
            color_index: 0,
            current_color,
            repeat,
            pos_x: 0,
            pos_y: 0,
            max_x: 0,
            max_y: 0,
            pan: 2,
            pad: 1,
            raster_aspect_ratio: None,
            target_width: 0,
            target_height: 0,
            background,
            command: PayloadCommand::Data,
        };

        decoder.apply_dcs_settings(settings);
        Ok(decoder)
    }

    fn apply_dcs_settings(&mut self, settings: DcsSettings) {
        if let Some(ar) = settings.aspect_ratio {
            self.pad = match ar {
                0 | 1 => 2,
                2 => 5,
                3 | 4 => 4,
                5 | 6 => 3,
                7 | 8 => 2,
                9 => 1,
                _ => self.pad,
            };
        }

        if let Some(mut grid) = settings.grid_size {
            if grid == 0 {
                grid = 10;
            }
            self.pan = (self.pan * grid as usize).max(1) / 10;
            self.pad = (self.pad * grid as usize).max(1) / 10;
            self.pan = self.pan.max(1);
            self.pad = self.pad.max(1);
        }
    }

    fn process(&mut self, data: &[u8]) -> Result<SixelFeedResult> {
        let mut idx = 0usize;
        while idx < data.len() {
            let byte = data[idx];
            if (b'?'..=b'~').contains(&byte) {
                if !matches!(self.command, PayloadCommand::Data) {
                    self.finish_command()?;
                }
                // Drawing cannot change the parser command state within a SIXEL run.
                loop {
                    self.handle_sixel(data[idx])?;
                    idx += 1;
                    if idx == data.len() || !(b'?'..=b'~').contains(&data[idx]) {
                        break;
                    }
                }
                continue;
            }
            if is_ignored_control(byte) {
                idx += 1;
                continue;
            }
            let consumed = match &mut self.command {
                PayloadCommand::Repeat(value) if byte.is_ascii_digit() => {
                    *value = value.saturating_mul(10).saturating_add(usize::from(byte - b'0'));
                    true
                }
                PayloadCommand::Color(params) => params.push(byte),
                PayloadCommand::Raster(params) => params.push(byte),
                _ => false,
            };
            if consumed {
                idx += 1;
                continue;
            }
            if !matches!(self.command, PayloadCommand::Data) {
                self.finish_command()?;
            }
            match byte {
                b'$' => {
                    self.pos_x = 0;
                }
                b'-' => {
                    self.pos_x = 0;
                    self.pos_y = self.pos_y.checked_add(SIXEL_CELL_HEIGHT).ok_or(SixelError::IntegerOverflow)?;
                }
                b'!' => self.command = PayloadCommand::Repeat(0),
                b'#' => self.command = PayloadCommand::Color(Parameters::new()),
                b'"' => self.command = PayloadCommand::Raster(Parameters::new()),
                byte if terminates_sixel(byte) => {
                    return Ok(SixelFeedResult {
                        consumed: idx,
                        status: SixelFeedStatus::Terminated(byte),
                    })
                }
                _ => {}
            }
            idx += 1;
        }
        Ok(SixelFeedResult {
            consumed: idx,
            status: SixelFeedStatus::NeedMoreData,
        })
    }

    fn finish_command(&mut self) -> Result<()> {
        match std::mem::replace(&mut self.command, PayloadCommand::Data) {
            PayloadCommand::Data => {}
            PayloadCommand::Repeat(value) => {
                if value > SIXEL_REPEAT_MAX {
                    return Err(SixelError::InvalidData("malformed SIXEL data".to_string()));
                }
                self.repeat = value.max(1);
            }
            PayloadCommand::Color(mut params) => self.handle_color_command(params.finish())?,
            PayloadCommand::Raster(mut params) => self.handle_raster_command(params.finish())?,
        }
        Ok(())
    }

    #[inline]
    fn handle_sixel(&mut self, ch: u8) -> Result<()> {
        let bits = ch - b'?';
        let span = self.repeat.max(1);
        self.repeat = 1;

        let width_needed = self.pos_x + span;
        let height_needed = self.pos_y + SIXEL_CELL_HEIGHT;

        // Quick overflow check
        if width_needed > SIXEL_WIDTH_LIMIT || height_needed > SIXEL_HEIGHT_LIMIT {
            return Err(SixelError::InvalidData("malformed SIXEL data".to_string()));
        }

        if width_needed > self.canvas.width || height_needed > self.canvas.height {
            self.canvas.ensure_visible(width_needed, height_needed, self.background)?;
        }

        // Use cached color for performance
        let color = self.current_color;
        let mut touched = false;

        // Unroll loop - process all 6 bits
        if (bits & 0b000001) != 0 {
            self.canvas.paint_span(self.pos_y, self.pos_x, span, color);
            touched = true;
        }
        if (bits & 0b000010) != 0 {
            self.canvas.paint_span(self.pos_y + 1, self.pos_x, span, color);
            touched = true;
        }
        if (bits & 0b000100) != 0 {
            self.canvas.paint_span(self.pos_y + 2, self.pos_x, span, color);
            touched = true;
        }
        if (bits & 0b001000) != 0 {
            self.canvas.paint_span(self.pos_y + 3, self.pos_x, span, color);
            touched = true;
        }
        if (bits & 0b010000) != 0 {
            self.canvas.paint_span(self.pos_y + 4, self.pos_x, span, color);
            touched = true;
        }
        if (bits & 0b100000) != 0 {
            self.canvas.paint_span(self.pos_y + 5, self.pos_x, span, color);
            touched = true;
        }

        if span > 0 {
            let last_x = self.pos_x + span - 1;
            if last_x > self.max_x {
                self.max_x = last_x;
            }
        }

        if touched {
            let last_y = self.pos_y + SIXEL_CELL_HEIGHT - 1;
            if last_y > self.max_y {
                self.max_y = last_y;
            }
        }

        self.pos_x = width_needed;
        Ok(())
    }

    fn handle_color_command(&mut self, params: &[i32]) -> Result<()> {
        if params.is_empty() {
            self.color_index = 0;
            self.current_color = self.palette.rgb_bytes(0);
            return Ok(());
        }

        let color_idx = params[0].max(0) as usize;
        self.color_index = color_idx.min(SIXEL_PALETTE_MAX - 1);
        self.current_color = self.palette.rgb_bytes(self.color_index);

        if params.len() >= 5 {
            let colorspace = params[1];
            match colorspace {
                1 => {
                    self.palette.set_hls(self.color_index, params[2], params[3], params[4]);
                    self.current_color = self.palette.rgb_bytes(self.color_index);
                }
                2 => {
                    self.palette.set_rgb_percent(self.color_index, params[2], params[3], params[4]);
                    self.current_color = self.palette.rgb_bytes(self.color_index);
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_raster_command(&mut self, storage: &[i32]) -> Result<()> {
        let count = storage.len();
        if count > 0 {
            let pad = storage[0].max(1) as usize;
            self.pad = pad;
        }
        if count > 1 {
            let pan = storage[1].max(1) as usize;
            self.pan = pan;
            // Raster parameters are vertical:horizontal. Normalize without
            // multiplication so large parameter values cannot overflow.
            // Ratios outside the public enum fall back to the DCS metadata.
            self.raster_aspect_ratio = match (self.pad / self.pan, self.pad % self.pan) {
                (1, 0) => Some(PixelAspectRatio::Square),
                (2, 0) => Some(PixelAspectRatio::Ratio2To1),
                (3, 0) => Some(PixelAspectRatio::Ratio3To1),
                (5, 0) => Some(PixelAspectRatio::Ratio5To1),
                _ => None,
            };
        }
        if count > 2 {
            let ph = storage[2].max(0) as usize;
            if ph > 0 {
                self.target_width = ph;
            }
        }
        if count > 3 {
            let pv = storage[3].max(0) as usize;
            if pv > 0 {
                self.target_height = pv;
            }
        }

        if self.target_width > 0 || self.target_height > 0 {
            let width = self.target_width.max(1);
            let height = self.target_height.max(1);
            self.guard_dimensions(width, height)?;
            self.canvas.ensure_visible(width, height, self.background)?;
        }

        Ok(())
    }

    fn guard_dimensions(&self, width: usize, height: usize) -> Result<()> {
        if width > SIXEL_WIDTH_LIMIT || height > SIXEL_HEIGHT_LIMIT {
            return Err(SixelError::InvalidData("malformed SIXEL data".to_string()));
        }
        if width.saturating_mul(height) > MAX_PIXELS {
            return Err(SixelError::InvalidData("image dimensions too large".to_string()));
        }
        Ok(())
    }

    fn finalize(&mut self) -> Result<(Vec<u8>, usize, usize)> {
        let width = self.max_x + 1;
        let height = self.max_y + 1;
        let desired_width = width.max(self.target_width.max(1));
        let desired_height = height.max(self.target_height.max(1));
        self.guard_dimensions(desired_width, desired_height)?;
        self.canvas.ensure_visible(desired_width, desired_height, self.background)?;
        let (width, height) = (self.canvas.width, self.canvas.height);
        Ok((self.canvas.take_pixels(), width, height))
    }
}

#[derive(Clone, Debug)]
struct Palette {
    colors: [u32; SIXEL_PALETTE_MAX],
}

impl Palette {
    fn new() -> Self {
        let mut colors = [0u32; SIXEL_PALETTE_MAX];
        const BASE: &[(i32, i32, i32)] = &[
            (0, 0, 0),
            (20, 20, 80),
            (80, 13, 13),
            (20, 80, 20),
            (80, 20, 80),
            (20, 80, 80),
            (80, 80, 20),
            (53, 53, 53),
            (26, 26, 26),
            (33, 33, 60),
            (60, 26, 26),
            (33, 60, 33),
            (60, 33, 60),
            (33, 60, 60),
            (60, 60, 33),
            (80, 80, 80),
        ];

        for (idx, &(r, g, b)) in BASE.iter().enumerate() {
            colors[idx] = pack_rgb(percent_to_byte(r), percent_to_byte(g), percent_to_byte(b));
        }

        let mut cursor = BASE.len();
        for r in 0..6 {
            for g in 0..6 {
                for b in 0..6 {
                    let red = percent_to_byte(r * 20);
                    let green = percent_to_byte(g * 20);
                    let blue = percent_to_byte(b * 20);
                    if cursor < SIXEL_PALETTE_MAX {
                        colors[cursor] = pack_rgb(red, green, blue);
                    }
                    cursor += 1;
                }
            }
        }

        for level in 0..24 {
            if cursor >= SIXEL_PALETTE_MAX {
                break;
            }
            let value = percent_to_byte(level * 100 / 23);
            colors[cursor] = pack_rgb(value, value, value);
            cursor += 1;
        }

        while cursor < SIXEL_PALETTE_MAX {
            colors[cursor] = 0x00ffffff;
            cursor += 1;
        }

        Self { colors }
    }

    fn rgb_bytes(&self, index: usize) -> [u8; 4] {
        let color = self.colors[index.min(SIXEL_PALETTE_MAX - 1)];
        [
            ((color >> 16) & 0xff) as u8,
            ((color >> 8) & 0xff) as u8,
            (color & 0xff) as u8,
            0xFF, // Alpha channel
        ]
    }

    fn set_rgb_percent(&mut self, index: usize, r: i32, g: i32, b: i32) {
        let red = percent_to_byte(r);
        let green = percent_to_byte(g);
        let blue = percent_to_byte(b);
        if index < SIXEL_PALETTE_MAX {
            self.colors[index] = pack_rgb(red, green, blue);
        }
    }

    fn set_hls(&mut self, index: usize, h: i32, l: i32, s: i32) {
        if index >= SIXEL_PALETTE_MAX {
            return;
        }
        let rgb = hls_to_rgb(h, l, s);
        self.colors[index] = pack_rgb(rgb[0], rgb[1], rgb[2]);
    }
}

struct Canvas {
    data: Vec<u8>,
    width: usize,
    height: usize,
    /// Allocated row width in pixels; may exceed `width` to amortize growth.
    stride: usize,
    /// Allocated row count; may exceed `height` to amortize growth.
    capacity_height: usize,
}

impl Canvas {
    fn new(background: [u8; 4]) -> Self {
        let mut data = vec![0u8; 4];
        data[..4].copy_from_slice(&background);
        Self {
            data,
            width: 1,
            height: 1,
            stride: 1,
            capacity_height: 1,
        }
    }

    fn ensure_visible(&mut self, width: usize, height: usize, background: [u8; 4]) -> Result<()> {
        if width <= self.width && height <= self.height {
            return Ok(());
        }

        let new_width = width.max(self.width).max(1);
        let new_height = height.max(self.height).max(1);

        if new_width.saturating_mul(new_height) > MAX_PIXELS {
            return Err(SixelError::InvalidData("image dimensions too large".to_string()));
        }

        if new_width > self.stride || new_height > self.capacity_height {
            self.grow_capacity(new_width, new_height);
        }

        self.expose(new_width, new_height, background);
        Ok(())
    }

    /// Reallocates with geometric slack so repeated single-column growth stays linear overall.
    fn grow_capacity(&mut self, new_width: usize, new_height: usize) {
        let mut stride = self.stride.max(1);
        while stride < new_width {
            stride = stride.saturating_mul(2);
        }
        let mut capacity_height = self.capacity_height.max(1);
        while capacity_height < new_height {
            capacity_height = capacity_height.saturating_mul(2);
        }

        if stride.saturating_mul(capacity_height) > MAX_PIXELS {
            stride = new_width;
            capacity_height = new_height;
        }

        let mut data = vec![0u8; stride * capacity_height * 4];
        let row_bytes = self.width * 4;
        for row in 0..self.height {
            let src = row * self.stride * 4;
            let dst = row * stride * 4;
            data[dst..dst + row_bytes].copy_from_slice(&self.data[src..src + row_bytes]);
        }

        self.data = data;
        self.stride = stride;
        self.capacity_height = capacity_height;
    }

    /// Fills the area newly uncovered by a logical resize with the background color.
    fn expose(&mut self, new_width: usize, new_height: usize, background: [u8; 4]) {
        if new_width > self.width {
            let start = self.width * 4;
            let end = new_width * 4;
            for row in 0..self.height {
                let base = row * self.stride * 4;
                fill_rgba_span(&mut self.data[base + start..base + end], background);
            }
        }

        for row in self.height..new_height {
            let base = row * self.stride * 4;
            fill_rgba_span(&mut self.data[base..base + new_width * 4], background);
        }

        self.width = new_width;
        self.height = new_height;
    }

    /// Returns tightly packed RGBA rows, dropping any unused capacity padding.
    fn take_pixels(&mut self) -> Vec<u8> {
        let row_bytes = self.width * 4;
        if self.stride == self.width {
            self.data.truncate(row_bytes * self.height);
            return std::mem::take(&mut self.data);
        }

        let mut out = vec![0u8; row_bytes * self.height];
        for row in 0..self.height {
            let src = row * self.stride * 4;
            let dst = row * row_bytes;
            out[dst..dst + row_bytes].copy_from_slice(&self.data[src..src + row_bytes]);
        }
        out
    }

    // Avoid up to six out-of-line calls per SIXEL in the larger streaming parser.
    #[inline(always)]
    fn paint_span(&mut self, y: usize, x: usize, len: usize, color: [u8; 4]) {
        if len == 0 || y >= self.height || x >= self.width {
            return;
        }
        // Clip the span to the available width
        let available = self.width - x;
        let actual_len = len.min(available);
        let start = (y * self.stride + x) * 4;

        // Fast path for single pixel
        if actual_len == 1 {
            unsafe {
                let ptr = self.data.as_mut_ptr().add(start);
                *ptr = color[0];
                *ptr.add(1) = color[1];
                *ptr.add(2) = color[2];
                *ptr.add(3) = color[3];
            }
            return;
        }

        let end = start + actual_len * 4;
        fill_rgba_span(&mut self.data[start..end], color);
    }
}

fn strip_string_terminator(data: &[u8]) -> &[u8] {
    if data.ends_with(b"\x1b\\") {
        &data[..data.len() - 2]
    } else if data.last() == Some(&0x9c) {
        &data[..data.len() - 1]
    } else {
        data
    }
}

#[inline]
fn terminates_sixel(byte: u8) -> bool {
    matches!(byte, 0x18 | 0x1a | 0x1b | 0x80..=0x9f)
}

#[inline]
fn is_ignored_control(byte: u8) -> bool {
    matches!(byte, 0x00..=0x17 | 0x19 | 0x1c..=0x1f | 0x7f)
}

fn percent_to_byte(value: i32) -> u8 {
    let clamped = value.clamp(0, 100);
    ((clamped * 255 + 50) / 100) as u8
}

fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

fn hls_to_rgb(h: i32, l: i32, s: i32) -> [u8; 3] {
    if s <= 0 {
        let gray = percent_to_byte(l);
        return [gray, gray, gray];
    }

    // Normalize before applying SIXEL's hue offset: parsed values may have
    // saturated at i32::MAX, so adding the offset first could overflow.
    let hue = (h.rem_euclid(360) + 240) % 360;
    let hue = hue as f64 / 360.0;
    let lum = (l.clamp(0, 100) as f64) / 100.0;
    let sat = (s.clamp(0, 100) as f64) / 100.0;

    let q = if lum < 0.5 { lum * (1.0 + sat) } else { lum + sat - lum * sat };
    let p = 2.0 * lum - q;

    let r = hue_to_rgb(p, q, hue + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, hue);
    let b = hue_to_rgb(p, q, hue - 1.0 / 3.0);

    [
        (r * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8,
        (g * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8,
        (b * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8,
    ]
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

fn fill_rgba_span(buf: &mut [u8], color: [u8; 4]) {
    if buf.is_empty() {
        return;
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        if try_fill_rgba_span_simd(buf, color) {
            return;
        }
    }

    fill_rgba_span_scalar(buf, color);
}

fn fill_rgba_span_scalar(buf: &mut [u8], color: [u8; 4]) {
    let len = buf.len();
    if len <= 4 {
        for (idx, byte) in buf.iter_mut().enumerate() {
            *byte = color[idx % 4];
        }
        return;
    }

    buf[..4].copy_from_slice(&color);
    let mut written = 4;
    while written < len {
        let copy = (len - written).min(written);
        let src = buf[..copy].as_ptr();
        unsafe {
            std::ptr::copy_nonoverlapping(src, buf[written..].as_mut_ptr(), copy);
        }
        written += copy;
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
fn try_fill_rgba_span_simd(buf: &mut [u8], color: [u8; 4]) -> bool {
    if buf.len() < 64 {
        return false;
    }

    #[cfg(target_arch = "x86")]
    {
        if !std::is_x86_feature_detected!("sse2") {
            return false;
        }
    }

    unsafe { fill_rgba_span_sse(buf, color) };
    true
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
unsafe fn fill_rgba_span_sse(buf: &mut [u8], color: [u8; 4]) {
    let mut pattern = [0u8; 16];
    for idx in 0..16 {
        pattern[idx] = color[idx % 4];
    }

    let vec = _mm_loadu_si128(pattern.as_ptr() as *const __m128i);
    let mut chunks = buf.chunks_exact_mut(16);
    for chunk in &mut chunks {
        // Each chunk contains exactly 16 writable bytes. No pointer past the
        // allocation is computed, including when the final chunk ends there.
        _mm_storeu_si128(chunk.as_mut_ptr() as *mut __m128i, vec);
    }
    let remainder = chunks.into_remainder();
    remainder.copy_from_slice(&pattern[..remainder.len()]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_fill_handles_unaligned_slices_and_tails() {
        for len in (0..=129).chain([255, 256, 257]) {
            for offset in 0..4 {
                for color in [[0, 0, 0, 0], [17, 83, 149, 255]] {
                    let expected: Vec<_> = (0..len).map(|i| color[i % 4]).collect();
                    let mut scalar = vec![0xa5; len];
                    fill_rgba_span_scalar(&mut scalar, color);
                    assert_eq!(scalar, expected);

                    // Boxed slices end exactly at the allocation boundary, with
                    // no spare capacity to hide an out-of-bounds pointer add.
                    let mut data = vec![0xa5; offset + len].into_boxed_slice();
                    fill_rgba_span(&mut data[offset..], color);
                    assert_eq!(&data[..offset], vec![0xa5; offset].as_slice());
                    assert_eq!(&data[offset..], expected);
                }
            }
        }
    }
}
