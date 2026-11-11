use colored::Colorize;
use inquire::{Confirm, CustomType, Select, Text};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::commands::{extract_commands, CommandLog};
use crate::config::ConvertOptions;
use crate::converter::Converter;
use crate::recorder::{record_session, RecordOptions};
use crate::types::{format_bytes, OutputFormat, Preset, Theme};

// ─── Display wrappers for Select menus ───────────────────────────────────────

#[derive(Debug, Clone)]
enum MainAction {
    ConvertExisting,
    RecordAndConvert,
    ExtractCommands,
}

impl fmt::Display for MainAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MainAction::ConvertExisting => write!(f, "🎬 Convert an existing .cast recording to video"),
            MainAction::RecordAndConvert => write!(f, "🔴 Record a new terminal session (with command tracking)"),
            MainAction::ExtractCommands => write!(f, "📋 Extract commands from an existing .cast file"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormatChoice {
    Mp4,
    WebM,
    Gif,
}

impl fmt::Display for FormatChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatChoice::Mp4 => write!(f, "MP4  — H.264 / faststart (Universal, mobile & web)"),
            FormatChoice::WebM => write!(f, "WebM — VP9 (Open format, high web compression)"),
            FormatChoice::Gif => write!(f, "GIF  — Animated GIF image"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ThemeChoice {
    Preset(Theme, &'static str),
    Custom,
}

impl fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThemeChoice::Preset(_, desc) => write!(f, "{}", desc),
            ThemeChoice::Custom => write!(f, "Custom — Enter theme name"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ScaleChoice { X1, X2, X3 }

impl fmt::Display for ScaleChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScaleChoice::X2 => write!(f, "2x Retina / HiDPI (Razor sharp text, recommended)"),
            ScaleChoice::X1 => write!(f, "1x Standard (Original pixel resolution)"),
            ScaleChoice::X3 => write!(f, "3x Ultra (4K presentations & displays)"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SpeedChoice { S1_0, S1_25, S1_5, S2_0, Custom }

impl fmt::Display for SpeedChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpeedChoice::S1_0  => write!(f, "1.0x — Normal speed"),
            SpeedChoice::S1_25 => write!(f, "1.25x — Slightly faster"),
            SpeedChoice::S1_5  => write!(f, "1.5x — Fast"),
            SpeedChoice::S2_0  => write!(f, "2.0x — Double speed"),
            SpeedChoice::Custom => write!(f, "Custom speed..."),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PauseChoice { Cap2_0, Cap1_0, NoCap, Custom }

impl fmt::Display for PauseChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PauseChoice::Cap2_0  => write!(f, "Cap to 2.0s (Recommended, trims long pauses)"),
            PauseChoice::Cap1_0  => write!(f, "Cap to 1.0s (Fast-paced flow)"),
            PauseChoice::NoCap   => write!(f, "No cap (Keep 100% natural typing timing)"),
            PauseChoice::Custom  => write!(f, "Custom pause limit..."),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum CommandSaveFormat { Report, Script, Both }

impl fmt::Display for CommandSaveFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandSaveFormat::Report => write!(f, "📄 Annotated report (.commands.txt) — Timestamped log"),
            CommandSaveFormat::Script => write!(f, "📜 Executable script (.commands.sh) — Re-runnable"),
            CommandSaveFormat::Both   => write!(f, "📦 Both report + script"),
        }
    }
}

// ─── Banner ──────────────────────────────────────────────────────────────────

fn print_banner() {
    println!();
    println!("{}", "╭────────────────────────────────────────────────────────────╮".cyan());
    println!("{}", "│              🎥  cast2video Studio  v0.2.0                 │".cyan().bold());
    println!("{}", "│     Record  •  Convert  •  Track Commands                  │".cyan());
    println!("{}", "╰────────────────────────────────────────────────────────────╯".cyan());
    println!();
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

fn find_cast_files() -> Vec<PathBuf> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(&cwd) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("cast") {
                files.push(p);
            }
        }
    }
    files.sort();
    files
}

