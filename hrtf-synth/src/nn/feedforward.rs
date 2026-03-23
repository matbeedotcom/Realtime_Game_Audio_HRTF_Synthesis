//! Feedforward neural network implementation

use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};

/// A single dense (fully connected) layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenseLayer {
    /// Weight matrix [out_dim × in_dim]
    pub weights: DMatrix<f32>,
    /// Bias vector [out_dim]
    pub bias: DVector<f32>,
}

impl DenseLayer {
    /// Create a new dense layer
    pub fn new(weights: DMatrix<f32>, bias: DVector<f32>) -> Self {
        assert_eq!(weights.nrows(), bias.len());
        Self { weights, bias }
    }

    /// Forward pass with tanh activation
    pub fn forward_tanh(&self, input: &DVector<f32>) -> DVector<f32> {
        let z = &self.weights * input + &self.bias;
        z.map(|x| x.tanh())
    }

    /// Forward pass with linear activation (no activation)
    pub fn forward_linear(&self, input: &DVector<f32>) -> DVector<f32> {
        &self.weights * input + &self.bias
    }

    /// Input dimension
    pub fn input_dim(&self) -> usize {
        self.weights.ncols()
    }

    /// Output dimension
    pub fn output_dim(&self) -> usize {
        self.weights.nrows()
    }
}

/// Input normalization parameters (mapstd in MATLAB)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputNorm {
    /// Mean for each input feature
    pub mean: DVector<f32>,
    /// Standard deviation for each input feature
    pub std: DVector<f32>,
}

impl InputNorm {
    /// Normalize input: (x - mean) / std
    pub fn normalize(&self, input: &DVector<f32>) -> DVector<f32> {
        let mut result = input - &self.mean;
        for i in 0..result.len() {
            if self.std[i].abs() > 1e-10 {
                result[i] /= self.std[i];
            }
        }
        result
    }
}

/// Output denormalization parameters (mapminmax reverse in MATLAB)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDenorm {
    /// Original minimum values
    pub xmin: DVector<f32>,
    /// Original maximum values
    pub xmax: DVector<f32>,
    /// Target range minimum (usually 0)
    pub ymin: f32,
    /// Target range maximum (usually 1)
    pub ymax: f32,
}

impl OutputDenorm {
    /// Denormalize output from [ymin, ymax] back to [xmin, xmax]
    /// Formula: x = (y - ymin) * (xmax - xmin) / (ymax - ymin) + xmin
    pub fn denormalize(&self, output: &DVector<f32>) -> DVector<f32> {
        let y_range = self.ymax - self.ymin;
        let mut result = DVector::zeros(output.len());

        for i in 0..output.len() {
            let x_range = self.xmax[i] - self.xmin[i];
            if y_range.abs() > 1e-10 {
                result[i] = (output[i] - self.ymin) * x_range / y_range + self.xmin[i];
            } else {
                result[i] = self.xmin[i];
            }
        }

        result
    }
}

/// Feedforward neural network (8 -> 20 -> 12 architecture)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedforwardNetwork {
    /// Hidden layer: 8 inputs -> 20 neurons with tanh
    pub hidden: DenseLayer,
    /// Output layer: 20 inputs -> 12 outputs with linear activation
    pub output: DenseLayer,
    /// Input normalization parameters
    pub input_norm: Option<InputNorm>,
    /// Output denormalization parameters
    pub output_denorm: OutputDenorm,
}

impl FeedforwardNetwork {
    /// Create a new feedforward network
    pub fn new(
        hidden: DenseLayer,
        output: DenseLayer,
        input_norm: Option<InputNorm>,
        output_denorm: OutputDenorm,
    ) -> Self {
        Self {
            hidden,
            output,
            input_norm,
            output_denorm,
        }
    }

    /// Forward pass: anthropometry -> PCA scores
    ///
    /// 1. Normalize input (if input_norm is set)
    /// 2. Hidden layer with tanh
    /// 3. Output layer with linear
    /// 4. Denormalize output
    pub fn predict(&self, anthropometry: &DVector<f32>) -> DVector<f32> {
        // Normalize input
        let normalized = if let Some(ref norm) = self.input_norm {
            norm.normalize(anthropometry)
        } else {
            anthropometry.clone()
        };

        // Hidden layer with tanh
        let hidden_out = self.hidden.forward_tanh(&normalized);

        // Output layer (linear)
        let output = self.output.forward_linear(&hidden_out);

        // Denormalize output
        self.output_denorm.denormalize(&output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_dense_layer_dimensions() {
        let weights = DMatrix::from_row_slice(3, 2, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let bias = DVector::from_row_slice(&[0.1, 0.2, 0.3]);

        let layer = DenseLayer::new(weights, bias);
        assert_eq!(layer.input_dim(), 2);
        assert_eq!(layer.output_dim(), 3);
    }

    #[test]
    fn test_input_normalization() {
        let norm = InputNorm {
            mean: DVector::from_row_slice(&[10.0, 20.0]),
            std: DVector::from_row_slice(&[2.0, 5.0]),
        };

        let input = DVector::from_row_slice(&[12.0, 25.0]);
        let normalized = norm.normalize(&input);

        assert_relative_eq!(normalized[0], 1.0, epsilon = 1e-6);
        assert_relative_eq!(normalized[1], 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_output_denormalization() {
        let denorm = OutputDenorm {
            xmin: DVector::from_row_slice(&[-10.0, 0.0]),
            xmax: DVector::from_row_slice(&[10.0, 100.0]),
            ymin: 0.0,
            ymax: 1.0,
        };

        let output = DVector::from_row_slice(&[0.5, 0.25]);
        let denormalized = denorm.denormalize(&output);

        assert_relative_eq!(denormalized[0], 0.0, epsilon = 1e-6);
        assert_relative_eq!(denormalized[1], 25.0, epsilon = 1e-6);
    }
}
