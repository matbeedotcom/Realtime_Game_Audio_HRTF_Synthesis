//! Neural network inference modules

pub mod feedforward;
pub mod hrtf_model;

pub use feedforward::{DenseLayer, FeedforwardNetwork};
pub use hrtf_model::HrtfModel;
