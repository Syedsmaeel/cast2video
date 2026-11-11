use crate::types::{OutputFormat, Preset, Theme};
use std::path::{Path, PathBuf};

/// Configuration options for converting an asciinema .cast file or URL to video.
#[derive(Debug, Clone)]
pub struct ConvertOptions {
    /// Path to input .cast file or asciinema URL
    pub input: String,
    /// Destination path for the converted video
    pub output_path: PathBuf,
    /// Video format (MP4, WebM, or GIF)
    pub format: OutputFormat,
    /// Color theme
    pub theme: Theme,
    /// Playback speed multiplier (default: 1.0)
    pub speed: f64,
    /// Maximum idle pause in seconds between keystrokes (None or 0 disables)
    pub idle_time_limit: Option<f64>,
    /// Frame rate cap in FPS (default: 30)
    pub fps_cap: u32,
    /// Monospace font family list
    pub font_family: Option<String>,
    /// Font size in pixels (default: 16)
    pub font_size: Option<u32>,
    /// Terminal line height multiplier (default: 1.4)
    pub line_height: Option<f64>,
    /// Terminal width override in columns
    pub cols: Option<u32>,
    /// Terminal height override in rows
    pub rows: Option<u32>,
    /// Integer upscale factor (e.g. 2 for 2x Retina scaling)
    pub scale: u32,
    /// Constant Rate Factor (video quality)
    pub crf: Option<u32>,
    /// H.264 compression preset
    pub preset: Preset,
    /// Whether to preserve the intermediate GIF file
    pub keep_intermediate: bool,
    /// Suppress informational output
    pub quiet: bool,
    /// Display underlying tool diagnostic messages
    pub verbose: bool,
    /// Dry run: display exact commands without executing
    pub dry_run: bool,
}

impl ConvertOptions {
    /// Initialize a new Builder for `ConvertOptions`.
    pub fn builder<S: Into<String>>(input: S) -> ConvertOptionsBuilder {
        ConvertOptionsBuilder::new(input)
    }

    /// Check if the input is an HTTP or HTTPS URL.
    pub fn is_url(&self) -> bool {
        self.input.starts_with("http://") || self.input.starts_with("https://")
    }

    /// Effective CRF value based on the target format if not explicitly set.
    pub fn effective_crf(&self) -> u32 {
        if let Some(c) = self.crf {
            return c;
        }
        match self.format {
            OutputFormat::Mp4 => 20,
            OutputFormat::WebM => 28,
            OutputFormat::Gif => 0,
        }
    }
}

/// Builder pattern for `ConvertOptions`.
#[derive(Debug, Clone)]
pub struct ConvertOptionsBuilder {
    input: String,
    output_path: Option<PathBuf>,
    format: Option<OutputFormat>,
    theme: Theme,
    speed: f64,
    idle_time_limit: Option<f64>,
    fps_cap: u32,
    font_family: Option<String>,
    font_size: Option<u32>,
    line_height: Option<f64>,
    cols: Option<u32>,
    rows: Option<u32>,
    scale: u32,
    crf: Option<u32>,
    preset: Preset,
    keep_intermediate: bool,
    quiet: bool,
    verbose: bool,
    dry_run: bool,
}

impl ConvertOptionsBuilder {
    pub fn new<S: Into<String>>(input: S) -> Self {
        Self {
            input: input.into(),
            output_path: None,
            format: None,
            theme: Theme::Dracula,
            speed: 1.0,
            idle_time_limit: Some(2.0),
            fps_cap: 30,
            font_family: None,
            font_size: None,
            line_height: None,
            cols: None,
            rows: None,
            scale: 1,
            crf: None,
            preset: Preset::Medium,
            keep_intermediate: false,
            quiet: false,
            verbose: false,
            dry_run: false,
        }
    }

    pub fn output<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.output_path = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn format(mut self, format: OutputFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    pub fn idle_time_limit(mut self, limit: Option<f64>) -> Self {
        self.idle_time_limit = limit;
        self
    }

    pub fn fps_cap(mut self, fps: u32) -> Self {
        self.fps_cap = fps;
        self
    }

    pub fn font_family<S: Into<String>>(mut self, font: S) -> Self {
        self.font_family = Some(font.into());
        self
    }

    pub fn font_size(mut self, size: u32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn line_height(mut self, height: f64) -> Self {
        self.line_height = Some(height);
        self
    }

    pub fn cols(mut self, cols: u32) -> Self {
        self.cols = Some(cols);
        self
    }

    pub fn rows(mut self, rows: u32) -> Self {
        self.rows = Some(rows);
        self
    }

    pub fn scale(mut self, scale: u32) -> Self {
        self.scale = scale;
        self
    }

    pub fn crf(mut self, crf: u32) -> Self {
        self.crf = Some(crf);
        self
    }

    pub fn preset(mut self, preset: Preset) -> Self {
        self.preset = preset;
        self
    }

    pub fn keep_intermediate(mut self, keep: bool) -> Self {
        self.keep_intermediate = keep;
        self
    }

    pub fn quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Build the `ConvertOptions` instance.
    /// If output path or format is omitted, auto-infers appropriate defaults.
    pub fn build(self) -> ConvertOptions {
        let is_url = self.input.starts_with("http://") || self.input.starts_with("https://");

        // Determine format from output path if format was not explicitly given
        let format = self.format.unwrap_or_else(|| {
            if let Some(ref out) = self.output_path {
                if let Some(ext) = out.extension().and_then(|s| s.to_str()) {
                    match ext.to_lowercase().as_str() {
                        "webm" => OutputFormat::WebM,
                        "gif" => OutputFormat::Gif,
                        _ => OutputFormat::Mp4,
                    }
                } else {
                    OutputFormat::Mp4
                }
            } else {
                OutputFormat::Mp4
            }
        });

        // Determine default output path if not provided
        let output_path = self.output_path.unwrap_or_else(|| {
            let ext = format.extension();
            if is_url {
                PathBuf::from(format!("asciinema_recording.{}", ext))
            } else {
                Path::new(&self.input).with_extension(ext)
            }
        });

        ConvertOptions {
            input: self.input,
            output_path,
            format,
            theme: self.theme,
            speed: self.speed,
            idle_time_limit: self.idle_time_limit,
            fps_cap: self.fps_cap,
            font_family: self.font_family,
            font_size: self.font_size,
            line_height: self.line_height,
            cols: self.cols,
            rows: self.rows,
            scale: self.scale,
            crf: self.crf,
            preset: self.preset,
            keep_intermediate: self.keep_intermediate,
            quiet: self.quiet,
            verbose: self.verbose,
            dry_run: self.dry_run,
        }
    }
}
