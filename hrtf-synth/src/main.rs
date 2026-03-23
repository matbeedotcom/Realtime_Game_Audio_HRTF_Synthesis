//! HRTF Synthesizer CLI
//!
//! Command-line interface for generating personalized HRTFs from anthropometric measurements.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

use hrtf_synth::{Anthropometry, Config, HrtfSynthesizer};

#[derive(Parser, Debug)]
#[command(name = "hrtf-synth")]
#[command(author, version, about = "Synthesize personalized HRTFs from anthropometric measurements")]
struct Args {
    /// Path to the model file (.bin)
    #[arg(short, long)]
    model: PathBuf,

    /// Head width in cm (distance between ears)
    #[arg(long)]
    head_width: f32,

    /// Head depth in cm (front to back)
    #[arg(long)]
    head_depth: f32,

    /// Left ear parameters (6 values, comma-separated): d1,d2,d3,d5,d7,d8
    #[arg(long, value_delimiter = ',', num_args = 6)]
    ear_left: Vec<f32>,

    /// Right ear parameters (6 values, comma-separated): d1,d2,d3,d5,d7,d8
    #[arg(long, value_delimiter = ',', num_args = 6)]
    ear_right: Vec<f32>,

    /// Output file path
    #[arg(short, long)]
    output: PathBuf,

    /// Output format: wav (HeSuVi) or sofa
    #[arg(short, long, default_value = "wav")]
    format: String,

    /// Sample rate (44100, 48000, 96000)
    #[arg(short, long, default_value = "44100")]
    sample_rate: u32,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate ear parameters
    if args.ear_left.len() != 6 {
        anyhow::bail!(
            "Left ear requires exactly 6 parameters (d1,d2,d3,d5,d7,d8), got {}",
            args.ear_left.len()
        );
    }
    if args.ear_right.len() != 6 {
        anyhow::bail!(
            "Right ear requires exactly 6 parameters (d1,d2,d3,d5,d7,d8), got {}",
            args.ear_right.len()
        );
    }

    // Build anthropometry
    let anthro = Anthropometry {
        head_width: args.head_width,
        head_depth: args.head_depth,
        ear_params_left: args.ear_left.try_into().unwrap(),
        ear_params_right: args.ear_right.try_into().unwrap(),
    };

    if args.verbose {
        println!("Loading model from: {}", args.model.display());
    }

    // Load model
    let config = Config {
        sample_rate: args.sample_rate,
        ..Default::default()
    };

    let synthesizer =
        HrtfSynthesizer::load(&args.model, config).context("Failed to load model")?;

    if args.verbose {
        println!("Model loaded: {} directions", synthesizer.num_directions());
        println!("Anthropometry:");
        println!("  Head: {:.1} x {:.1} cm", anthro.head_width, anthro.head_depth);
        println!("  Left ear:  {:?}", anthro.ear_params_left);
        println!("  Right ear: {:?}", anthro.ear_params_right);
        println!();
    }

    // Create progress bar
    let pb = ProgressBar::new(synthesizer.num_directions() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Synthesize HRTFs
    println!("Synthesizing HRTFs...");
    let hrtf_data = synthesizer.synthesize_with_progress(&anthro, |_| {
        pb.inc(1);
    });
    pb.finish_with_message("Done!");

    // Export
    println!("Exporting to: {}", args.output.display());
    match args.format.to_lowercase().as_str() {
        "wav" | "hesuvi" => {
            hrtf_synth::io::hesuvi::write_hesuvi(&args.output, &hrtf_data, args.sample_rate)
                .context("Failed to write HeSuVi WAV")?;
        }
        #[cfg(feature = "sofa")]
        "sofa" => {
            hrtf_synth::io::sofa::write_sofa(&args.output, &hrtf_data)
                .context("Failed to write SOFA")?;
        }
        #[cfg(not(feature = "sofa"))]
        "sofa" => {
            anyhow::bail!("SOFA output requires the 'sofa' feature. Rebuild with --features sofa");
        }
        _ => {
            anyhow::bail!("Unknown format: {}. Use 'wav' or 'sofa'", args.format);
        }
    }

    println!("Complete!");
    Ok(())
}
