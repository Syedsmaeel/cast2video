use crate::config::ConvertOptions;
use crate::error::{CastError, Result};
use crate::types::{ConversionResult, OutputFormat, VideoMetadata};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

/// Core converter struct managing binary paths and orchestrating video rendering.
#[derive(Debug, Clone)]
pub struct Converter {
    pub agg_path: PathBuf,
    pub ffmpeg_path: PathBuf,
    pub ffprobe_path: Option<PathBuf>,
}

impl Converter {
    /// Detects `agg` and `ffmpeg` in the system PATH and initializes the Converter.
    pub fn new() -> Result<Self> {
        let agg_path = which::which("agg").map_err(|_| CastError::MissingDependency {
            binary: "agg".to_string(),
            advice: "Install agg (asciinema gif generator):\n  curl -sL https://github.com/asciinema/agg/releases/latest/download/agg-x86_64-unknown-linux-musl -o ~/.local/bin/agg && chmod +x ~/.local/bin/agg".to_string(),
        })?;

        let ffmpeg_path = which::which("ffmpeg").map_err(|_| CastError::MissingDependency {
            binary: "ffmpeg".to_string(),
            advice: "Install FFmpeg:\n  Ubuntu/Debian: sudo apt install ffmpeg\n  macOS: brew install ffmpeg".to_string(),
        })?;

        let ffprobe_path = which::which("ffprobe").ok();

        Ok(Self {
            agg_path,
            ffmpeg_path,
            ffprobe_path,
        })
    }

    /// Explicitly configure the executable paths for `agg`, `ffmpeg`, and `ffprobe`.
    pub fn with_paths<P1: AsRef<Path>, P2: AsRef<Path>>(
        agg_path: P1,
        ffmpeg_path: P2,
        ffprobe_path: Option<PathBuf>,
    ) -> Self {
        Self {
            agg_path: agg_path.as_ref().to_path_buf(),
            ffmpeg_path: ffmpeg_path.as_ref().to_path_buf(),
            ffprobe_path,
        }
    }

