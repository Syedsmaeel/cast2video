use clap::Parser;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;

use cast2video::{
    run_interactive_wizard, ConvertOptions, Converter, OutputFormat, Preset, Theme,
};

#[derive(Parser, Debug)]
#[command(
    name = "cast2video-rs",
    author = "Antigravity",
    version,
    about = "Interactive & High-performance Asciinema (.cast) to MP4 / WebM video converter",
    long_about = "Converts asciinema recording files (.cast) or URLs into production-ready MP4 or WebM videos.\nRun without arguments or with -i to launch the interactive prompt wizard."
)]
struct Cli {
    /// Input .cast file(s), directory, or asciinema recording URL. If omitted, launches interactive wizard.
    #[arg(value_name = "INPUTS")]
    inputs: Vec<String>,

    /// Launch interactive terminal wizard (default when no input is specified)
    #[arg(short = 'i', long)]
    interactive: bool,

    /// Output video file path (for single input) or output directory (for batch mode)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Target video format (mp4, webm, gif). Inferred from output filename if omitted
    #[arg(short, long, value_enum)]
    format: Option<OutputFormat>,

    /// Color theme (dracula, monokai, nord, solarized-dark, github-dark, kanagawa, gruvbox-dark, etc.)
    #[arg(long, default_value = "dracula")]
    theme: String,

    /// Playback speed multiplier (e.g. 1.5 = 50% faster)
    #[arg(long, default_value_t = 1.0)]
    speed: f64,

    /// Limit idle pauses between keystrokes to max seconds (0 disables)
    #[arg(long, default_value_t = 2.0)]
    idle_time_limit: f64,

    /// Frame rate cap in FPS
    #[arg(long, default_value_t = 30)]
    fps: u32,

    /// Monospace font family list (e.g. 'JetBrains Mono', 'Fira Code')
    #[arg(long)]
    font_family: Option<String>,

    /// Font size in pixels
    #[arg(long)]
    font_size: Option<u32>,

    /// Line height multiplier
    #[arg(long)]
    line_height: Option<f64>,

    /// Override terminal columns width
    #[arg(long)]
    cols: Option<u32>,

    /// Override terminal rows height
    #[arg(long)]
    rows: Option<u32>,

    /// Integer upscale factor (e.g. 2 for crystal clear 2x Retina upscale)
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32))]
    scale: u32,

    /// Constant Rate Factor (video quality: lower = higher quality; MP4 default: 20, WebM default: 28)
    #[arg(long)]
    crf: Option<u32>,

    /// H.264 compression preset (speed vs file size)
    #[arg(long, value_enum, default_value_t = Preset::Medium)]
    preset: Preset,

    /// Preserve intermediate animated GIF alongside the converted video
    #[arg(long)]
    keep_intermediate: bool,

    /// Number of parallel worker threads for batch conversions (defaults to CPU cores)
    #[arg(short = 'j', long)]
    jobs: Option<usize>,

    /// Force sequential processing instead of parallel for batch conversions
    #[arg(long)]
    sequential: bool,

    /// Quiet mode: suppress non-essential output
    #[arg(short, long)]
    quiet: bool,

    /// Verbose mode: show underlying agg and ffmpeg diagnostic output
    #[arg(short, long)]
    verbose: bool,

    /// Display exact underlying commands without executing them
    #[arg(long)]
    dry_run: bool,
}

