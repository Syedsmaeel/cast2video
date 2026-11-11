use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

/// Supported output video/image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[value(name = "mp4")]
    Mp4,
    #[value(name = "webm")]
    WebM,
    #[value(name = "gif")]
    Gif,
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Mp4 => "mp4",
            OutputFormat::WebM => "webm",
            OutputFormat::Gif => "gif",
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.extension())
    }
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mp4" => Ok(OutputFormat::Mp4),
            "webm" => Ok(OutputFormat::WebM),
            "gif" => Ok(OutputFormat::Gif),
            other => Err(format!("Unsupported output format: '{}'. Choose from mp4, webm, gif", other)),
        }
    }
}

/// Color theme for the terminal recording.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Asciinema,
    Dracula,
    GithubDark,
    GithubLight,
    GruvboxDark,
    Kanagawa,
    KanagawaDragon,
    KanagawaLight,
    Monokai,
    Nord,
    SolarizedDark,
    SolarizedLight,
    Custom(String),
}

impl Theme {
    pub fn as_str(&self) -> &str {
        match self {
            Theme::Asciinema => "asciinema",
            Theme::Dracula => "dracula",
            Theme::GithubDark => "github-dark",
            Theme::GithubLight => "github-light",
            Theme::GruvboxDark => "gruvbox-dark",
            Theme::Kanagawa => "kanagawa",
            Theme::KanagawaDragon => "kanagawa-dragon",
            Theme::KanagawaLight => "kanagawa-light",
            Theme::Monokai => "monokai",
            Theme::Nord => "nord",
            Theme::SolarizedDark => "solarized-dark",
            Theme::SolarizedLight => "solarized-light",
            Theme::Custom(name) => name.as_str(),
        }
    }
}

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Theme {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let t = match s.to_lowercase().as_str() {
            "asciinema" => Theme::Asciinema,
            "dracula" => Theme::Dracula,
            "github-dark" => Theme::GithubDark,
            "github-light" => Theme::GithubLight,
            "gruvbox-dark" => Theme::GruvboxDark,
            "kanagawa" => Theme::Kanagawa,
            "kanagawa-dragon" => Theme::KanagawaDragon,
            "kanagawa-light" => Theme::KanagawaLight,
            "monokai" => Theme::Monokai,
            "nord" => Theme::Nord,
            "solarized-dark" => Theme::SolarizedDark,
            "solarized-light" => Theme::SolarizedLight,
            _ => Theme::Custom(s.to_string()),
        };
        Ok(t)
    }
}

/// H.264 compression presets for FFmpeg.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    Ultrafast,
    Superfast,
    Veryfast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
}

impl Preset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Preset::Ultrafast => "ultrafast",
            Preset::Superfast => "superfast",
            Preset::Veryfast => "veryfast",
            Preset::Faster => "faster",
            Preset::Fast => "fast",
            Preset::Medium => "medium",
            Preset::Slow => "slow",
            Preset::Slower => "slower",
        }
    }
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Video resolution and duration info extracted from ffprobe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct VideoMetadata {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_secs: Option<f64>,
}

/// Result of a completed video conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub input: String,
    pub output_path: PathBuf,
    pub format: OutputFormat,
    pub output_size_bytes: u64,
    pub input_size_bytes: Option<u64>,
    pub metadata: Option<VideoMetadata>,
    pub elapsed: Duration,
}

impl ConversionResult {
    pub fn formatted_output_size(&self) -> String {
        format_bytes(self.output_size_bytes)
    }

    pub fn formatted_input_size(&self) -> String {
        self.input_size_bytes
            .map(format_bytes)
            .unwrap_or_else(|| "URL".to_string())
    }

    pub fn formatted_duration(&self) -> Option<String> {
        self.metadata
            .as_ref()
            .and_then(|m| m.duration_secs)
            .map(|d| format!("{:.1}s", d))
    }

    pub fn formatted_dimensions(&self) -> Option<String> {
        self.metadata.as_ref().and_then(|m| match (m.width, m.height) {
            (Some(w), Some(h)) => Some(format!("{}x{}", w, h)),
            _ => None,
        })
    }
}

/// Utility function to format byte counts into human-readable strings.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    for (i, unit) in UNITS.iter().enumerate() {
        if size < 1024.0 || i == UNITS.len() - 1 {
            return if i == 0 {
                format!("{} B", bytes)
            } else {
                format!("{:.1} {}", size, unit)
            };
        }
        size /= 1024.0;
    }
    format!("{} B", bytes)
}
