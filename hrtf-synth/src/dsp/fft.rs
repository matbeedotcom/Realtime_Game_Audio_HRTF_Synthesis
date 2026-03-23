//! FFT processing utilities

use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

/// FFT processor with cached plans for efficiency
pub struct FftProcessor {
    forward_fft: Arc<dyn Fft<f32>>,
    inverse_fft: Arc<dyn Fft<f32>>,
    size: usize,
}

impl FftProcessor {
    /// Create a new FFT processor for the given size
    pub fn new(size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let forward_fft = planner.plan_fft_forward(size);
        let inverse_fft = planner.plan_fft_inverse(size);

        Self {
            forward_fft,
            inverse_fft,
            size,
        }
    }

    /// Get the FFT size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Perform forward FFT in-place
    pub fn forward_inplace(&self, buffer: &mut [Complex<f32>]) {
        assert_eq!(buffer.len(), self.size);
        self.forward_fft.process(buffer);
    }

    /// Perform inverse FFT in-place
    pub fn inverse_inplace(&self, buffer: &mut [Complex<f32>]) {
        assert_eq!(buffer.len(), self.size);
        self.inverse_fft.process(buffer);

        // Normalize by 1/N
        let scale = 1.0 / self.size as f32;
        for x in buffer.iter_mut() {
            *x *= scale;
        }
    }

    /// Forward FFT from real input to complex output
    pub fn forward(&self, input: &[f32]) -> Vec<Complex<f32>> {
        assert_eq!(input.len(), self.size);

        let mut buffer: Vec<Complex<f32>> = input.iter().map(|&x| Complex::new(x, 0.0)).collect();

        self.forward_fft.process(&mut buffer);
        buffer
    }

    /// Inverse FFT from complex input to real output
    pub fn inverse(&self, input: &[Complex<f32>]) -> Vec<f32> {
        assert_eq!(input.len(), self.size);

        let mut buffer = input.to_vec();
        self.inverse_fft.process(&mut buffer);

        // Normalize and take real part
        let scale = 1.0 / self.size as f32;
        buffer.iter().map(|x| x.re * scale).collect()
    }

    /// Real-to-complex FFT (returns only positive frequencies + DC + Nyquist)
    pub fn rfft(&self, input: &[f32]) -> Vec<Complex<f32>> {
        let full = self.forward(input);
        // Return DC to Nyquist (N/2 + 1 points)
        full[..self.size / 2 + 1].to_vec()
    }

    /// Complex-to-real inverse FFT
    /// Input should have N/2 + 1 points (DC to Nyquist)
    pub fn irfft(&self, input: &[Complex<f32>]) -> Vec<f32> {
        assert_eq!(input.len(), self.size / 2 + 1);

        // Reconstruct full spectrum with conjugate symmetry
        let mut full: Vec<Complex<f32>> = Vec::with_capacity(self.size);

        // DC to Nyquist
        full.extend_from_slice(input);

        // Nyquist+1 to N-1 (conjugate mirror of 1 to N/2-1)
        for i in (1..self.size / 2).rev() {
            full.push(input[i].conj());
        }

        self.inverse(&full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_forward_inverse_roundtrip() {
        let fft = FftProcessor::new(64);
        let input: Vec<f32> = (0..64).map(|i| (i as f32).sin()).collect();

        let freq = fft.forward(&input);
        let output = fft.inverse(&freq);

        for (a, b) in input.iter().zip(output.iter()) {
            assert_relative_eq!(a, b, epsilon = 1e-5);
        }
    }

    #[test]
    fn test_rfft_irfft_roundtrip() {
        let fft = FftProcessor::new(64);
        let input: Vec<f32> = (0..64).map(|i| (i as f32 * 0.1).cos()).collect();

        let freq = fft.rfft(&input);
        assert_eq!(freq.len(), 33); // N/2 + 1

        let output = fft.irfft(&freq);

        for (a, b) in input.iter().zip(output.iter()) {
            assert_relative_eq!(a, b, epsilon = 1e-5);
        }
    }
}
