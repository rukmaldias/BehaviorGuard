/// Risk scoring — compares a session's feature vector to the enrolled baseline.
///
/// Phase 1: statistical z-score distance (no ML model required).
/// Phase 2: per-user autoencoder reconstruction error (trained on-device at enrollment).
pub mod autoencoder;
pub mod scorer;

pub use autoencoder::Autoencoder;
pub use scorer::{RiskScore, Scorer};
