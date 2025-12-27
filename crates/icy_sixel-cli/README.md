# icy_sixel-cli

A command-line tool for encoding, decoding, and animating SIXEL graphics.

SIXEL is a bitmap graphics format supported by many terminal emulators including xterm, mlterm, foot, WezTerm, and others.

## Installation

```bash
cargo install icy_sixel-cli
```

Or build from source:

```bash
git clone https://github.com/mkrueger/icy_sixel
cd icy_sixel
cargo build --release -p icy_sixel-cli
```

## Commands

### Encode

Convert images (PNG, JPEG, GIF, WebP) to SIXEL format:

```bash
# Display image in terminal
sixel encode image.png

# Save to file
sixel encode image.png -o output.six

# Read from stdin
cat image.png | sixel encode > output.six

# Custom settings
sixel encode image.png --colors 64 --diffusion 0.5 --method kmeans
```

### Decode

Convert SIXEL files back to PNG:

```bash
# Decode to PNG (auto-generates output filename)
sixel decode image.six

# Specify output file
sixel decode image.six -o output.png

# Read from stdin
cat image.six | sixel decode -o output.png
```

### Animate

Play or convert animated GIFs:

```bash
# Play animation in terminal (Ctrl+C to stop)
sixel animate animation.gif

# Adjust playback speed (2x faster)
sixel animate animation.gif --speed 2.0

# Limit loop count
sixel animate animation.gif --loops 3

# Extract a single frame
sixel animate animation.gif --frame 5 -o frame5.six

# Save all frames to file
sixel animate animation.gif -o animation.six
```

## Options

### Global Options

| Option | Description |
|--------|-------------|
| `-q, --quiet` | Suppress informational messages (errors still shown) |
| `-h, --help` | Print help information |
| `-V, --version` | Print version |

### Encode Options

| Option | Default | Description |
|--------|---------|-------------|
| `-o, --output <FILE>` | stdout | Output file |
| `-c, --colors <N>` | 256 | Maximum colors (2-256) |
| `-d, --diffusion <F>` | 0.875 | Floyd-Steinberg dithering strength (0.0-1.0) |
| `-m, --method <METHOD>` | wu | Color quantization: `wu` or `kmeans` |
| `-a, --aspect-ratio <RATIO>` | square | Pixel aspect ratio (see below) |
| `-b, --background <MODE>` | transparent | Background mode: `transparent` or `opaque` |

### Animate Options

All encode options plus:

| Option | Default | Description |
|--------|---------|-------------|
| `-s, --speed <F>` | 1.0 | Speed multiplier |
| `-l, --loops <N>` | 0 | Loop count (0=GIF default, -1=infinite) |
| `-f, --frame <N>` | - | Extract single frame (0-indexed) |

### Pixel Aspect Ratios

| Value | Ratio | Description |
|-------|-------|-------------|
| `square` | 1:1 | Modern terminals (default) |
| `ratio2to1` | 2:1 | VT240/VT340 native |
| `ratio3to1` | 3:1 | Tall pixels |
| `ratio5to1` | 5:1 | Very tall pixels |

## Examples

```bash
# High-quality encode with maximum colors
sixel encode photo.jpg --colors 256 --diffusion 0.875

# Fast preview with fewer colors
sixel encode image.png --colors 16 --diffusion 0

# K-means quantization (slower but may be more accurate)
sixel encode image.png --method kmeans

# Opaque background (fills undrawn pixels)
sixel encode image.png --background opaque

# Silent operation for scripts
sixel -q encode image.png > output.six

# Slow-motion GIF playback
sixel animate animation.gif --speed 0.5

# Convert GIF to static SIXEL (first frame)
sixel animate animation.gif --frame 0 > still.six
```

## Supported Formats

**Input (encode/animate):**
- PNG
- JPEG
- GIF (static and animated)
- WebP

**Output (decode):**
- PNG

## Terminal Compatibility

SIXEL is supported by:
- xterm (with `+sixel` build option)
- mlterm
- foot
- WezTerm
- Contour
- ctx
- And many more

Test if your terminal supports SIXEL:
```bash
echo -e '\eP0;0;0q"1;1;1;1#0;2;100;0;0#0~-\e\\'
```

You should see a small red dot if SIXEL is supported.

## Related

- [icy_sixel](https://crates.io/crates/icy_sixel) - The underlying Rust library
- [libsixel](https://github.com/saitoha/libsixel) - The original C implementation
- [All About SIXELs](https://www.digiater.nl/openvms/decus/vax90b1/krypton-strng/all-about-sixels.text) - SIXEL specification

## License

Licensed under the Apache License, Version 2.0 — see [LICENSE](../../LICENSE-APACHE) for details.
