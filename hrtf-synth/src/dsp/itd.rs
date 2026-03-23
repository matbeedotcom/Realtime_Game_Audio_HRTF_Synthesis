//! Interaural Time Difference (ITD) calculation

use serde::{Deserialize, Serialize};

/// Speed of sound in cm/s (at ~20°C)
const SPEED_OF_SOUND: f64 = 343.0 * 100.0; // 34300 cm/s

/// ITD reference database for adaptive ITD calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItdDatabase {
    /// Reference ITD values in seconds
    pub itd: Vec<f32>,
    /// Source positions [azimuth, elevation, distance]
    pub positions: Vec<[f32; 3]>,
    /// Reference head width in cm
    pub ref_width: f32,
    /// Reference head depth in cm
    pub ref_depth: f32,
}

impl ItdDatabase {
    /// Create a new ITD database
    pub fn new(
        itd: Vec<f32>,
        positions: Vec<[f32; 3]>,
        ref_width: f32,
        ref_depth: f32,
    ) -> Self {
        Self {
            itd,
            positions,
            ref_width,
            ref_depth,
        }
    }

    /// Find the nearest position index
    pub fn find_nearest(&self, azimuth: f32, elevation: f32) -> usize {
        let mut min_dist = f32::MAX;
        let mut min_idx = 0;

        for (i, pos) in self.positions.iter().enumerate() {
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
}

/// Calculate effective head radius from width and depth using Algazi formula.
///
/// r = 0.51 * width + 0.18 * depth + 3.2 (in cm)
fn head_radius(width_cm: f64, depth_cm: f64) -> f64 {
    0.51 * width_cm + 0.18 * depth_cm + 3.2
}

/// Calculate ITD using spherical head model (Woodworth & Schlosberg, optimized by Savioja).
///
/// ITD = (3 * r / (2 * c)) * sin(azimuth) * cos(elevation)
///
/// # Arguments
/// * `azimuth_deg` - Source azimuth in degrees (SOFA convention: 0-360)
/// * `elevation_deg` - Source elevation in degrees (-90 to 90)
/// * `head_width_cm` - Head width (distance between ears) in cm
/// * `head_depth_cm` - Head depth (front to back) in cm
///
/// # Returns
/// ITD in seconds (positive = right ear leading)
pub fn calculate_itd_spherical(
    azimuth_deg: f64,
    elevation_deg: f64,
    head_width_cm: f64,
    head_depth_cm: f64,
) -> f64 {
    let r = head_radius(head_width_cm, head_depth_cm);

    let azimuth_rad = azimuth_deg.to_radians();
    let elevation_rad = elevation_deg.to_radians();

    (3.0 * r / (2.0 * SPEED_OF_SOUND)) * azimuth_rad.sin() * elevation_rad.cos()
}

/// Calculate ITD using adaptive method based on reference measurements.
///
/// This adapts a measured ITD from a reference head to the target head
/// by scaling based on the ratio of head radii.
///
/// # Arguments
/// * `azimuth_deg` - Source azimuth in degrees (SOFA convention: 0-360)
/// * `elevation_deg` - Source elevation in degrees (-90 to 90)
/// * `head_width_cm` - Target head width in cm
/// * `head_depth_cm` - Target head depth in cm
/// * `ref_db` - Reference ITD database
///
/// # Returns
/// ITD in seconds (positive = right ear leading)
pub fn calculate_itd_adaptive(
    azimuth_deg: f64,
    elevation_deg: f64,
    head_width_cm: f64,
    head_depth_cm: f64,
    ref_db: &ItdDatabase,
) -> f64 {
    // Find nearest position in reference database
    let idx = ref_db.find_nearest(azimuth_deg as f32, elevation_deg as f32);

    // Get reference ITD
    let itd_ref = ref_db.itd[idx] as f64;

    // Calculate head radii
    let target_radius = head_radius(head_width_cm, head_depth_cm);
    let ref_radius = head_radius(ref_db.ref_width as f64, ref_db.ref_depth as f64);

    // Scale ITD by ratio of radii
    itd_ref * (target_radius / ref_radius)
}

/// Convert ITD in seconds to samples at the given sample rate
pub fn itd_to_samples(itd_seconds: f64, sample_rate: u32) -> f64 {
    itd_seconds.abs() * sample_rate as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_head_radius() {
        // Test with typical head dimensions
        let r = head_radius(15.0, 20.0);
        assert!(r > 10.0 && r < 15.0); // Reasonable head radius
    }

    #[test]
    fn test_itd_spherical_frontal() {
        // ITD should be 0 for frontal sources
        let itd = calculate_itd_spherical(0.0, 0.0, 15.0, 20.0);
        assert_relative_eq!(itd, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_itd_spherical_lateral() {
        // ITD should be maximum for lateral sources (90 degrees)
        let itd_90 = calculate_itd_spherical(90.0, 0.0, 15.0, 20.0);
        let itd_45 = calculate_itd_spherical(45.0, 0.0, 15.0, 20.0);

        assert!(itd_90 > itd_45);
        assert!(itd_90 > 0.0);

        // Typical max ITD is around 600-700 microseconds
        assert!(itd_90.abs() < 0.001); // Less than 1ms
        assert!(itd_90.abs() > 0.0003); // More than 300us
    }

    #[test]
    fn test_itd_to_samples() {
        let samples = itd_to_samples(0.0005, 44100); // 500us at 44.1kHz
        assert_relative_eq!(samples, 22.05, epsilon = 0.01);
    }
}
