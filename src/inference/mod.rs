/// Risk scoring — compares a session's feature vector to the enrolled baseline.
///
/// Phase 1: statistical z-score distance (no ML model required).
/// Phase 2 (future): TFLite autoencoder reconstruction error replaces z-score.
pub mod scorer;

pub use scorer::{RiskScore, Scorer};
