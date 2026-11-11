use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during cast-to-video conversion.
#[derive(Error, Debug)]
pub enum CastError {
    #[error("Required dependency '{binary}' not found in PATH.\n{advice}")]
    MissingDependency {
        binary: String,
        advice: String,
    },

    #[error("Input file does not exist: {0}")]
    InputNotFound(PathBuf),

    #[error("Output directory does not exist and could not be created: {0}")]
    OutputDirectoryFailed(PathBuf),

    #[error("agg execution failed with status {status:?}: {stderr}")]
    AggFailed {
        status: Option<i32>,
        stderr: String,
    },

    #[error("FFmpeg execution failed with status {status:?}: {stderr}")]
    FfmpegFailed {
        status: Option<i32>,
        stderr: String,
    },

    #[error("Intermediate render file missing: {0}")]
    IntermediateFileMissing(PathBuf),

    #[error("Invalid conversion option: {0}")]
    InvalidOption(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, CastError>;
