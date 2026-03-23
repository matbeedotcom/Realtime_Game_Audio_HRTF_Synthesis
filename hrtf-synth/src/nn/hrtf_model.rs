//! HRTF neural network model container

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use nalgebra::{DMatrix, DVector};
use thiserror::Error;

use super::feedforward::{DenseLayer, FeedforwardNetwork, InputNorm, OutputDenorm};
use crate::dsp::ItdDatabase;

/// Model loading errors
#[derive(Debug, Error)]
pub enum ModelError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid magic bytes - not an HRTF model file")]
    InvalidMagic,

    #[error("Unsupported model version: {0}")]
    UnsupportedVersion(u32),

    #[error("Invalid model dimensions")]
    InvalidDimensions,
}

/// HRTF neural network model
///
/// Contains all neural networks (one per direction × channel) and PCA data
/// needed for HRTF synthesis.
pub struct HrtfModel {
    /// Neural networks indexed as [direction][channel]
    /// channel 0 = left, channel 1 = right
    networks: Vec<Vec<FeedforwardNetwork>>,

    /// PCA coefficients [direction][channel] of [freq_bins × n_pca]
    pca_coeffs: Vec<Vec<DMatrix<f32>>>,

    /// PCA mean vectors [direction][channel] of [freq_bins]
    pca_means: Vec<Vec<DVector<f32>>>,

    /// Source positions [azimuth, elevation, distance]
    positions: Vec<[f32; 3]>,

    /// ITD reference database
    itd_ref: ItdDatabase,

    /// Number of frequency bins
    n_freq_bins: usize,

    /// Number of PCA components
    n_pca: usize,
}

impl HrtfModel {
    /// Load a model from a binary file
    pub fn load(path: &Path) -> Result<Self, ModelError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read and verify magic bytes
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"HRTF" {
            return Err(ModelError::InvalidMagic);
        }

        // Read version
        let version = read_u32(&mut reader)?;
        if version != 1 {
            return Err(ModelError::UnsupportedVersion(version));
        }

        // Read dimensions
        let n_dirs = read_u32(&mut reader)? as usize;
        let n_channels = read_u32(&mut reader)? as usize;
        let n_freq_bins = read_u32(&mut reader)? as usize;
        let n_pca = read_u32(&mut reader)? as usize;

        // Allocate storage
        let mut networks = Vec::with_capacity(n_dirs);
        let mut pca_coeffs = Vec::with_capacity(n_dirs);
        let mut pca_means = Vec::with_capacity(n_dirs);

        // Read networks and PCA data
        for _dir_idx in 0..n_dirs {
            let mut dir_networks = Vec::with_capacity(n_channels);
            let mut dir_coeffs = Vec::with_capacity(n_channels);
            let mut dir_means = Vec::with_capacity(n_channels);

            for _ch_idx in 0..n_channels {
                // Read network weights
                // W1: [20 × 8], b1: [20], W2: [12 × 20], b2: [12]
                let w1 = read_matrix(&mut reader, 20, 8)?;
                let b1 = read_vector(&mut reader, 20)?;
                let w2 = read_matrix(&mut reader, 12, 20)?;
                let b2 = read_vector(&mut reader, 12)?;

                // Read input normalization (optional)
                let has_input_norm = read_bool(&mut reader)?;
                let input_norm = if has_input_norm {
                    let mean = read_vector(&mut reader, 8)?;
                    let std = read_vector(&mut reader, 8)?;
                    Some(InputNorm { mean, std })
                } else {
                    None
                };

                // Read output denormalization
                let xmin = read_vector(&mut reader, n_pca)?;
                let xmax = read_vector(&mut reader, n_pca)?;
                let ymin = read_f32(&mut reader)?;
                let ymax = read_f32(&mut reader)?;

                let output_denorm = OutputDenorm {
                    xmin,
                    xmax,
                    ymin,
                    ymax,
                };

                // Create network
                let hidden = DenseLayer::new(w1, b1);
                let output = DenseLayer::new(w2, b2);
                let network = FeedforwardNetwork::new(hidden, output, input_norm, output_denorm);
                dir_networks.push(network);

                // Read PCA coefficients [freq_bins × n_pca]
                let coeffs = read_matrix(&mut reader, n_freq_bins, n_pca)?;
                dir_coeffs.push(coeffs);

                // Read PCA mean [freq_bins]
                let mean = read_vector(&mut reader, n_freq_bins)?;
                dir_means.push(mean);
            }

            networks.push(dir_networks);
            pca_coeffs.push(dir_coeffs);
            pca_means.push(dir_means);
        }

