//! Real-time HRTF convolution engine
//!
//! Zero-allocation, cache-friendly direct convolution for the APO real-time path.
//! Each of 7 surround speaker channels is convolved with a left-ear and right-ear IR,
//! then summed to produce binaural stereo output. LFE is mixed equally to both ears.

use crate::channel_map::{ACTIVE_SPEAKERS, LFE_GAIN};

/// Real-time HRTF convolver — all memory pre-allocated, no heap ops in `process()`.
pub struct HrtfConvolver {
    /// Flat IR storage: [speaker_idx * 2 * ir_length + ear * ir_length + tap]
    /// speaker_idx: 0..7 active speakers (FL, FR, FC, RL, RR, SL, SR)
    /// ear: 0=left, 1=right
    irs: Box<[f32]>,

    /// Circular delay lines: [speaker_idx * ir_length + sample]
    delay_lines: Box<[f32]>,

    /// Current write position in the circular buffers (shared across all speakers)
    write_pos: usize,

    /// IR length in samples (typically 200)
    ir_length: usize,
}

impl HrtfConvolver {
    /// Create a new convolver from pre-computed speaker IRs.
    ///
    /// `irs_flat` layout: for each of 7 speakers, left-ear IR followed by right-ear IR
    /// (total length = 7 * 2 * ir_length)
    pub fn new(irs_flat: Box<[f32]>, ir_length: usize) -> Self {
        assert_eq!(irs_flat.len(), ACTIVE_SPEAKERS * 2 * ir_length);

        let delay_lines = vec![0.0f32; ACTIVE_SPEAKERS * ir_length].into_boxed_slice();

        Self {
            irs: irs_flat,
            delay_lines,
            write_pos: 0,
            ir_length,
        }
    }

    /// Process a buffer of interleaved 8-channel (7.1) input into interleaved 2-channel output.
    ///
    /// # Safety contract (for RT path)
    /// - No heap allocations
    /// - No syscalls
    /// - No panics (bounds are guaranteed by construction)
    ///
    /// `input`:  interleaved 8ch float samples, length = n_frames * 8
    /// `output`: interleaved 2ch float samples, length = n_frames * 2
    pub fn process(&mut self, input: &[f32], output: &mut [f32], n_frames: usize) {
        let ir_len = self.ir_length;

        for frame in 0..n_frames {
            let in_base = frame * 8;
            let out_base = frame * 2;

            let mut out_left: f32 = 0.0;
            let mut out_right: f32 = 0.0;

            // LFE channel (index 3) — mix equally to both ears
            let lfe_sample = input[in_base + 3] * LFE_GAIN;
            out_left += lfe_sample;
            out_right += lfe_sample;

            // For each active speaker channel, write to delay line and convolve
            for (speaker_idx, &input_ch) in crate::channel_map::CHANNEL_TO_SPEAKER.iter().enumerate() {
                let sample = input[in_base + input_ch];

                // Write to circular delay line
                let dl_base = speaker_idx * ir_len;
                self.delay_lines[dl_base + self.write_pos] = sample;

                // Convolve with left-ear and right-ear IRs
                let ir_base_l = (speaker_idx * 2) * ir_len;
                let ir_base_r = (speaker_idx * 2 + 1) * ir_len;

                let mut sum_l: f32 = 0.0;
                let mut sum_r: f32 = 0.0;

                // Direct time-domain convolution using the circular delay line
                // y[n] = sum_{k=0}^{N-1} h[k] * x[n-k]
                for k in 0..ir_len {
                    // Read from delay line with wraparound
                    let read_pos = if self.write_pos >= k {
                        self.write_pos - k
                    } else {
                        ir_len - (k - self.write_pos)
                    };

                    let delayed_sample = self.delay_lines[dl_base + read_pos];
                    sum_l += self.irs[ir_base_l + k] * delayed_sample;
                    sum_r += self.irs[ir_base_r + k] * delayed_sample;
                }

                out_left += sum_l;
                out_right += sum_r;
            }

            output[out_base] = out_left;
            output[out_base + 1] = out_right;

            // Advance write position per-sample (not per-buffer)
            self.write_pos = (self.write_pos + 1) % ir_len;
        }
    }

    /// Returns pointer and byte-size of the IR buffer, for VirtualLock.
    pub fn ir_buffer_info(&self) -> (*const f32, usize) {
        (self.irs.as_ptr(), self.irs.len() * std::mem::size_of::<f32>())
    }

    /// Returns pointer and byte-size of the delay line buffer, for VirtualLock.
    pub fn delay_line_buffer_info(&self) -> (*const f32, usize) {
        (self.delay_lines.as_ptr(), self.delay_lines.len() * std::mem::size_of::<f32>())
    }

