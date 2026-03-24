//! HRTF Synthesizer - main synthesis pipeline

use std::path::Path;

use nalgebra::DVector;
#[cfg(feature = "cli")]
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::dsp::{
    apply_fractional_delay, calculate_itd_adaptive, itd_to_samples, minimum_phase_from_magnitude,
};
use crate::nn::HrtfModel;

/// Synthesizer errors
#[derive(Debug, Error)]
pub enum SynthesizerError {
    #[error("Model error: {0}")]
    Model(#[from] crate::nn::hrtf_model::ModelError),

    #[error("Invalid anthropometry: {0}")]
    InvalidAnthropometry(String),
}

/// Synthesis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Output sample rate
    pub sample_rate: u32,
    /// IR length (samples)
    pub ir_length: usize,
    /// Apply low-frequency extension
    pub apply_lfe: bool,
    /// LFE minimum frequency (Hz)
    pub lfe_min_freq: f32,
    /// LFE maximum frequency (Hz)
    pub lfe_max_freq: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            ir_length: 200, // 100 freq bins * 2
            apply_lfe: false, // Not implemented yet
            lfe_min_freq: 15.0,
            lfe_max_freq: 500.0,
        }
    }
}

/// Anthropometric measurements for HRTF personalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anthropometry {
    /// Head width in cm (distance between ears)
    pub head_width: f32,
    /// Head depth in cm (front to back)
    pub head_depth: f32,
    /// Left ear parameters: [d1, d2, d3, d5, d7, d8]
    pub ear_params_left: [f32; 6],
    /// Right ear parameters: [d1, d2, d3, d5, d7, d8]
    pub ear_params_right: [f32; 6],
}

impl Anthropometry {
    /// Build the input vector for the neural network (left channel)
    pub fn to_input_left(&self) -> DVector<f32> {
        DVector::from_vec(vec![
            self.head_width,
            self.head_depth,
            self.ear_params_left[0], // d1
            self.ear_params_left[1], // d2
            self.ear_params_left[2], // d3
            self.ear_params_left[3], // d5
            self.ear_params_left[4], // d7
            self.ear_params_left[5], // d8
        ])
    }

    /// Build the input vector for the neural network (right channel)
    pub fn to_input_right(&self) -> DVector<f32> {
        DVector::from_vec(vec![
            self.head_width,
            self.head_depth,
            self.ear_params_right[0], // d1
            self.ear_params_right[1], // d2
            self.ear_params_right[2], // d3
            self.ear_params_right[3], // d5
            self.ear_params_right[4], // d7
            self.ear_params_right[5], // d8
        ])
    }
}

/// Container for synthesized HRTF data
pub struct HrtfData {
    /// Impulse responses [direction][channel][sample]
    irs: Vec<Vec<Vec<f32>>>,
    /// Source positions [azimuth, elevation, distance]
    positions: Vec<[f32; 3]>,
    /// Sample rate
    sample_rate: u32,
}

impl HrtfData {
    /// Create new HRTF data container
    pub fn new(n_directions: usize, n_samples: usize, sample_rate: u32) -> Self {
        Self {
            irs: vec![vec![vec![0.0; n_samples]; 2]; n_directions],
            positions: vec![[0.0; 3]; n_directions],
            sample_rate,
        }
    }

    /// Get the number of directions
    pub fn num_directions(&self) -> usize {
        self.irs.len()
    }

    /// Get the IR length in samples
    pub fn ir_length(&self) -> usize {
        if self.irs.is_empty() || self.irs[0].is_empty() {
            0
        } else {
            self.irs[0][0].len()
        }
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get all positions
    pub fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }

    /// Set position for a direction
    pub fn set_position(&mut self, dir_idx: usize, azimuth: f32, elevation: f32, distance: f32) {
        self.positions[dir_idx] = [azimuth, elevation, distance];
    }

    /// Set IR for a direction and channel
    pub fn set_ir(&mut self, dir_idx: usize, channel: usize, ir: Vec<f32>) {
        self.irs[dir_idx][channel] = ir;
    }

    /// Get a sample from the IR
    pub fn get_sample(&self, dir_idx: usize, channel: usize, sample_idx: usize) -> f32 {
        self.irs[dir_idx][channel][sample_idx]
    }

    /// Get IR for a direction and channel
    pub fn get_ir(&self, dir_idx: usize, channel: usize) -> &[f32] {
        &self.irs[dir_idx][channel]
    }
}

/// HRTF Synthesizer
pub struct HrtfSynthesizer {
    model: HrtfModel,
    config: Config,
}

