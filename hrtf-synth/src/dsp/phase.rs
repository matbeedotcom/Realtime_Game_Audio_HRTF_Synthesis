//! Minimum phase reconstruction from magnitude spectrum

use super::{hilbert_transform, FftProcessor};
use num_complex::Complex;

/// Reconstruct a minimum-phase impulse response from a log-magnitude spectrum.
///
/// This implements the algorithm used in the MATLAB code:
/// 1. Convert dB to linear: `mag = 10^(mag_db/20)`
/// 2. Mirror spectrum: `[mag, flip(mag)]`
/// 3. Compute minimum phase: `phi = imag(hilbert(-log(mag + eps)))`
/// 4. Create complex spectrum: `H = mag * exp(j*phi)`
/// 5. IFFT to time domain
///
/// # Arguments
/// * `magnitude_db` - Log magnitude spectrum (single-sided, from DC to just before Nyquist)
///
/// # Returns
/// Time-domain impulse response with length 2 * input_len
pub fn minimum_phase_from_magnitude(magnitude_db: &[f32]) -> Vec<f32> {
    let half_len = magnitude_db.len();
    let n = half_len * 2;

    // Convert dB to linear magnitude
    let magnitude: Vec<f32> = magnitude_db.iter().map(|&db| 10.0_f32.powf(db / 20.0)).collect();

    // Mirror spectrum (create full symmetric spectrum)
    // [mag[0], mag[1], ..., mag[N/2-1], mag[N/2-1], mag[N/2-2], ..., mag[1]]
    let mut full_mag = magnitude.clone();
    for i in (1..half_len).rev() {
        full_mag.push(magnitude[i]);
    }

    // Pad to next power of 2 for FFT (Hilbert transform requires power-of-2 length)
    let fft_len = full_mag.len().next_power_of_two();
    full_mag.resize(fft_len, 0.0);

    // Compute minimum phase using Hilbert transform
    // phi = imag(hilbert(-log(|H| + eps)))
    let eps = 1e-10_f32;
    let log_mag: Vec<f32> = full_mag.iter().map(|&m| -(m + eps).ln()).collect();

    let min_phase = hilbert_transform(&log_mag);

    // Create complex spectrum with magnitude and minimum phase
    // H = |H| * exp(j * phi)
    let fft = FftProcessor::new(fft_len);
    let mut spectrum: Vec<Complex<f32>> = full_mag
        .iter()
        .zip(min_phase.iter())
        .map(|(&mag, &phase)| Complex::from_polar(mag, phase))
        .collect();

    // Inverse FFT to get time-domain IR
    fft.inverse_inplace(&mut spectrum);

    // Return real part, truncated to original expected length
    spectrum.iter().take(n).map(|c| c.re).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimum_phase_basic() {
        // Create a simple flat magnitude response
        let mag_db = vec![0.0_f32; 64];

        let ir = minimum_phase_from_magnitude(&mag_db);

        // Should be an impulse (all energy at start)
        assert_eq!(ir.len(), 128);

        // First sample should have most energy
        let max_idx = ir
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.abs().partial_cmp(&b.abs()).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        // Peak should be near the beginning
        assert!(max_idx < 10);
    }

    #[test]
    fn test_minimum_phase_lowpass() {
        // Create a lowpass-like magnitude response
        let mut mag_db = vec![0.0_f32; 64];
        for i in 32..64 {
            mag_db[i] = -40.0; // Attenuate high frequencies
        }

        let ir = minimum_phase_from_magnitude(&mag_db);

        // Should be a causal filter
        assert_eq!(ir.len(), 128);
    }
}
