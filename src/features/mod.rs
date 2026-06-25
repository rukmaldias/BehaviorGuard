/// Feature extraction — converts raw events into fixed-length numeric vectors.
///
/// Each session produces one `FeatureVector`. Vectors are what the ML model
/// sees; raw events are never stored beyond the active session.
pub mod extractor;
pub mod stats;

pub use extractor::{extract, FeatureVector};

/// Number of features in a `FeatureVector`. Must match the model input shape.
pub const FEATURE_DIM: usize = 32;