fn pick_cast_file(cast_files: &[PathBuf]) -> std::result::Result<String, Box<dyn std::error::Error>> {
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    enum InputChoice { LocalFile(PathBuf, u64), BatchAll(Vec<PathBuf>), CustomInput }

    impl fmt::Display for InputChoice {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                InputChoice::LocalFile(p, sz) => {
                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                    write!(f, "📁 {} ({})", name, format_bytes(*sz))
                }
                InputChoice::BatchAll(v) => write!(f, "⚡ All {} .cast files", v.len()),
                InputChoice::CustomInput  => write!(f, "🔗 Enter custom file path or URL..."),
            }
        }
    }

    let mut opts = Vec::new();
    for p in cast_files {
        let sz = fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        opts.push(InputChoice::LocalFile(p.clone(), sz));
    }
    opts.push(InputChoice::CustomInput);

    let sel = Select::new("Select recording:", opts).with_page_size(10).prompt()?;
    match sel {
        InputChoice::LocalFile(p, _) => Ok(p.to_string_lossy().to_string()),
        InputChoice::BatchAll(_) => unreachable!(),
        InputChoice::CustomInput => {
            let v = Text::new("Enter .cast file path or URL:")
                .with_placeholder("e.g. ./demo.cast or https://asciinema.org/a/...")
                .prompt()?;
            Ok(v.trim().to_string())
        }
    }
}

fn prompt_video_options() -> std::result::Result<(OutputFormat, Theme, u32, f64, Option<f64>), Box<dyn std::error::Error>> {
    // Format
    let fc = Select::new("Output format:", vec![FormatChoice::Mp4, FormatChoice::WebM, FormatChoice::Gif]).prompt()?;
    let format = match fc { FormatChoice::Mp4 => OutputFormat::Mp4, FormatChoice::WebM => OutputFormat::WebM, FormatChoice::Gif => OutputFormat::Gif };

    // Theme
    let theme_opts = vec![
        ThemeChoice::Preset(Theme::Dracula, "Dracula — Vibrant dark purple / pink / cyan"),
        ThemeChoice::Preset(Theme::Nord, "Nord — Cool arctic blue-gray"),
        ThemeChoice::Preset(Theme::Monokai, "Monokai — High-contrast classic"),
        ThemeChoice::Preset(Theme::Kanagawa, "Kanagawa — Japanese woodblock aesthetic"),
        ThemeChoice::Preset(Theme::GruvboxDark, "Gruvbox Dark — Warm retro groove"),
        ThemeChoice::Preset(Theme::GithubDark, "GitHub Dark — Modern GitHub dark"),
        ThemeChoice::Preset(Theme::GithubLight, "GitHub Light — Clean white background"),
        ThemeChoice::Preset(Theme::SolarizedDark, "Solarized Dark — Classic cyan / blue"),
        ThemeChoice::Preset(Theme::Asciinema, "Asciinema — Standard terminal colors"),
        ThemeChoice::Custom,
    ];
    let tc = Select::new("Color theme:", theme_opts).with_page_size(10).prompt()?;
    let theme = match tc {
        ThemeChoice::Preset(t, _) => t,
        ThemeChoice::Custom => {
            let s = Text::new("Enter theme name:").with_default("dracula").prompt()?;
            Theme::from_str(&s).unwrap_or(Theme::Dracula)
        }
    };

    // Scale
    let sc = Select::new("Resolution scaling:", vec![ScaleChoice::X2, ScaleChoice::X1, ScaleChoice::X3]).prompt()?;
    let scale = match sc { ScaleChoice::X1 => 1, ScaleChoice::X2 => 2, ScaleChoice::X3 => 3 };

    // Speed
    let sp = Select::new("Playback speed:", vec![SpeedChoice::S1_0, SpeedChoice::S1_25, SpeedChoice::S1_5, SpeedChoice::S2_0, SpeedChoice::Custom]).prompt()?;
    let speed = match sp {
        SpeedChoice::S1_0 => 1.0, SpeedChoice::S1_25 => 1.25, SpeedChoice::S1_5 => 1.5, SpeedChoice::S2_0 => 2.0,
        SpeedChoice::Custom => CustomType::<f64>::new("Speed multiplier:").with_default(1.0).prompt()?,
    };

    // Pause cap
    let pc = Select::new("Typing pause limit:", vec![PauseChoice::Cap2_0, PauseChoice::Cap1_0, PauseChoice::NoCap, PauseChoice::Custom]).prompt()?;
    let idle = match pc {
        PauseChoice::Cap2_0 => Some(2.0), PauseChoice::Cap1_0 => Some(1.0), PauseChoice::NoCap => None,
        PauseChoice::Custom => {
            let v = CustomType::<f64>::new("Max pause (seconds, 0=disabled):").with_default(2.0).prompt()?;
            if v > 0.0 { Some(v) } else { None }
        }
    };

    Ok((format, theme, scale, speed, idle))
}