impl HrtfSynthesizer {
    /// Load a synthesizer from a model file
    pub fn load(model_path: &Path, config: Config) -> Result<Self, SynthesizerError> {
        let model = HrtfModel::load(model_path)?;
        Ok(Self { model, config })
    }

    /// Get the number of spatial directions
    pub fn num_directions(&self) -> usize {
        self.model.num_directions()
    }

    /// Synthesize HRTFs for all directions
    pub fn synthesize(&self, anthro: &Anthropometry) -> HrtfData {
        self.synthesize_with_progress(anthro, |_| {})
    }

    /// Synthesize HRTFs with progress callback
    pub fn synthesize_with_progress<F>(&self, anthro: &Anthropometry, progress: F) -> HrtfData
    where
        F: Fn(usize) + Sync,
    {
        let n_dirs = self.model.num_directions();
        let ir_len = self.model.num_freq_bins() * 2; // Full spectrum length

        let mut hrtf_data = HrtfData::new(n_dirs, ir_len, self.config.sample_rate);

        // Copy positions
        for (i, pos) in self.model.positions().iter().enumerate() {
            hrtf_data.set_position(i, pos[0], pos[1], pos[2]);
        }

        // Build input vectors
        let input_left = anthro.to_input_left();
        let input_right = anthro.to_input_right();

        // Synthesize each direction
        #[cfg(feature = "cli")]
        let results: Vec<_> = (0..n_dirs)
            .into_par_iter()
            .map(|dir_idx| {
                let (ir_left, ir_right) =
                    self.synthesize_direction(dir_idx, &input_left, &input_right, anthro);
                progress(dir_idx);
                (dir_idx, ir_left, ir_right)
            })
            .collect();

        #[cfg(not(feature = "cli"))]
        let results: Vec<_> = (0..n_dirs)
            .map(|dir_idx| {
                let (ir_left, ir_right) =
                    self.synthesize_direction(dir_idx, &input_left, &input_right, anthro);
                progress(dir_idx);
                (dir_idx, ir_left, ir_right)
            })
            .collect();

        // Store results
        for (dir_idx, ir_left, ir_right) in results {
            hrtf_data.set_ir(dir_idx, 0, ir_left);
            hrtf_data.set_ir(dir_idx, 1, ir_right);
        }

        // Normalize
        let peak = self.find_peak(&hrtf_data);
        if peak > 0.0 {
            self.normalize(&mut hrtf_data, peak);
        }

        hrtf_data
    }

    /// Synthesize a single direction
    fn synthesize_direction(
        &self,
        dir_idx: usize,
        input_left: &DVector<f32>,
        input_right: &DVector<f32>,
        anthro: &Anthropometry,
    ) -> (Vec<f32>, Vec<f32>) {
        // 1. Neural network inference → PCA scores
        // 2. PCA reconstruction → magnitude spectrum
        let mag_left = self.model.predict_magnitude(input_left, dir_idx, 0);
        let mag_right = self.model.predict_magnitude(input_right, dir_idx, 1);

        // 3. Minimum phase reconstruction → IR
        let mut ir_left = minimum_phase_from_magnitude(mag_left.as_slice());
        let mut ir_right = minimum_phase_from_magnitude(mag_right.as_slice());

        // 4. Calculate and apply ITD
        let (az, el, _) = self.model.get_position(dir_idx);
        let itd_sec = calculate_itd_adaptive(
            az as f64,
            el as f64,
            anthro.head_width as f64,
            anthro.head_depth as f64,
            self.model.itd_ref(),
        );

        let itd_samples = itd_to_samples(itd_sec, self.config.sample_rate);
        let offset = 10.0; // Base offset in samples

        // Determine which ear gets the delay based on azimuth
        let (left_delay, right_delay) = if az >= 180.0 {
            (itd_samples + offset, offset)
        } else {
            (offset, itd_samples + offset)
        };

        apply_fractional_delay(&mut ir_left, left_delay);
        apply_fractional_delay(&mut ir_right, right_delay);

        (ir_left, ir_right)
    }

    /// Find peak amplitude across all IRs
    fn find_peak(&self, data: &HrtfData) -> f32 {
        let mut peak = 0.0f32;
        for dir_idx in 0..data.num_directions() {
            for ch in 0..2 {
                for &sample in data.get_ir(dir_idx, ch) {
                    peak = peak.max(sample.abs());
                }
            }
        }
        peak
    }

    /// Normalize all IRs
    fn normalize(&self, data: &mut HrtfData, peak: f32) {
        let scale = 1.0 / peak;
        for dir_idx in 0..data.num_directions() {
            for ch in 0..2 {
                let ir = &mut data.irs[dir_idx][ch];
                for sample in ir.iter_mut() {
                    *sample *= scale;
                }
            }
        }
    }
}
