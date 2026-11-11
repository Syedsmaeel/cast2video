use crate::error::{CastError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Options for recording a terminal session.
#[derive(Debug, Clone)]
pub struct RecordOptions {
    /// Output .cast file path
    pub output_path: PathBuf,
    /// Whether to capture stdin (input keystrokes) — required for command tracking
    pub capture_stdin: bool,
    /// Override terminal columns
    pub cols: Option<u32>,
    /// Override terminal rows
    pub rows: Option<u32>,
    /// Idle time limit in seconds (pauses longer than this are capped)
    pub idle_time_limit: Option<f64>,
    /// Custom title/name for the recording
    pub title: Option<String>,
    /// Custom shell command to run inside the recording (default: user's shell)
    pub command: Option<String>,
    /// Append to existing recording file instead of overwriting
    pub append: bool,
    /// Quiet mode: suppress asciinema's own status messages
    pub quiet: bool,
}

impl RecordOptions {
    /// Create a new RecordOptions with sensible defaults.
    pub fn new<P: AsRef<Path>>(output_path: P) -> Self {
        Self {
            output_path: output_path.as_ref().to_path_buf(),
            capture_stdin: true,
            cols: None,
            rows: None,
            idle_time_limit: None,
            title: None,
            command: None,
            append: false,
            quiet: false,
        }
    }
}

/// Result of a completed recording session.
#[derive(Debug, Clone)]
pub struct RecordResult {
    /// Path to the generated .cast file
    pub cast_path: PathBuf,
    /// File size of the .cast recording in bytes
    pub cast_size_bytes: u64,
    /// Total number of events in the recording
    pub event_count: usize,
}

/// Record a new terminal session using asciinema.
///
/// This launches `asciinema rec` in the foreground, giving the user
/// an interactive shell. The recording stops when the user types `exit`
/// or presses Ctrl+D.
pub fn record_session(options: &RecordOptions) -> Result<RecordResult> {
    // Check asciinema is available
    let asciinema_path = which::which("asciinema").map_err(|_| CastError::MissingDependency {
        binary: "asciinema".to_string(),
        advice: "Install asciinema:\n  Ubuntu/Debian: sudo apt install asciinema\n  macOS: brew install asciinema\n  pip: pip install asciinema".to_string(),
    })?;

    // Ensure output parent directory exists
    if let Some(parent) = options.output_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|_| CastError::OutputDirectoryFailed(parent.to_path_buf()))?;
        }
    }

    // Build asciinema rec command
    let mut cmd = Command::new(&asciinema_path);
    cmd.arg("rec");

    // --stdin captures input keystrokes for command tracking
    if options.capture_stdin {
        cmd.arg("--stdin");
    }

    if let Some(limit) = options.idle_time_limit {
        cmd.arg("--idle-time-limit").arg(limit.to_string());
    }

    if let Some(ref title) = options.title {
        cmd.arg("--title").arg(title);
    }

    if let Some(ref shell_cmd) = options.command {
        cmd.arg("--command").arg(shell_cmd);
    }

    if options.append {
        cmd.arg("--append");
    }

    if options.quiet {
        cmd.arg("--quiet");
    }

    if let Some(cols) = options.cols {
        cmd.arg("--cols").arg(cols.to_string());
    }

    if let Some(rows) = options.rows {
        cmd.arg("--rows").arg(rows.to_string());
    }

    // Overwrite existing file
    cmd.arg("--overwrite");

    // Output file
    cmd.arg(&options.output_path);

    // Run in foreground (interactive — inherits stdio)
    let status = cmd
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(CastError::AggFailed {
            status: status.code(),
            stderr: "asciinema rec exited with non-zero status".to_string(),
        });
    }

    if !options.output_path.exists() {
        return Err(CastError::IntermediateFileMissing(
            options.output_path.clone(),
        ));
    }

    let cast_size_bytes = fs::metadata(&options.output_path)?.len();

    // Count events in the recording
    let content = fs::read_to_string(&options.output_path)?;
    let event_count = content.lines().count().saturating_sub(1); // first line is header

    Ok(RecordResult {
        cast_path: options.output_path.clone(),
        cast_size_bytes,
        event_count,
    })
}