        // Read positions [n_dirs × 3]
        let mut positions = Vec::with_capacity(n_dirs);
        for _ in 0..n_dirs {
            let az = read_f32(&mut reader)?;
            let el = read_f32(&mut reader)?;
            let dist = read_f32(&mut reader)?;
            positions.push([az, el, dist]);
        }

        // Read ITD reference data
        let n_itd = read_u32(&mut reader)? as usize;

        let mut itd_values = Vec::with_capacity(n_itd);
        for _ in 0..n_itd {
            itd_values.push(read_f32(&mut reader)?);
        }

        let mut itd_positions = Vec::with_capacity(n_itd);
        for _ in 0..n_itd {
            let az = read_f32(&mut reader)?;
            let el = read_f32(&mut reader)?;
            let dist = read_f32(&mut reader)?;
            itd_positions.push([az, el, dist]);
        }

        let ref_width = read_f32(&mut reader)?;
        let ref_depth = read_f32(&mut reader)?;

        let itd_ref = ItdDatabase::new(itd_values, itd_positions, ref_width, ref_depth);

        Ok(Self {
            networks,
            pca_coeffs,
            pca_means,
            positions,
            itd_ref,
            n_freq_bins,
            n_pca,
        })
    }

    /// Get the number of spatial directions
    pub fn num_directions(&self) -> usize {
        self.positions.len()
    }

    /// Get the number of channels (2: left, right)
    pub fn num_channels(&self) -> usize {
        2
    }

    /// Get the number of frequency bins
    pub fn num_freq_bins(&self) -> usize {
        self.n_freq_bins
    }

    /// Get the number of PCA components
    pub fn num_pca(&self) -> usize {
        self.n_pca
    }

    /// Get position for a direction index
    pub fn get_position(&self, dir_idx: usize) -> (f32, f32, f32) {
        let pos = &self.positions[dir_idx];
        (pos[0], pos[1], pos[2])
    }

    /// Get all positions
    pub fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }

    /// Get the ITD reference database
    pub fn itd_ref(&self) -> &ItdDatabase {
        &self.itd_ref
    }

    /// Predict magnitude spectrum for a direction and channel
    ///
    /// Returns log-magnitude spectrum in dB
    pub fn predict_magnitude(
        &self,
        anthropometry: &DVector<f32>,
        dir_idx: usize,
        channel: usize,
    ) -> DVector<f32> {
        // Get PCA scores from neural network
        let scores = self.networks[dir_idx][channel].predict(anthropometry);

        // Reconstruct magnitude: DTF = coeffs * scores + mean
        let coeffs = &self.pca_coeffs[dir_idx][channel];
        let mean = &self.pca_means[dir_idx][channel];

        coeffs * scores + mean
    }
}

// Binary reading helpers

fn read_u32<R: Read>(reader: &mut R) -> std::io::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_f32<R: Read>(reader: &mut R) -> std::io::Result<f32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

fn read_bool<R: Read>(reader: &mut R) -> std::io::Result<bool> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0] != 0)
}

fn read_vector<R: Read>(reader: &mut R, len: usize) -> std::io::Result<DVector<f32>> {
    let mut data = vec![0.0f32; len];
    for i in 0..len {
        data[i] = read_f32(reader)?;
    }
    Ok(DVector::from_vec(data))
}

fn read_matrix<R: Read>(reader: &mut R, rows: usize, cols: usize) -> std::io::Result<DMatrix<f32>> {
    let mut data = vec![0.0f32; rows * cols];
    for i in 0..rows * cols {
        data[i] = read_f32(reader)?;
    }
    // DMatrix uses column-major order by default, but we're reading row-major
    // So we need to transpose or adjust
    Ok(DMatrix::from_row_slice(rows, cols, &data))
}