fn main() {
    let cli = Cli::parse();

    // ── Interactive mode: launch wizard when no inputs given or -i flag ──
    if cli.interactive || cli.inputs.is_empty() {
        if let Err(err) = run_interactive_wizard() {
            eprintln!("{} {}", "✖ Interactive wizard error:".red().bold(), err);
            process::exit(1);
        }
        return;
    }

    // ── Direct CLI mode (original behavior) ──

    // Set thread pool size if specified
    if let Some(jobs) = cli.jobs {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build_global();
    }

    // Resolve inputs (handle directories and globs)
    let mut resolved_inputs: Vec<String> = Vec::new();
    for inp in &cli.inputs {
        if inp.starts_with("http://") || inp.starts_with("https://") {
            resolved_inputs.push(inp.clone());
        } else {
            let p = Path::new(inp);
            if p.is_dir() {
                if let Ok(entries) = std::fs::read_dir(p) {
                    let mut found = Vec::new();
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("cast") {
                            found.push(path.to_string_lossy().to_string());
                        }
                    }
                    found.sort();
                    if found.is_empty() && !cli.quiet {
                        eprintln!(
                            "{} No .cast files found in directory: {}",
                            "▲".yellow(),
                            p.display()
                        );
                    }
                    resolved_inputs.extend(found);
                }
            } else if p.exists() {
                resolved_inputs.push(inp.clone());
            } else {
                eprintln!("{} Input file not found: {}", "✖ Error:".red().bold(), inp);
                process::exit(1);
            }
        }
    }

    if resolved_inputs.is_empty() {
        eprintln!("{} No input files to process.", "✖ Error:".red().bold());
        process::exit(1);
    }

    let is_batch = resolved_inputs.len() > 1;

    let converter = match Converter::new() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("{} {}", "✖ Error:".red().bold(), err);
            process::exit(1);
        }
    };

    let theme = Theme::from_str(&cli.theme).unwrap_or(Theme::Dracula);
    let idle_time_limit = if cli.idle_time_limit > 0.0 {
        Some(cli.idle_time_limit)
    } else {
        None
    };

    let mut options_batch: Vec<ConvertOptions> = Vec::new();

    for src in &resolved_inputs {
        let is_url = src.starts_with("http://") || src.starts_with("https://");

        let fmt = cli.format.unwrap_or_else(|| {
            if let Some(ref out) = cli.output {
                if !is_batch && (out.extension().is_some()) {
                    let ext = out.extension().unwrap().to_string_lossy().to_lowercase();
                    OutputFormat::from_str(&ext).unwrap_or(OutputFormat::Mp4)
                } else {
                    OutputFormat::Mp4
                }
            } else {
                OutputFormat::Mp4
            }
        });

        let dest_path: PathBuf = if let Some(ref out) = cli.output {
            if is_batch || out.is_dir() || (out.extension().is_none() && !out.exists()) {
                let stem = if is_url {
                    "recording"
                } else {
                    Path::new(src)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("recording")
                };
                out.join(format!("{}.{}", stem, fmt.extension()))
            } else {
                out.clone()
            }
        } else if is_url {
            PathBuf::from(format!("asciinema_recording.{}", fmt.extension()))
        } else {
            Path::new(src).with_extension(fmt.extension())
        };

        let mut builder = ConvertOptions::builder(src)
            .output(dest_path)
            .format(fmt)
            .theme(theme.clone())
            .speed(cli.speed)
            .idle_time_limit(idle_time_limit)
            .fps_cap(cli.fps)
            .scale(cli.scale)
            .preset(cli.preset)
            .keep_intermediate(cli.keep_intermediate)
            .quiet(cli.quiet)
            .verbose(cli.verbose)
            .dry_run(cli.dry_run);

        if let Some(ref f) = cli.font_family {
            builder = builder.font_family(f);
        }
        if let Some(s) = cli.font_size {
            builder = builder.font_size(s);
        }
        if let Some(lh) = cli.line_height {
            builder = builder.line_height(lh);
        }
        if let Some(c) = cli.cols {
            builder = builder.cols(c);
        }
        if let Some(r) = cli.rows {
            builder = builder.rows(r);
        }
        if let Some(crf) = cli.crf {
            builder = builder.crf(crf);
        }

        options_batch.push(builder.build());
    }

    let parallel = is_batch && !cli.sequential && !cli.dry_run;

    if !cli.quiet {
        if parallel {
            println!(
                " {} Converting {} files in parallel...",
                "●".cyan(),
                options_batch.len()
            );
        } else {
            for opts in &options_batch {
                let display_in = if opts.is_url() {
                    opts.input.clone()
                } else {
                    Path::new(&opts.input)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                };
                let display_out = opts
                    .output_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                println!(
                    " {} Converting {} → {} ({})",
                    "●".cyan(),
                    display_in.bold(),
                    display_out.bold(),
                    opts.format.to_string().to_uppercase()
                );
            }
        }
    }

    let results = converter.convert_batch(&options_batch, parallel);

    let mut had_error = false;
    let mut succeeded_count = 0;

    for (opts, res) in options_batch.iter().zip(results) {
        match res {
            Ok(conv) => {
                succeeded_count += 1;
                if !cli.quiet && !cli.dry_run {
                    let out_name = conv
                        .output_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    let dims = conv.formatted_dimensions();
                    let dur = conv.formatted_duration();
                    let mut meta = Vec::new();
                    if let Some(d) = dims {
                        meta.push(d);
                    }
                    if let Some(du) = dur {
                        meta.push(du);
                    }
                    meta.push(conv.formatted_output_size());

                    println!(
                        " {} Completed: {} ({}) in {:.2}s [Source: {}]",
                        "✔".green(),
                        out_name.bold(),
                        meta.join(", "),
                        conv.elapsed.as_secs_f64(),
                        conv.formatted_input_size()
                    );
                }
            }
            Err(err) => {
                had_error = true;
                eprintln!(
                    " {} Failed converting {}: {}",
                    "✖".red(),
                    opts.input.bold(),
                    err
                );
            }
        }
    }

    if is_batch && !cli.quiet {
        println!("\n{}", "=".repeat(50));
        let total = options_batch.len();
        if had_error {
            println!(
                " Batch Summary: {}",
                format!("{}/{} succeeded", succeeded_count, total).yellow()
            );
        } else {
            println!(
                " Batch Summary: {}",
                format!("{}/{} succeeded", succeeded_count, total).green()
            );
        }
        println!("{}", "=".repeat(50));
    }

    if had_error {
        process::exit(1);
    }
}
