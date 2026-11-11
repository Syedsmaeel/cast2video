# cast2video (Rust Crate & CLI)

A professional, high-performance Rust library and CLI tool for converting asciinema session recordings (`.cast` files) or URLs into production-ready **MP4** (H.264) or **WebM** (VP9) videos.

---

## Architecture Overview

```
                      ┌──────────────────────┐
                      │  .cast File or URL   │
                      └──────────┬───────────┘
                                 │
                     ┌───────────▼───────────┐
                     │ agg (asciinema engine)│ (renders typography, themes, Nerd fonts)
                     └───────────┬───────────┘
                                 │ intermediate frame stream
                     ┌───────────▼───────────┐
                     │   FFmpeg Transcoder   │ (H.264/yuv420p + faststart or VP9 WebM)
                     └───────────┬───────────┘
                                 │
                     ┌───────────▼───────────┐
                     │   .mp4 / .webm Video  │
                     └───────────────────────┘
```

---

## 1. Using as a Rust Library

Add `cast2video` to your `Cargo.toml`:

```toml
[dependencies]
cast2video = { path = "/home/null/cast2video" }
```

### Basic Example

```rust
use cast2video::{Converter, ConvertOptions, OutputFormat, Theme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let converter = Converter::new()?;

    let options = ConvertOptions::builder("recording.cast")
        .output("recording.mp4")
        .format(OutputFormat::Mp4)
        .theme(Theme::Dracula)
        .speed(1.5)
        .scale(2)                // 2x Retina crisp text upscale
        .idle_time_limit(Some(2.0)) // Cap long pauses
        .build();

    let result = converter.convert(&options)?;

    println!("Saved to {:?}", result.output_path);
    println!("File size: {}", result.formatted_output_size());
    println!("Duration:  {:?}", result.formatted_duration());
    Ok(())
}
```

### Multi-threaded Batch Processing with Rayon

```rust
use cast2video::{Converter, ConvertOptions, OutputFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let converter = Converter::new()?;

    let batch = vec![
        ConvertOptions::builder("demo1.cast").output("demo1.mp4").build(),
        ConvertOptions::builder("demo2.cast").output("demo2.mp4").build(),
        ConvertOptions::builder("demo3.cast").output("demo3.mp4").build(),
    ];

    // Runs conversions across all CPU cores in parallel via Rayon
    let results = converter.convert_batch(&batch, /* parallel */ true);

    for res in results {
        match res {
            Ok(c) => println!("✔ Completed: {:?}", c.output_path),
            Err(e) => eprintln!("✖ Failed: {}", e),
        }
    }
    Ok(())
}
```

---

## 2. Using as a CLI Tool

The native binary `cast2video-rs` is installed into `~/.local/bin/cast2video-rs`.

### Usage Examples

```bash
# Convert to MP4 with default settings (Dracula theme, 30fps)
cast2video-rs my_session.cast

# Convert to WebM with 1.5x speed and Nord theme
cast2video-rs my_session.cast -o demo.webm --theme nord --speed 1.5

# High-Res 2x Retina scale with 2-second max idle pause
cast2video-rs my_session.cast -o demo.mp4 --scale 2 --idle-time-limit 2.0

# Batch convert multiple files in parallel using all CPU cores
cast2video-rs ./recordings/*.cast -o ./videos/

# Limit parallel workers (e.g. 4 jobs)
cast2video-rs ./recordings/*.cast -o ./videos/ -j 4

# Convert directly from asciinema.org URL
cast2video-rs https://asciinema.org/a/335480 -o asciinema_demo.mp4
```

### CLI Options

| Flag | Default | Description |
| :--- | :--- | :--- |
| `inputs` | *(required)* | One or more `.cast` files, globs, directories, or asciinema URLs |
| `-o, --output` | auto (`.mp4`) | Output file path (single file) or destination folder (batch) |
| `-f, --format` | `mp4` | Video format: `mp4`, `webm`, or `gif` |
| `--theme` | `dracula` | Color theme (`dracula`, `nord`, `monokai`, `kanagawa`, `github-dark`, etc.) |
| `--speed` | `1.0` | Playback speed multiplier (e.g. `1.5`) |
| `--idle-time-limit` | `2.0` | Max pause duration in seconds (`0` disables) |
| `--scale` | `1` | Integer scaling multiplier (`1`, `2`, `3`, `4`) for crisp HiDPI text |
| `--fps` | `30` | Frame rate cap (e.g. `30` or `60`) |
| `--crf` | `20` / `28` | Constant Rate Factor (lower = higher quality) |
| `--preset` | `medium` | H.264 speed preset (`fast`, `medium`, `slow`) |
| `-j, --jobs` | CPU cores | Number of parallel worker threads for batch conversions |
| `--sequential` | `false` | Force sequential execution for batch processing |
| `--dry-run` | `false` | Print exact underlying commands without running them |
| `-v, --verbose` | `false` | Display diagnostic logs from `agg` and `ffmpeg` |
| `-q, --quiet` | `false` | Suppress non-essential output |

---

## 3. System Requirements

* `agg` (asciinema gif generator): [https://github.com/asciinema/agg](https://github.com/asciinema/agg)
* `ffmpeg`: [https://ffmpeg.org](https://ffmpeg.org)
