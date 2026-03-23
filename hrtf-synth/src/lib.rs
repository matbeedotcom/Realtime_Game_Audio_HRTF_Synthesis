//! HRTF Synthesizer Library
//!
//! Synthesizes personalized Head-Related Transfer Functions (HRTFs) from
//! anthropometric measurements using pre-trained neural networks and PCA reconstruction.

pub mod dsp;
#[cfg(feature = "gui")]
pub mod gui;
pub mod io;
pub mod nn;
pub mod synthesizer;

pub use synthesizer::{Anthropometry, Config, HrtfData, HrtfSynthesizer};
