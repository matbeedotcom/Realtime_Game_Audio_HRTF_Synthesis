//! Background synthesis thread management

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::{Anthropometry, Config, HrtfSynthesizer};

/// Synthesis state machine
pub enum SynthState {
    Idle,
    Running {
        progress: Arc<AtomicUsize>,
        total: usize,
        handle: Option<JoinHandle<Result<(), String>>>,
    },
    Complete {
        message: String,
    },
    Error {
        message: String,
    },
}

impl SynthState {
    /// Check if synthesis thread has finished and transition state
    pub fn poll(&mut self) {
        let finished = matches!(self, SynthState::Running { handle, .. } if handle.as_ref().map_or(false, |h| h.is_finished()));

        if finished {
            if let SynthState::Running { handle, .. } = std::mem::replace(self, SynthState::Idle) {
                match handle.unwrap().join() {
                    Ok(Ok(())) => {
                        *self = SynthState::Complete {
                            message: "HRTF synthesis complete! WAV file saved.".into(),
                        };
                    }
                    Ok(Err(e)) => {
                        *self = SynthState::Error { message: e };
                    }
                    Err(_) => {
                        *self = SynthState::Error {
                            message: "Synthesis thread panicked.".into(),
                        };
                    }
                }
            }
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self, SynthState::Running { .. })
    }
}

/// Launch synthesis on a background thread
pub fn start(
    model_path: PathBuf,
    sample_rate: u32,
    head_width: f32,
    head_depth: f32,
    ear_left: [f32; 6],
    ear_right: [f32; 6],
    output_path: PathBuf,
) -> SynthState {
    let progress = Arc::new(AtomicUsize::new(0));
    let progress_clone = Arc::clone(&progress);

    let handle = std::thread::spawn(move || -> Result<(), String> {
        let config = Config {
            sample_rate,
            ..Default::default()
        };

        let synthesizer =
            HrtfSynthesizer::load(&model_path, config).map_err(|e| format!("Failed to load model: {e}"))?;

        let anthro = Anthropometry {
            head_width,
            head_depth,
            ear_params_left: ear_left,
            ear_params_right: ear_right,
        };

        let hrtf_data = synthesizer.synthesize_with_progress(&anthro, |_| {
            progress_clone.fetch_add(1, Ordering::Relaxed);
        });

        crate::io::hesuvi::write_hesuvi(&output_path, &hrtf_data, sample_rate)
            .map_err(|e| format!("Failed to write WAV: {e}"))?;

        Ok(())
    });

    // We need to know the total directions count. Since we can't know it before loading
    // the model (which happens on the thread), we use a reasonable default.
    // The model has 648 directions.
    SynthState::Running {
        progress,
        total: 648,
        handle: Some(handle),
    }
}
