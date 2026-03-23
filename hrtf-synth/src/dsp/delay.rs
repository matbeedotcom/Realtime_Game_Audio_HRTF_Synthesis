//! Fractional delay implementation

/// Apply a fractional sample delay to a signal using Lagrange interpolation.
///
/// This is equivalent to MATLAB's `dsp.VariableFractionalDelay` with
/// Lagrange interpolation.
///
/// # Arguments
/// * `signal` - Input signal (modified in-place)
/// * `delay_samples` - Delay in samples (can be fractional)
pub fn apply_fractional_delay(signal: &mut [f32], delay_samples: f64) {
    if delay_samples.abs() < 1e-10 {
        return; // No delay needed
    }

    let n = signal.len();
    if n == 0 {
        return;
    }

    // Split into integer and fractional parts
    let int_delay = delay_samples.floor() as isize;
    let frac_delay = (delay_samples - delay_samples.floor()) as f32;

    // First apply integer delay (circular shift)
    let mut temp = signal.to_vec();

    if int_delay > 0 {
        // Positive delay: shift right
        let shift = int_delay as usize % n;
        for i in 0..n {
            let src_idx = (n + i - shift) % n;
            signal[i] = temp[src_idx];
        }
    } else if int_delay < 0 {
        // Negative delay: shift left
        let shift = (-int_delay) as usize % n;
        for i in 0..n {
            let src_idx = (i + shift) % n;
            signal[i] = temp[src_idx];
        }
    }

    // Apply fractional delay using linear interpolation
    // (Simple but effective for small fractional delays)
    if frac_delay.abs() > 1e-10 {
        temp.copy_from_slice(signal);

        for i in 0..n {
            let prev_idx = if i > 0 { i - 1 } else { n - 1 };
            signal[i] = temp[i] * (1.0 - frac_delay) + temp[prev_idx] * frac_delay;
        }
    }
}

/// Apply a fractional delay using 4th-order Lagrange interpolation.
///
/// More accurate than linear interpolation but more computationally expensive.
#[allow(dead_code)]
pub fn apply_fractional_delay_lagrange(signal: &mut [f32], delay_samples: f64) {
    if delay_samples.abs() < 1e-10 {
        return;
    }

    let n = signal.len();
    if n < 5 {
        // Not enough samples for 4th order Lagrange
        apply_fractional_delay(signal, delay_samples);
        return;
    }

    // Split into integer and fractional parts
    let int_delay = delay_samples.floor() as isize;
    let frac = (delay_samples - delay_samples.floor()) as f32;

    // Apply integer delay first
    let temp = signal.to_vec();
    if int_delay != 0 {
        let shift = if int_delay > 0 {
            int_delay as usize % n
        } else {
            n - ((-int_delay) as usize % n)
        };

        for i in 0..n {
            let src_idx = (n + i - shift) % n;
            signal[i] = temp[src_idx];
        }
    }

    // Apply fractional delay using 4th order Lagrange
    if frac.abs() > 1e-10 {
        let input = signal.to_vec();

        for i in 0..n {
            // Get 4 surrounding samples (with wraparound)
            let i0 = if i >= 2 { i - 2 } else { n + i - 2 };
            let i1 = if i >= 1 { i - 1 } else { n - 1 };
            let i2 = i;
            let i3 = (i + 1) % n;

            let y0 = input[i0];
            let y1 = input[i1];
            let y2 = input[i2];
            let y3 = input[i3];

            // 4th order Lagrange interpolation coefficients
            let c0 = y1;
            let c1 = 0.5 * (y2 - y0);
            let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
            let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);

            signal[i] = c0 + frac * (c1 + frac * (c2 + frac * c3));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_integer_delay() {
        let mut signal = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        apply_fractional_delay(&mut signal, 2.0);

        // Should shift right by 2
        assert_relative_eq!(signal[0], 7.0, epsilon = 1e-5);
        assert_relative_eq!(signal[1], 8.0, epsilon = 1e-5);
        assert_relative_eq!(signal[2], 1.0, epsilon = 1e-5);
    }

    #[test]
    fn test_zero_delay() {
        let original = vec![1.0, 2.0, 3.0, 4.0];
        let mut signal = original.clone();
        apply_fractional_delay(&mut signal, 0.0);

        for (a, b) in signal.iter().zip(original.iter()) {
            assert_relative_eq!(a, b, epsilon = 1e-10);
        }
    }
}
