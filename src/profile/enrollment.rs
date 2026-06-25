use crate::features::{FeatureVector, FEATURE_DIM};
use serde::{Deserialize, Serialize};

/// Minimum number of sessions required before scoring is available.
pub const SESSIONS_REQUIRED: usize = 5;

/// The baseline profile — mean and std of each feature across enrolled sessions.
///
/// Used by the scorer to compute z-score distance from the enrolled baseline.
/// Produced once enrollment is complete; never updated after that without
/// explicit re-enrollment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineProfile {
    /// Per-feature mean across all enrolled sessions.
    pub mean: [f32; FEATURE_DIM],
    /// Per-feature standard deviation across all enrolled sessions.
    pub std: [f32; FEATURE_DIM],
    /// Number of sessions used to build this baseline.
    pub session_count: usize,
}

impl BaselineProfile {
    /// Builds a baseline from a set of feature vectors.
    /// Panics if `vectors` is empty.
    pub fn from_vectors(vectors: &[FeatureVector]) -> Self {
        assert!(!vectors.is_empty());
        let n = vectors.len() as f32;
        let mut mean = [0.0f32; FEATURE_DIM];
        let mut std = [0.0f32; FEATURE_DIM];

        for v in vectors {
            for (i, &x) in v.0.iter().enumerate() {
                mean[i] += x;
            }
        }
        for m in mean.iter_mut() {
            *m /= n;
        }
        for v in vectors {
            for (i, &x) in v.0.iter().enumerate() {
                std[i] += (x - mean[i]).powi(2);
            }
        }
        for s in std.iter_mut() {
            *s = (*s / n).sqrt().max(1e-6); // floor to avoid division by zero
        }

        Self { mean, std, session_count: vectors.len() }
    }
}

/// Tracks the enrollment process before a profile is ready.
#[derive(Debug, Default)]
pub struct EnrollmentState {
    pub collected: Vec<FeatureVector>,
}

impl EnrollmentState {
    pub fn add(&mut self, fv: FeatureVector) {
        self.collected.push(fv);
    }

    pub fn is_complete(&self) -> bool {
        self.collected.len() >= SESSIONS_REQUIRED
    }

    pub fn sessions_remaining(&self) -> usize {
        SESSIONS_REQUIRED.saturating_sub(self.collected.len())
    }

    pub fn build_profile(&self) -> Option<BaselineProfile> {
        if self.is_complete() {
            Some(BaselineProfile::from_vectors(&self.collected))
        } else {
            None
        }
    }
}
