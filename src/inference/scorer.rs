use crate::features::{FeatureVector, FEATURE_DIM};
use crate::inference::autoencoder::{self, Autoencoder, MAX_RECONSTRUCTION_MSE};
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

/// Phase 1 scorer: normalised mean absolute z-score.
///
/// Each feature's deviation is measured in units of its enrolled standard
/// deviation, averaged across all features, and clamped to [0, 1].
/// Used as a fallback when no autoencoder model is available.
pub struct Scorer;

impl Scorer {
    pub fn score(
        fv: &FeatureVector,
        profile: &BaselineProfile,
        events_used: usize,
    ) -> RiskScore {
        let mut total_z = 0.0f32;
        for i in 0..FEATURE_DIM {
            total_z += ((fv.0[i] - profile.mean[i]) / profile.std[i]).abs();
        }
        let mean_z = total_z / FEATURE_DIM as f32;
        let score = (mean_z / 3.0).min(1.0);
        let confidence = confidence(events_used, profile.session_count);
        RiskScore { score, confidence, events_used }
    }

    /// Phase 2 scorer: autoencoder reconstruction error on z-normalised input.
    ///
    /// Captures joint feature distributions (e.g. dwell-flight correlation)
    /// that the per-feature z-score misses.
    pub fn score_with_model(
        fv: &FeatureVector,
        profile: &BaselineProfile,
        model: &Autoencoder,
        events_used: usize,
    ) -> RiskScore {
        let z = autoencoder::z_normalize(fv, profile);
        let mse = model.reconstruction_mse(&z);
        let score = (mse / MAX_RECONSTRUCTION_MSE).min(1.0);
        let confidence = confidence(events_used, profile.session_count);
        RiskScore { score, confidence, events_used }
    }
}

fn confidence(events_used: usize, session_count: usize) -> f32 {
    (events_used as f32 / 50.0).min(1.0) * (session_count as f32 / 10.0).min(1.0)
}
