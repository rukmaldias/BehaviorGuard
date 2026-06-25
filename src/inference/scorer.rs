use crate::features::{FeatureVector, FEATURE_DIM};
use crate::profile::enrollment::BaselineProfile;

/// The output of a scoring operation.
#[derive(Debug, Clone)]
pub struct RiskScore {
    /// Anomaly score in [0.0, 1.0]. 0.0 = matches baseline perfectly.
    /// 1.0 = maximally anomalous. Values above ~0.7 should trigger
    /// step-up authentication.
    pub score: f32,
    /// Confidence in [0.0, 1.0]. Higher when more signal types contributed
    /// and more events were present in the session.
    pub confidence: f32,
    /// Number of individual events processed in this session.
    pub events_used: usize,
}

impl RiskScore {
    pub fn is_anomalous(&self, threshold: f32) -> bool {
        self.score >= threshold
    }
}

/// Scores a `FeatureVector` against a `BaselineProfile`.
///
/// Current implementation: normalised mean absolute z-score.
/// Each feature's deviation is measured in units of its enrolled standard
/// deviation, then averaged and clamped to [0, 1].
///
/// This is replaced by TFLite autoencoder reconstruction error in Phase 2.
pub struct Scorer;

impl Scorer {
    pub fn score(
        fv: &FeatureVector,
        profile: &BaselineProfile,
        events_used: usize,
    ) -> RiskScore {
        let mut total_z = 0.0f32;
        let mut active = 0usize;

        for i in 0..FEATURE_DIM {
            let z = ((fv.0[i] - profile.mean[i]) / profile.std[i]).abs();
            total_z += z;
            active += 1;
        }

        let mean_z = if active > 0 { total_z / active as f32 } else { 0.0 };

        // Map mean z-score to [0, 1]:
        // z=0 → score=0.0, z=3 → score≈1.0 (sigmoid-like clamp)
        let score = (mean_z / 3.0).min(1.0);

        // Confidence grows with session richness
        let confidence = (events_used as f32 / 50.0).min(1.0)
            * (profile.session_count as f32 / 10.0).min(1.0);

        RiskScore { score, confidence, events_used }
    }
}