fn save_command_log(log: &CommandLog, cast_stem: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if log.commands.is_empty() {
        println!(" {} No commands detected in the recording.", "▲".yellow());
        println!("   Tip: use {} when recording to enable keystroke capture.", "--stdin".bold());
        return Ok(());
    }

    println!();
    println!("{}", "─── Extracted Commands ───".cyan().bold());
    for cmd in &log.commands {
        println!(
            "  {} {:<8} {}",
            format!("[{}]", cmd.index).dimmed(),
            format!("{:.1}s", cmd.timestamp).dimmed(),
            cmd.command
        );
    }
    println!("{}", format!("  ({} commands, {:.1}s total)", log.commands.len(), log.duration_secs).dimmed());
    println!();

    let save_fmt = Select::new("Save command log as:", vec![
        CommandSaveFormat::Both,
        CommandSaveFormat::Report,
        CommandSaveFormat::Script,
    ]).prompt()?;

    match save_fmt {
        CommandSaveFormat::Report | CommandSaveFormat::Both => {
            let rp = format!("{}.commands.txt", cast_stem);
            let rp = Text::new("Report file:").with_default(&rp).prompt()?;
            let saved = log.save_report(&rp)?;
            println!(" {} Command log saved: {}", "✔".green(), saved.display());
        }
        _ => {}
    }

    match save_fmt {
        CommandSaveFormat::Script | CommandSaveFormat::Both => {
            let sp = format!("{}.commands.sh", cast_stem);
            let sp = Text::new("Script file:").with_default(&sp).prompt()?;
            let saved = log.save_script(&sp)?;
            println!(" {} Executable script saved: {}", "✔".green(), saved.display());
        }
        _ => {}
    }

    Ok(())
}

// ─── Main wizard entry point ─────────────────────────────────────────────────

/// Run the interactive CLI wizard.
pub fn run_interactive_wizard() -> std::result::Result<(), Box<dyn std::error::Error>> {
    print_banner();

    // Top-level action selection
    let action = Select::new("What would you like to do?", vec![
        MainAction::ConvertExisting,
        MainAction::RecordAndConvert,
        MainAction::ExtractCommands,
    ]).prompt()?;

    match action {
        MainAction::ConvertExisting => wizard_convert()?,
        MainAction::RecordAndConvert => wizard_record()?,
        MainAction::ExtractCommands => wizard_extract()?,
    }

    Ok(())
}

// ─── Convert existing recording ─────────────────────────────────────────────

