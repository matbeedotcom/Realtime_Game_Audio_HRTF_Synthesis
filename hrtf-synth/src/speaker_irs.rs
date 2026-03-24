//! Speaker IR set — the bridge between HRTF synthesis and real-time APO processing.
//!
//! Extracts the 7 speaker-direction IRs (left + right ear) from a full HrtfData set,
//! and provides serialization for the APO to load at runtime.

use crate::io::hesuvi::{find_closest_position, DEFAULT_SPEAKERS};
use crate::HrtfData;

/// Magic bytes for the IR file format
const MAGIC: &[u8; 4] = b"HRIR";
/// Format version
const VERSION: u32 = 1;
/// Number of speaker directions (7.1 minus LFE)
const NUM_SPEAKERS: usize = 7;

/// Pre-computed speaker IRs for the APO.
///
/// Layout: for each of 7 speakers, left-ear IR followed by right-ear IR.
/// Total length = 7 * 2 * ir_length.
pub struct SpeakerIrSet {
    /// Flat IR data: [speaker][ear][tap]
    pub irs: Vec<f32>,
    /// IR length per ear per speaker
    pub ir_length: usize,
    /// Sample rate these IRs were generated at
    pub sample_rate: u32,
}

impl SpeakerIrSet {
    /// Extract speaker IRs from a full HrtfData set.
    ///
    /// Finds the closest synthesized direction for each of the 7 HeSuVi speaker
    /// positions and copies the left/right ear IRs.
    pub fn from_hrtf_data(data: &HrtfData) -> Self {
        let ir_length = data.ir_length();
        let mut irs = Vec::with_capacity(NUM_SPEAKERS * 2 * ir_length);

        for speaker in &DEFAULT_SPEAKERS {
            let dir_idx = find_closest_position(data, speaker.azimuth, speaker.elevation);

            // Left ear IR
            irs.extend_from_slice(data.get_ir(dir_idx, 0));
            // Right ear IR
            irs.extend_from_slice(data.get_ir(dir_idx, 1));
        }

        Self {
            irs,
            ir_length,
            sample_rate: data.sample_rate(),
        }
    }

    /// Save to the binary format expected by the APO.
    ///
    /// Format: [HRIR magic][version u32][n_speakers u32][ir_length u32][sample_rate u32][f32 data...]
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {e}"))?;
        }

        let mut output =
            Vec::with_capacity(4 + 4 * 4 + self.irs.len() * 4);

        // Header
        output.extend_from_slice(MAGIC);
        output.extend_from_slice(&VERSION.to_le_bytes());
        output.extend_from_slice(&(NUM_SPEAKERS as u32).to_le_bytes());
        output.extend_from_slice(&(self.ir_length as u32).to_le_bytes());
        output.extend_from_slice(&self.sample_rate.to_le_bytes());

        // Float data
        for &val in &self.irs {
            output.extend_from_slice(&val.to_le_bytes());
        }

        std::fs::write(path, &output).map_err(|e| format!("Failed to write IR file: {e}"))
    }

    /// Load from the binary format.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("Failed to read IR file: {e}"))?;

        // Header: 4 (magic) + 4*4 (fields) = 20 bytes
        if data.len() < 20 {
            return Err("IR file too small".into());
        }

        if &data[0..4] != MAGIC {
            return Err("Invalid IR file magic".into());
        }

        let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
        if version != VERSION {
            return Err(format!("Unsupported IR version: {version}"));
        }

        let n_speakers = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
        let ir_length = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
        let sample_rate = u32::from_le_bytes(data[16..20].try_into().unwrap());

        if n_speakers != NUM_SPEAKERS {
            return Err(format!("Expected {NUM_SPEAKERS} speakers, got {n_speakers}"));
        }

        let expected_floats = NUM_SPEAKERS * 2 * ir_length;
        let expected_bytes = 20 + expected_floats * 4;
        if data.len() < expected_bytes {
            return Err(format!(
                "IR file too small: expected {expected_bytes} bytes, got {}",
                data.len()
            ));
        }

        let mut irs = vec![0.0f32; expected_floats];
        for (i, chunk) in data[20..20 + expected_floats * 4].chunks_exact(4).enumerate() {
            irs[i] = f32::from_le_bytes(chunk.try_into().unwrap());
        }

        Ok(Self {
            irs,
            ir_length,
            sample_rate,
        })
    }
}
