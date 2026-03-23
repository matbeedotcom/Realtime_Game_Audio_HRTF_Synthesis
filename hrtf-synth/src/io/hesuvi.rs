//! HeSuVi WAV format export

use std::path::Path;

use hound::{SampleFormat, WavSpec, WavWriter};
use thiserror::Error;

use crate::HrtfData;

/// HeSuVi export errors
#[derive(Debug, Error)]
pub enum HeSuViError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WAV error: {0}")]
    Wav(#[from] hound::Error),

    #[error("Invalid speaker configuration")]
    InvalidSpeakers,
}

/// Speaker configuration for HeSuVi
#[derive(Debug, Clone, Copy)]
pub struct SpeakerConfig {
    pub azimuth: f32,
    pub elevation: f32,
}

/// Default 7-speaker layout for HeSuVi
pub const DEFAULT_SPEAKERS: [SpeakerConfig; 7] = [
    SpeakerConfig { azimuth: 30.0, elevation: 0.0 },   // FR (Front Right)
    SpeakerConfig { azimuth: 110.0, elevation: 0.0 },  // SR (Side Right)
    SpeakerConfig { azimuth: 150.0, elevation: 0.0 },  // RR (Rear Right)
    SpeakerConfig { azimuth: 0.0, elevation: 0.0 },    // FC (Front Center)
    SpeakerConfig { azimuth: 330.0, elevation: 0.0 },  // FL (Front Left)
    SpeakerConfig { azimuth: 250.0, elevation: 0.0 },  // SL (Side Left)
    SpeakerConfig { azimuth: 210.0, elevation: 0.0 },  // RL (Rear Left)
];

/// Write HRTF data to HeSuVi WAV format
///
/// HeSuVi format is a 14-channel WAV file containing impulse responses
/// for 7 speaker positions × 2 ears (left/right).
///
/// Channel order: FR_L, FR_R, SR_L, SR_R, RR_L, RR_R, FC_L, FC_R, FL_L, FL_R, SL_L, SL_R, RL_L, RL_R
pub fn write_hesuvi(
    path: &Path,
    data: &HrtfData,
    sample_rate: u32,
) -> Result<(), HeSuViError> {
    write_hesuvi_custom(path, data, sample_rate, &DEFAULT_SPEAKERS)
}

/// Write HRTF data to HeSuVi WAV format with custom speaker positions
pub fn write_hesuvi_custom(
    path: &Path,
    data: &HrtfData,
    sample_rate: u32,
    speakers: &[SpeakerConfig],
) -> Result<(), HeSuViError> {
    if speakers.len() != 7 {
        return Err(HeSuViError::InvalidSpeakers);
    }

    // Find closest HRTF positions for each speaker
    let mut speaker_indices = Vec::with_capacity(7);
    for speaker in speakers {
        let idx = find_closest_position(data, speaker.azimuth, speaker.elevation);
        speaker_indices.push(idx);
    }

    // Determine IR length
    let ir_len = data.ir_length();

    // WAV spec: 14 channels (7 speakers × 2 ears), 32-bit float
    let spec = WavSpec {
        channels: 14,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let mut writer = WavWriter::create(path, spec)?;

    // Find peak for normalization
    let mut peak = 0.0f32;
    for &idx in &speaker_indices {
        for sample_idx in 0..ir_len {
            let l = data.get_sample(idx, 0, sample_idx).abs();
            let r = data.get_sample(idx, 1, sample_idx).abs();
            peak = peak.max(l).max(r);
        }
    }

    let norm_factor = if peak > 0.0 { 0.975 / peak } else { 1.0 };

    // Write samples (interleaved)
    for sample_idx in 0..ir_len {
        for &pos_idx in &speaker_indices {
            // Left ear
            let left = data.get_sample(pos_idx, 0, sample_idx) * norm_factor;
            writer.write_sample(left)?;

            // Right ear
            let right = data.get_sample(pos_idx, 1, sample_idx) * norm_factor;
            writer.write_sample(right)?;
        }
    }

    writer.finalize()?;
    Ok(())
}

/// Find the closest HRTF position to a given azimuth/elevation
fn find_closest_position(data: &HrtfData, azimuth: f32, elevation: f32) -> usize {
    let mut min_dist = f32::MAX;
    let mut min_idx = 0;

    for (i, pos) in data.positions().iter().enumerate() {
        let d_az = pos[0] - azimuth;
        let d_el = pos[1] - elevation;
        let dist = d_az * d_az + d_el * d_el;

        if dist < min_dist {
            min_dist = dist;
            min_idx = i;
        }
    }

    min_idx
}