fn wizard_convert() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let converter = match Converter::new() {
        Ok(c) => c,
        Err(err) => { eprintln!("{} {}", "✖ Error:".red().bold(), err); return Ok(()); }
    };

    let cast_files = find_cast_files();
    let input = pick_cast_file(&cast_files)?;
    if input.is_empty() { println!("{}", "No input provided.".yellow()); return Ok(()); }

    let (format, theme, scale, speed, idle) = prompt_video_options()?;

    let default_out = if input.starts_with("http") {
        format!("asciinema_recording.{}", format.extension())
    } else {
        let stem = Path::new(&input).file_stem().and_then(|s| s.to_str()).unwrap_or("recording");
        format!("{}.{}", stem, format.extension())
    };
    let dest = Text::new("Output video file:").with_default(&default_out).prompt()?;

    // Ask about command extraction
    let extract = Confirm::new("Also extract commands from this recording?").with_default(true).prompt()?;

    // Summary
    println!();
    println!("{}", "─── Conversion Configuration ───".cyan().bold());
    println!("  • Input:      {}", input);
    println!("  • Output:     {}", dest.bold());
    println!("  • Format:     {}", format.to_string().to_uppercase().bold());
    println!("  • Theme:      {}", theme.as_str());
    println!("  • Speed:      {:.2}x", speed);
    println!("  • Pause Cap:  {}", idle.map(|s| format!("{:.1}s", s)).unwrap_or_else(|| "None".to_string()));
    println!("  • Scale:      {}x", scale);
    println!("  • Commands:   {}", if extract { "Yes" } else { "No" });
    println!("{}", "────────────────────────────────".cyan());
    println!();

    if !Confirm::new("Start rendering?").with_default(true).prompt()? {
        println!("{}", "Cancelled.".yellow());
        return Ok(());
    }

    // Extract commands first (before conversion, from the raw cast file)
    if extract {
        let cast_stem = Path::new(&input).file_stem().and_then(|s| s.to_str()).unwrap_or("recording");
        match extract_commands(&input) {
            Ok(log) => save_command_log(&log, cast_stem)?,
            Err(e) => eprintln!(" {} Command extraction failed: {}", "▲".yellow(), e),
        }
    }

    println!();
    println!(" {} Rendering video...", "●".cyan());

    let opt = ConvertOptions::builder(&input)
        .output(PathBuf::from(&dest))
        .format(format)
        .theme(theme)
        .speed(speed)
        .idle_time_limit(idle)
        .scale(scale)
        .preset(Preset::Medium)
        .build();

    match converter.convert(&opt) {
        Ok(c) => {
            let name = c.output_path.file_name().unwrap_or_default().to_string_lossy();
            let dims = c.formatted_dimensions().unwrap_or_default();
            let dur = c.formatted_duration().unwrap_or_default();
            println!();
            println!(
                "{} Video created: {} ({}, {}, {}) in {:.2}s",
                "✔".green().bold(), name.bold(), dims, dur,
                c.formatted_output_size(), c.elapsed.as_secs_f64()
            );
            println!("   Saved at: {}", c.output_path.canonicalize().unwrap_or(c.output_path).display());
        }
        Err(e) => eprintln!("{} Conversion failed: {}", "✖ Error:".red().bold(), e),
    }

    Ok(())
}

// ─── Record new session ─────────────────────────────────────────────────────

