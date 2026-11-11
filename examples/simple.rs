use cast2video::{ConvertOptions, Converter, OutputFormat, Theme};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Initialize converter (checks for 'agg' and 'ffmpeg' on system PATH)
    let converter = Converter::new()?;

    // 2. Configure options using Builder pattern
    let options = ConvertOptions::builder("my_session.cast")
        .output("my_session.mp4")
        .format(OutputFormat::Mp4)
        .theme(Theme::Nord)
        .speed(1.5)
        .scale(2)
        .idle_time_limit(Some(1.5))
        .build();

    // 3. Execute conversion
    println!("Starting conversion...");
    let result = converter.convert(&options)?;

    // 4. Inspect results
    println!("Conversion successful!");
    println!("Output: {:?}", result.output_path);
    println!("Size:   {}", result.formatted_output_size());
    if let Some(dur) = result.formatted_duration() {
        println!("Duration: {}", dur);
    }
    if let Some(dims) = result.formatted_dimensions() {
        println!("Resolution: {}", dims);
    }
    println!("Elapsed:  {:.2}s", result.elapsed.as_secs_f64());

    Ok(())
}
