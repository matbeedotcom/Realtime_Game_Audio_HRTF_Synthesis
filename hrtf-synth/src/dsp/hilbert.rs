//! Hilbert transform implementation

use super::FftProcessor;
use num_complex::Complex;

/// Compute the Hilbert transform of a real signal.
///
/// The Hilbert transform returns the imaginary part of the analytic signal.
/// This is used for minimum-phase reconstruction where:
/// `phase = Hilbert{-log(magnitude)}`
///
/// # Algorithm
/// 1. Compute FFT of input
/// 2. Zero DC and Nyquist
/// 3. Double positive frequencies (indices 1 to N/2-1)
/// 4. Zero negative frequencies (indices N/2+1 to N-1)
/// 5. Compute IFFT
/// 6. Return imaginary part
pub fn hilbert_transform(input: &[f32]) -> Vec<f32> {
    let n = input.len();
    assert!(n > 0 && n.is_power_of_two(), "Input length must be a power of 2");

    let fft = FftProcessor::new(n);

    // Forward FFT
    let mut spectrum = fft.forward(input);

    // Apply Hilbert transform in frequency domain
    // DC component (index 0): multiply by 0 (set to 0)
    spectrum[0] = Complex::new(0.0, 0.0);

    // Positive frequencies (1 to N/2-1): multiply by 2
    for i in 1..n / 2 {
        spectrum[i] *= 2.0;
    }

    // Nyquist (index N/2): multiply by 0 (set to 0)
    spectrum[n / 2] = Complex::new(0.0, 0.0);

    // Negative frequencies (N/2+1 to N-1): multiply by 0
    for i in (n / 2 + 1)..n {
        spectrum[i] = Complex::new(0.0, 0.0);
    }

    // The analytic signal is x + j*H{x}
    // We construct it by zeroing negative frequencies

    // DC stays the same
    // Positive frequencies (1 to N/2-1): multiply by 2
    for i in 1..n / 2 {
        spectrum[i] *= 2.0;
    }

    // Nyquist stays the same
    // Negative frequencies: set to 0
    for i in (n / 2 + 1)..n {
        spectrum[i] = Complex::new(0.0, 0.0);
    }

    // Inverse FFT gives analytic signal
    let mut analytic_buffer = spectrum;
    fft.inverse_inplace(&mut analytic_buffer);

    // Return imaginary part (the Hilbert transform)
    analytic_buffer.iter().map(|c| c.im).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::f32::consts::PI;

    #[test]
    fn test_hilbert_sine() {
        // Hilbert transform of sin(t) should be -cos(t)
        let n = 256;
        let input: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * i as f32 / n as f32).sin())
            .collect();

        let hilbert = hilbert_transform(&input);

        let expected: Vec<f32> = (0..n)
            .map(|i| -(2.0 * PI * i as f32 / n as f32).cos())
            .collect();

        // Check middle portion (edges have boundary effects)
        for i in 10..n - 10 {
            assert_relative_eq!(hilbert[i], expected[i], epsilon = 0.1);
        }
    }
}
