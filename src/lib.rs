//! # cast2video
//!
//! Professional Asciinema (`.cast`) to MP4 and WebM video converter library, CLI, and interactive studio.
//!
//! Also supports **recording** new terminal sessions with automatic command tracking.
//!
//! ## Quick Example
//!
//! ```no_run
//! use cast2video::{Converter, ConvertOptions, OutputFormat, Theme};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let converter = Converter::new()?;
//!
//!     let options = ConvertOptions::builder("demo.cast")
//!         .output("demo.mp4")
//!         .format(OutputFormat::Mp4)
//!         .theme(Theme::Dracula)
//!         .speed(1.5)
//!         .scale(2)
//!         .build();
//!
//!     let result = converter.convert(&options)?;
//!     println!(
//!         "Converted to {:?} ({}, duration: {:?})",
//!         result.output_path,
//!         result.formatted_output_size(),
//!         result.formatted_duration()
//!     );
//!     Ok(())
//! }
//! ```

pub mod commands;
pub mod config;
pub mod converter;
pub mod error;
pub mod interactive;
pub mod recorder;
pub mod types;

// Re-exports for convenient public API
pub use commands::{extract_commands, CommandLog, ExtractedCommand};
pub use config::{ConvertOptions, ConvertOptionsBuilder};
pub use converter::Converter;
pub use error::{CastError, Result};
pub use interactive::run_interactive_wizard;
pub use recorder::{record_session, RecordOptions, RecordResult};
pub use types::{format_bytes, ConversionResult, OutputFormat, Preset, Theme, VideoMetadata};

/// Convenience function to convert a single cast file or URL with default options.
pub fn convert<P1: AsRef<std::path::Path>, P2: AsRef<std::path::Path>>(
    input: P1,
    output: P2,
) -> Result<ConversionResult> {
    let converter = Converter::new()?;
    let options = ConvertOptions::builder(input.as_ref().to_string_lossy().to_string())
        .output(output)
        .build();
    converter.convert(&options)
}