    /// Process 8-channel interleaved audio in-place.
    /// Reads all 8 channels, computes binaural L/R, writes to ch 0-1, zeros ch 2-7.
    /// Safe: all channel reads into delay lines happen before any buffer writes.
    pub fn process_inplace(&mut self, buffer: &mut [f32], n_frames: usize) {
        let ir_len = self.ir_length;

        for frame in 0..n_frames {
            let base = frame * 8;

            let mut out_left: f32 = 0.0;
            let mut out_right: f32 = 0.0;

            // LFE channel (index 3) — mix equally to both ears
            let lfe_sample = buffer[base + 3] * LFE_GAIN;
            out_left += lfe_sample;
            out_right += lfe_sample;

            // Read all speaker channels into delay lines first
            for (speaker_idx, &input_ch) in crate::channel_map::CHANNEL_TO_SPEAKER.iter().enumerate() {
                let sample = buffer[base + input_ch];

                // Write to circular delay line
                let dl_base = speaker_idx * ir_len;
                self.delay_lines[dl_base + self.write_pos] = sample;

                // Convolve with left-ear and right-ear IRs
                let ir_base_l = (speaker_idx * 2) * ir_len;
                let ir_base_r = (speaker_idx * 2 + 1) * ir_len;

                let mut sum_l: f32 = 0.0;
                let mut sum_r: f32 = 0.0;

                for k in 0..ir_len {
                    let read_pos = if self.write_pos >= k {
                        self.write_pos - k
                    } else {
                        ir_len - (k - self.write_pos)
                    };
                    let delayed_sample = self.delay_lines[dl_base + read_pos];
                    sum_l += self.irs[ir_base_l + k] * delayed_sample;
                    sum_r += self.irs[ir_base_r + k] * delayed_sample;
                }

                out_left += sum_l;
                out_right += sum_r;
            }

            // Write binaural output to channels 0-1
            buffer[base] = out_left;
            buffer[base + 1] = out_right;
            // Zero channels 2-7
            for ch in 2..8 {
                buffer[base + ch] = 0.0;
            }

            self.write_pos = (self.write_pos + 1) % ir_len;
        }
    }

    /// Reset all delay lines to zero (e.g., on format change)
    pub fn reset(&mut self) {
        self.delay_lines.fill(0.0);
        self.write_pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Impulse IR for FR speaker, left ear only — send signal on input ch 1 (FR)
    #[test]
    fn test_impulse_passthrough() {
        let ir_len = 4;
        // 7 speakers × 2 ears × 4 taps = 56 floats
        let mut irs = vec![0.0f32; ACTIVE_SPEAKERS * 2 * ir_len];
        // Speaker 0 = FR (input ch 1), left ear, tap 0 = 1.0 (impulse)
        irs[0] = 1.0;

        let mut conv = HrtfConvolver::new(irs.into_boxed_slice(), ir_len);

        // Single frame: FR (ch 1) = 1.0, all others = 0.0
        let input = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0f32];
        let mut output = [0.0f32; 2];

        conv.process(&input, &mut output, 1);

        assert!((output[0] - 1.0).abs() < 1e-6, "Left should be 1.0, got {}", output[0]);
        assert!((output[1]).abs() < 1e-6, "Right should be 0.0, got {}", output[1]);
    }

    /// Multi-frame: verify delay line advances correctly across frames
    #[test]
    fn test_multi_frame_delay() {
        let ir_len = 4;
        let mut irs = vec![0.0f32; ACTIVE_SPEAKERS * 2 * ir_len];
        // Speaker 0 (FR), left ear: impulse at tap 1 (1-sample delay)
        irs[1] = 1.0;

        let mut conv = HrtfConvolver::new(irs.into_boxed_slice(), ir_len);

        // 3 frames: FR=1.0 on frame 0, then 0.0
        let input = [
            0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // frame 0: FR=1.0
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // frame 1: all zero
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // frame 2: all zero
        ];
        let mut output = [0.0f32; 6]; // 3 frames × 2 channels

        conv.process(&input, &mut output, 3);

        // With 1-sample delay IR, output should be delayed by 1 frame
        assert!((output[0]).abs() < 1e-6, "Frame 0 left should be 0, got {}", output[0]);
        assert!((output[2] - 1.0).abs() < 1e-6, "Frame 1 left should be 1.0, got {}", output[2]);
        assert!((output[4]).abs() < 1e-6, "Frame 2 left should be 0, got {}", output[4]);
    }

    /// LFE should mix equally to both channels
    #[test]
    fn test_lfe_passthrough() {
        let ir_len = 4;
        let irs = vec![0.0f32; ACTIVE_SPEAKERS * 2 * ir_len];
        let mut conv = HrtfConvolver::new(irs.into_boxed_slice(), ir_len);

        // LFE only (channel 3)
        let input = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0f32];
        let mut output = [0.0f32; 2];

        conv.process(&input, &mut output, 1);

        assert!((output[0] - LFE_GAIN).abs() < 1e-6);
        assert!((output[1] - LFE_GAIN).abs() < 1e-6);
    }
}
