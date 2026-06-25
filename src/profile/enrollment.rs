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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::FEATURE_DIM;
    use approx::assert_abs_diff_eq;

    fn make_fv(value: f32) -> FeatureVector {
        FeatureVector([value; FEATURE_DIM])
    }

    #[test]
    fn enrollment_starts_incomplete() {
        let e = EnrollmentState::default();
        assert!(!e.is_complete());
        assert_eq!(e.sessions_remaining(), SESSIONS_REQUIRED);
        assert!(e.build_profile().is_none());
    }

    #[test]
    fn enrollment_completes_at_required_count() {
        let mut e = EnrollmentState::default();
        for i in 0..SESSIONS_REQUIRED {
            assert!(!e.is_complete());
            assert_eq!(e.sessions_remaining(), SESSIONS_REQUIRED - i);
            e.add(make_fv(i as f32));
        }
        assert!(e.is_complete());
        assert_eq!(e.sessions_remaining(), 0);
        assert!(e.build_profile().is_some());
    }

    #[test]
    fn baseline_mean_correct_for_uniform_input() {
        let mut e = EnrollmentState::default();
        for _ in 0..SESSIONS_REQUIRED {
            e.add(make_fv(4.0));
        }
        let profile = e.build_profile().unwrap();
        for i in 0..FEATURE_DIM {
            assert_abs_diff_eq!(profile.mean[i], 4.0, epsilon = 1e-5);
        }
    }

    #[test]
    fn baseline_std_zero_for_identical_sessions() {
        let mut e = EnrollmentState::default();
        for _ in 0..SESSIONS_REQUIRED {
            e.add(make_fv(7.0));
        }
        let profile = e.build_profile().unwrap();
        // std is floored at 1e-6 to avoid division by zero in scorer
        for i in 0..FEATURE_DIM {
            assert!(profile.std[i] <= 1e-4, "std should be near zero: {}", profile.std[i]);
        }
    }

    #[test]
    fn baseline_mean_correct_for_two_values() {
        let mut e = EnrollmentState::default();
        for i in 0..SESSIONS_REQUIRED {
            // alternate 0.0 and 10.0 → mean = 5.0 or 4.0 depending on count
            e.add(make_fv(if i % 2 == 0 { 0.0 } else { 10.0 }));
        }
        let profile = e.build_profile().unwrap();
        // With 5 sessions: 0,10,0,10,0 → mean = 4.0
        for i in 0..FEATURE_DIM {
            assert_abs_diff_eq!(profile.mean[i], 4.0, epsilon = 1e-4);
        }
    }

    #[test]
    fn session_count_stored_in_profile() {
        let mut e = EnrollmentState::default();
        for _ in 0..SESSIONS_REQUIRED {
            e.add(make_fv(1.0));
        }
        let profile = e.build_profile().unwrap();
        assert_eq!(profile.session_count, SESSIONS_REQUIRED);
    }
}