    /// Converts a single `.cast` file or URL according to `options`.
    pub fn convert(&self, options: &ConvertOptions) -> Result<ConversionResult> {
        let start_time = Instant::now();

        // 1. Verify input exists if it's a local file
        let input_path = if !options.is_url() {
            let p = Path::new(&options.input);
            if !p.exists() {
                return Err(CastError::InputNotFound(p.to_path_buf()));
            }
            Some(p)
        } else {
            None
        };

        let input_size_bytes = input_path.and_then(|p| p.metadata().ok()).map(|m| m.len());

        // Ensure output parent directory exists
        if let Some(parent) = options.output_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|_| CastError::OutputDirectoryFailed(parent.to_path_buf()))?;
            }
        }

        // 2. Set up temporary directory for intermediate GIF rendering
        let temp_dir = tempfile::Builder::new()
            .prefix("cast2video_")
            .tempdir()?;
        let intermediate_gif = temp_dir.path().join("rendered.gif");

        // 3. Assemble agg command
        let mut agg_cmd = Command::new(&self.agg_path);
        agg_cmd.arg("--theme").arg(options.theme.as_str());

        if (options.speed - 1.0).abs() > 0.001 {
            agg_cmd.arg("--speed").arg(options.speed.to_string());
        }

        if let Some(idle) = options.idle_time_limit {
            if idle > 0.0 {
                agg_cmd.arg("--idle-time-limit").arg(idle.to_string());
            }
        }

        if options.fps_cap > 0 {
            agg_cmd.arg("--fps-cap").arg(options.fps_cap.to_string());
        }

        if let Some(ref font) = options.font_family {
            agg_cmd.arg("--font-family").arg(font);
        }

        if let Some(size) = options.font_size {
            agg_cmd.arg("--font-size").arg(size.to_string());
        }

        if let Some(lh) = options.line_height {
            agg_cmd.arg("--line-height").arg(lh.to_string());
        }

        if let Some(cols) = options.cols {
            agg_cmd.arg("--cols").arg(cols.to_string());
        }

        if let Some(rows) = options.rows {
            agg_cmd.arg("--rows").arg(rows.to_string());
        }

        if options.quiet {
            agg_cmd.arg("--quiet");
        } else if options.verbose {
            agg_cmd.arg("-v");
        }

        agg_cmd.arg(&options.input).arg(&intermediate_gif);

        if options.dry_run {
            println!("[DRY-RUN agg] {:?}", agg_cmd);
        } else {
            if !options.verbose {
                agg_cmd.stdout(Stdio::null()).stderr(Stdio::piped());
            }

            let output = agg_cmd.output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                return Err(CastError::AggFailed {
                    status: output.status.code(),
                    stderr,
                });
            }

            if !intermediate_gif.exists() {
                return Err(CastError::IntermediateFileMissing(intermediate_gif));
            }
        }

        // 4. Assemble FFmpeg video filters
        let mut filters = Vec::new();
        if options.scale > 1 {
            filters.push(format!("scale=iw*{}:ih*{}:flags=neighbor", options.scale, options.scale));
        }
        // Essential padding for H.264 (width & height must be even)
        filters.push("pad=ceil(iw/2)*2:ceil(ih/2)*2".to_string());
        let vf_arg = filters.join(",");

        let mut ff_cmd = Command::new(&self.ffmpeg_path);
        ff_cmd
            .arg("-y")
            .arg("-hide_banner")
            .arg("-i")
            .arg(&intermediate_gif)
            .arg("-vf")
            .arg(&vf_arg);

        if !options.verbose {
            ff_cmd.arg("-loglevel").arg("warning");
        }

        let crf = options.effective_crf();

        match options.format {
            OutputFormat::Mp4 => {
                ff_cmd
                    .arg("-c:v")
                    .arg("libx264")
                    .arg("-preset")
                    .arg(options.preset.as_str())
                    .arg("-crf")
                    .arg(crf.to_string())
                    .arg("-pix_fmt")
                    .arg("yuv420p")
                    .arg("-movflags")
                    .arg("+faststart")
                    .arg(&options.output_path);
            }
            OutputFormat::WebM => {
                ff_cmd
                    .arg("-c:v")
                    .arg("libvpx-vp9")
                    .arg("-b:v")
                    .arg("0")
                    .arg("-crf")
                    .arg(crf.to_string())
                    .arg("-pix_fmt")
                    .arg("yuva420p")
                    .arg(&options.output_path);
            }
            OutputFormat::Gif => {
                // If the target format is GIF, simply copy the generated intermediate
                if options.dry_run {
                    println!("[DRY-RUN] copy intermediate GIF to {:?}", options.output_path);
                    return Ok(ConversionResult {
                        input: options.input.clone(),
                        output_path: options.output_path.clone(),
                        format: OutputFormat::Gif,
                        output_size_bytes: 0,
                        input_size_bytes,
                        metadata: None,
                        elapsed: start_time.elapsed(),
                    });
                }
                fs::copy(&intermediate_gif, &options.output_path)?;
                let output_size_bytes = fs::metadata(&options.output_path)?.len();
                return Ok(ConversionResult {
                    input: options.input.clone(),
                    output_path: options.output_path.clone(),
                    format: OutputFormat::Gif,
                    output_size_bytes,
                    input_size_bytes,
                    metadata: None,
                    elapsed: start_time.elapsed(),
                });
            }
        }

        if options.dry_run {
            println!("[DRY-RUN ffmpeg] {:?}", ff_cmd);
            return Ok(ConversionResult {
                input: options.input.clone(),
                output_path: options.output_path.clone(),
                format: options.format,
                output_size_bytes: 0,
                input_size_bytes,
                metadata: None,
                elapsed: start_time.elapsed(),
            });
        }

        if !options.verbose {
            ff_cmd.stdout(Stdio::null()).stderr(Stdio::piped());
        }

        let ff_output = ff_cmd.output()?;
        if !ff_output.status.success() {
            let stderr = String::from_utf8_lossy(&ff_output.stderr).to_string();
            return Err(CastError::FfmpegFailed {
                status: ff_output.status.code(),
                stderr,
            });
        }

        // 5. Preserve intermediate GIF if requested
        if options.keep_intermediate {
            let preserved_gif = options.output_path.with_extension("gif");
            let _ = fs::copy(&intermediate_gif, preserved_gif);
        }

        // 6. Gather result statistics
        let output_size_bytes = fs::metadata(&options.output_path)?.len();
        let metadata = self.probe_metadata(&options.output_path);

        Ok(ConversionResult {
            input: options.input.clone(),
            output_path: options.output_path.clone(),
            format: options.format,
            output_size_bytes,
            input_size_bytes,
            metadata,
            elapsed: start_time.elapsed(),
        })
    }

    /// Convert a slice of options in parallel (via Rayon) or sequentially.
    pub fn convert_batch(
        &self,
        batch_options: &[ConvertOptions],
        parallel: bool,
    ) -> Vec<Result<ConversionResult>> {
        if parallel {
            batch_options.par_iter().map(|opts| self.convert(opts)).collect()
        } else {
            batch_options.iter().map(|opts| self.convert(opts)).collect()
        }
    }

    /// Extract video dimensions and duration using `ffprobe` if available.
    fn probe_metadata(&self, path: &Path) -> Option<VideoMetadata> {
        let ffprobe = self.ffprobe_path.as_ref()?;
        let output = Command::new(ffprobe)
            .arg("-v")
            .arg("error")
            .arg("-select_streams")
            .arg("v:0")
            .arg("-show_entries")
            .arg("stream=width,height,duration:format=duration")
            .arg("-of")
            .arg("json")
            .arg(path)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json_val: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
        let stream = json_val.get("streams")?.as_array()?.first()?;

        let width = stream.get("width").and_then(|v| v.as_u64()).map(|w| w as u32);
        let height = stream.get("height").and_then(|v| v.as_u64()).map(|h| h as u32);

        let duration_secs = stream
            .get("duration")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| {
                json_val
                    .get("format")
                    .and_then(|f| f.get("duration"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
            });

        Some(VideoMetadata {
            width,
            height,
            duration_secs,
        })
    }
}