fn wizard_record() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("{}", "─── New Terminal Recording ───".cyan().bold());
    println!("  This will start a new interactive shell. Everything you type");
    println!("  and see will be recorded. All commands are automatically tracked.");
    println!();
    println!("  {} Type {} or press {} to stop recording.", "Tip:".bold(), "exit".green().bold(), "Ctrl+D".green().bold());
    println!();

    let title = Text::new("Recording title (optional):")
        .with_placeholder("e.g. 'Setup demo', 'API walkthrough'")
        .with_default("")
        .prompt()?;

    let default_file = if title.is_empty() {
        "recording.cast".to_string()
    } else {
        format!("{}.cast", title.to_lowercase().replace(' ', "_").replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', ""))
    };
    let output_file = Text::new("Save recording as:").with_default(&default_file).prompt()?;

    let idle_cap = Confirm::new("Cap idle pauses during recording? (Prevents long dead time)")
        .with_default(true)
        .prompt()?;
    let idle_limit = if idle_cap {
        Some(CustomType::<f64>::new("Max idle seconds:").with_default(5.0).prompt()?)
    } else {
        None
    };

    let convert_after = Confirm::new("Convert to video after recording?").with_default(true).prompt()?;

    println!();
    println!("{}", "━".repeat(50).cyan());
    println!("{} Starting recording... {} or {} to stop.",
        "🔴 REC".red().bold(), "exit".bold(), "Ctrl+D".bold());
    println!("{}", "━".repeat(50).cyan());
    println!();

    let rec_opts = RecordOptions {
        output_path: PathBuf::from(&output_file),
        capture_stdin: true,
        cols: None,
        rows: None,
        idle_time_limit: idle_limit,
        title: if title.is_empty() { None } else { Some(title) },
        command: None,
        append: false,
        quiet: false,
    };

    let rec_result = match record_session(&rec_opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} Recording failed: {}", "✖ Error:".red().bold(), e);
            return Ok(());
        }
    };

    println!();
    println!("{}", "━".repeat(50).cyan());
    println!(
        "{} Recording saved: {} ({}, {} events)",
        "✔".green().bold(),
        rec_result.cast_path.display().to_string().bold(),
        format_bytes(rec_result.cast_size_bytes),
        rec_result.event_count
    );
    println!("{}", "━".repeat(50).cyan());

    // Extract commands
    let cast_stem = Path::new(&output_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("recording");

    match extract_commands(&rec_result.cast_path) {
        Ok(log) => save_command_log(&log, cast_stem)?,
        Err(e) => eprintln!(" {} Command extraction failed: {}", "▲".yellow(), e),
    }

    // Convert to video
    if convert_after {
        let converter = match Converter::new() {
            Ok(c) => c,
            Err(err) => { eprintln!("{} {}", "✖ Error:".red().bold(), err); return Ok(()); }
        };

        println!();
        let (format, theme, scale, speed, idle) = prompt_video_options()?;

        let video_file = format!("{}.{}", cast_stem, format.extension());
        let video_file = Text::new("Output video file:").with_default(&video_file).prompt()?;

        println!();
        println!(" {} Rendering video...", "●".cyan());

        let opt = ConvertOptions::builder(rec_result.cast_path.to_string_lossy().to_string())
            .output(PathBuf::from(&video_file))
            .format(format)
            .theme(theme)
            .speed(speed)
            .idle_time_limit(idle)
            .scale(scale)
            .preset(Preset::Medium)
            .build();

        match converter.convert(&opt) {
            Ok(c) => {
                let name = c.output_path.file_name().unwrap_or_default().to_string_lossy();
                let dims = c.formatted_dimensions().unwrap_or_default();
                let dur = c.formatted_duration().unwrap_or_default();
                println!();
                println!(
                    "{} Video created: {} ({}, {}, {}) in {:.2}s",
                    "✔".green().bold(), name.bold(), dims, dur,
                    c.formatted_output_size(), c.elapsed.as_secs_f64()
                );
            }
            Err(e) => eprintln!("{} Conversion failed: {}", "✖ Error:".red().bold(), e),
        }
    }

    Ok(())
}

// ─── Extract commands from existing recording ────────────────────────────────

fn wizard_extract() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cast_files = find_cast_files();
    let input = pick_cast_file(&cast_files)?;
    if input.is_empty() { println!("{}", "No input provided.".yellow()); return Ok(()); }

    let cast_stem = Path::new(&input)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("recording");

    println!();
    println!(" {} Parsing {}...", "●".cyan(), input.bold());

    match extract_commands(&input) {
        Ok(log) => save_command_log(&log, cast_stem)?,
        Err(e) => eprintln!("{} Command extraction failed: {}", "✖ Error:".red().bold(), e),
    }

    Ok(())
}
