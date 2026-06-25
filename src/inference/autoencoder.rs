use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::error::{BgError, Result};
use crate::features::FEATURE_DIM;

// ── Architecture ──────────────────────────────────────────────────────────────

const L1: usize = 16; // encoder hidden / decoder hidden
const L2: usize = 8;  // bottleneck

const TRAIN_EPOCHS: usize = 300;
const LEARNING_RATE: f32 = 8e-4;

/// Data augmentation: noisy copies added per enrollment vector.
const AUGMENT_COUNT: usize = 80;
const AUGMENT_NOISE: f32 = 0.12; // std dev of Gaussian noise added to z-scores

/// MSE value (on z-normalised input) that maps to risk score 1.0.
/// Calibrated for z-normalised features: an impostor deviating by ~2σ per
/// feature maps to roughly MAX_RECONSTRUCTION_MSE.
pub const MAX_RECONSTRUCTION_MSE: f32 = 3.5;

/// Input clamping: prevents numerical blow-up from extreme z-scores
/// (e.g., when enrollment std is near its 1e-6 floor).
const Z_CLAMP: f32 = 8.0;

// ── Math helpers ──────────────────────────────────────────────────────────────

/// W @ x + b.  W: [out × in], row-major.
fn linear(w: &[f32], b: &[f32], x: &[f32], out: usize, inp: usize) -> Vec<f32> {
    let mut r = b.to_vec();
    for i in 0..out {
        for j in 0..inp {
            r[i] += w[i * inp + j] * x[j];
        }
    }
    r
}

/// W.T @ v.  W: [out × in], result: [in].
fn linear_t(w: &[f32], v: &[f32], out: usize, inp: usize) -> Vec<f32> {
    let mut r = vec![0.0f32; inp];
    for i in 0..out {
        for j in 0..inp {
            r[j] += w[i * inp + j] * v[i];
        }
    }
    r
}

fn relu(v: &[f32]) -> Vec<f32> { v.iter().map(|&x| x.max(0.0)).collect() }
fn relu_d(z: f32) -> f32 { if z > 0.0 { 1.0 } else { 0.0 } }

/// Accumulate outer product into dst: dst[i*cols+j] += a[i] * b[j].
fn acc_outer(dst: &mut [f32], a: &[f32], b: &[f32], cols: usize) {
    for i in 0..a.len() {
        for j in 0..cols {
            dst[i * cols + j] += a[i] * b[j];
        }
    }
}

fn acc(dst: &mut [f32], src: &[f32]) {
    for (d, &s) in dst.iter_mut().zip(src) { *d += s; }
}

fn apply_update(w: &mut [f32], grad: &[f32], lr: f32) {
    for (wi, &g) in w.iter_mut().zip(grad) { *wi -= lr * g; }
}

// ── Autoencoder ───────────────────────────────────────────────────────────────

/// Per-user autoencoder trained on z-normalised enrollment feature vectors.
///
/// Architecture: 32 → ReLU(16) → ReLU(8) → ReLU(16) → 32 (linear output).
/// Reconstruction MSE on z-normalised input is the Phase 2 anomaly score:
/// low error = user matches enrolled profile, high error = anomalous user.
///
/// Trained on-device during enrollment via batch gradient descent.
/// Weights are ~5 KB and serialise to JSON for persistent storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Autoencoder {
    // Encoder: 32→16→8
    w1: Vec<f32>, b1: Vec<f32>, // [L1 × FEATURE_DIM], [L1]
    w2: Vec<f32>, b2: Vec<f32>, // [L2 × L1], [L2]
    // Decoder: 8→16→32
    w3: Vec<f32>, b3: Vec<f32>, // [L1 × L2], [L1]
    w4: Vec<f32>, b4: Vec<f32>, // [FEATURE_DIM × L1], [FEATURE_DIM]
}

impl Autoencoder {
    fn new_xavier(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut layer = |inp: usize, out: usize| -> (Vec<f32>, Vec<f32>) {
            let std = (2.0 / (inp + out) as f32).sqrt();
            let w = (0..out * inp).map(|_| rng.gen::<f32>() * 2.0 * std - std).collect();
            (w, vec![0.0f32; out])
        };
        let (w1, b1) = layer(FEATURE_DIM, L1);
        let (w2, b2) = layer(L1, L2);
        let (w3, b3) = layer(L2, L1);
        let (w4, b4) = layer(L1, FEATURE_DIM);
        Self { w1, b1, w2, b2, w3, b3, w4, b4 }
    }

    /// Reconstruct a z-normalised input vector.
    pub fn reconstruct(&self, x: &[f32; FEATURE_DIM]) -> [f32; FEATURE_DIM] {
        let h1 = relu(&linear(&self.w1, &self.b1, x, L1, FEATURE_DIM));
        let h2 = relu(&linear(&self.w2, &self.b2, &h1, L2, L1));
        let h3 = relu(&linear(&self.w3, &self.b3, &h2, L1, L2));
        let out = linear(&self.w4, &self.b4, &h3, FEATURE_DIM, L1);
        out.try_into().unwrap()
    }

    /// Mean reconstruction MSE for a z-normalised feature vector.
    pub fn reconstruction_mse(&self, x: &[f32; FEATURE_DIM]) -> f32 {
        let out = self.reconstruct(x);
        out.iter().zip(x).map(|(&o, &t)| (o - t).powi(2)).sum::<f32>() / FEATURE_DIM as f32
    }

    /// Maps reconstruction MSE → risk score in [0.0, 1.0].
    pub fn risk_score(&self, x: &[f32; FEATURE_DIM]) -> f32 {
        (self.reconstruction_mse(x) / MAX_RECONSTRUCTION_MSE).min(1.0)
    }

    // ── Training ──────────────────────────────────────────────────────────────

    fn train_epoch(&mut self, samples: &[[f32; FEATURE_DIM]], lr: f32) -> f32 {
        let n = samples.len() as f32;

        // Gradient accumulators
        let mut dw1 = vec![0.0f32; L1 * FEATURE_DIM];
        let mut db1 = vec![0.0f32; L1];
        let mut dw2 = vec![0.0f32; L2 * L1];
        let mut db2 = vec![0.0f32; L2];
        let mut dw3 = vec![0.0f32; L1 * L2];
        let mut db3 = vec![0.0f32; L1];
        let mut dw4 = vec![0.0f32; FEATURE_DIM * L1];
        let mut db4 = vec![0.0f32; FEATURE_DIM];

        let mut total_loss = 0.0f32;

        for x in samples {
            // ── Forward ────────────────────────────────────────────────────
            let z1 = linear(&self.w1, &self.b1, x, L1, FEATURE_DIM);
            let h1 = relu(&z1);
            let z2 = linear(&self.w2, &self.b2, &h1, L2, L1);
            let h2 = relu(&z2);
            let z3 = linear(&self.w3, &self.b3, &h2, L1, L2);
            let h3 = relu(&z3);
            let out = linear(&self.w4, &self.b4, &h3, FEATURE_DIM, L1);

            let loss: f32 = out.iter().zip(x).map(|(&o, &t)| (o - t).powi(2)).sum::<f32>()
                / FEATURE_DIM as f32;
            total_loss += loss;

            // ── Backward ───────────────────────────────────────────────────
            // dL/d_out = 2*(out - x) / FEATURE_DIM
            let d_out: Vec<f32> = out.iter().zip(x)
                .map(|(&o, &t)| 2.0 * (o - t) / FEATURE_DIM as f32)
                .collect();

            // Layer 4 (dec2): FEATURE_DIM × L1, linear output
            acc_outer(&mut dw4, &d_out, &h3, L1);
            acc(&mut db4, &d_out);
            let dh3 = linear_t(&self.w4, &d_out, FEATURE_DIM, L1);

            // Layer 3 (dec1): L1 × L2, ReLU
            let dz3: Vec<f32> = dh3.iter().zip(&z3).map(|(&g, &z)| g * relu_d(z)).collect();
            acc_outer(&mut dw3, &dz3, &h2, L2);
            acc(&mut db3, &dz3);
            let dh2 = linear_t(&self.w3, &dz3, L1, L2);

            // Layer 2 (enc2): L2 × L1, ReLU
            let dz2: Vec<f32> = dh2.iter().zip(&z2).map(|(&g, &z)| g * relu_d(z)).collect();
            acc_outer(&mut dw2, &dz2, &h1, L1);
            acc(&mut db2, &dz2);
            let dh1 = linear_t(&self.w2, &dz2, L2, L1);

            // Layer 1 (enc1): L1 × FEATURE_DIM, ReLU
            let dz1: Vec<f32> = dh1.iter().zip(&z1).map(|(&g, &z)| g * relu_d(z)).collect();
            acc_outer(&mut dw1, &dz1, x, FEATURE_DIM);
            acc(&mut db1, &dz1);
        }

        // Apply averaged gradients (batch GD)
        let lr_n = lr / n;
        apply_update(&mut self.w1, &dw1, lr_n);
        apply_update(&mut self.b1, &db1, lr_n);
        apply_update(&mut self.w2, &dw2, lr_n);
        apply_update(&mut self.b2, &db2, lr_n);
        apply_update(&mut self.w3, &dw3, lr_n);
        apply_update(&mut self.b3, &db3, lr_n);
        apply_update(&mut self.w4, &dw4, lr_n);
        apply_update(&mut self.b4, &db4, lr_n);

        total_loss / n
    }

    // ── Public fit API ────────────────────────────────────────────────────────

    /// Trains a per-user autoencoder from z-normalised enrollment feature vectors.
    ///
    /// Augments each enrollment vector with `AUGMENT_COUNT` Gaussian-noisy copies
    /// to regularise the small dataset.  Training runs for `TRAIN_EPOCHS` epochs
    /// of batch gradient descent with cosine-annealed learning rate.
    pub fn fit(z_vectors: &[[f32; FEATURE_DIM]]) -> Result<Self> {
        if z_vectors.is_empty() {
            return Err(BgError::InsufficientEvents { got: 0, need: 1 });
        }

        let mut rng = StdRng::seed_from_u64(0xBE_00_00_00u64);

        // Build augmented training set
        let mut samples: Vec<[f32; FEATURE_DIM]> = Vec::with_capacity(
            z_vectors.len() * (1 + AUGMENT_COUNT),
        );
        for z in z_vectors {
            samples.push(*z);
            for _ in 0..AUGMENT_COUNT {
                let mut noisy = *z;
                for v in noisy.iter_mut() {
                    *v += (rng.gen::<f32>() * 2.0 - 1.0) * AUGMENT_NOISE * std::f32::consts::SQRT_2;
                }
                samples.push(noisy);
            }
        }

        // Fisher-Yates shuffle
        for i in (1..samples.len()).rev() {
            let j = rng.gen_range(0..=i);
            samples.swap(i, j);
        }

        let mut ae = Self::new_xavier(0xAE00u64);

        // Cosine-annealed learning rate
        for epoch in 0..TRAIN_EPOCHS {
            let progress = epoch as f32 / TRAIN_EPOCHS as f32;
            let lr = LEARNING_RATE * 0.5 * (1.0 + (std::f32::consts::PI * progress).cos());
            ae.train_epoch(&samples, lr);
        }

        Ok(ae)
    }

    // ── Serialisation ─────────────────────────────────────────────────────────

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| BgError::Serialise(e.to_string()))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|e| BgError::Serialise(e.to_string()))
    }
}

impl Drop for Autoencoder {
    fn drop(&mut self) {
        self.w1.zeroize(); self.b1.zeroize();
        self.w2.zeroize(); self.b2.zeroize();
        self.w3.zeroize(); self.b3.zeroize();
        self.w4.zeroize(); self.b4.zeroize();
    }
}

// ── z-normalise helper ────────────────────────────────────────────────────────

use crate::features::FeatureVector;
use crate::profile::enrollment::BaselineProfile;

/// Z-normalises a feature vector against a profile and clamps to ±Z_CLAMP
/// to prevent numerical blow-up when enrollment std is near its 1e-6 floor.
pub fn z_normalize(fv: &FeatureVector, profile: &BaselineProfile) -> [f32; FEATURE_DIM] {
    let mut z = [0.0f32; FEATURE_DIM];
    for i in 0..FEATURE_DIM {
        z[i] = ((fv.0[i] - profile.mean[i]) / profile.std[i]).clamp(-Z_CLAMP, Z_CLAMP);
    }
    z
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    fn uniform_fv(v: f32) -> [f32; FEATURE_DIM] {
        [v; FEATURE_DIM]
    }

    #[test]
    fn reconstruct_output_is_finite() {
        let ae = Autoencoder::new_xavier(1);
        let out = ae.reconstruct(&uniform_fv(0.5));
        for &v in &out {
            assert!(v.is_finite(), "output contains non-finite value");
        }
    }

    #[test]
    fn mse_is_non_negative() {
        let ae = Autoencoder::new_xavier(2);
        assert!(ae.reconstruction_mse(&uniform_fv(1.0)) >= 0.0);
    }

    #[test]
    fn risk_score_clamped_to_one() {
        let ae = Autoencoder::new_xavier(3);
        // Extreme input will produce large MSE, risk_score must still be ≤ 1
        assert!(ae.risk_score(&uniform_fv(1000.0)) <= 1.0);
    }

    #[test]
    fn fit_reduces_reconstruction_error_for_training_data() {
        // Enroll on 5 near-zero vectors (typical z-normalised enrollment)
        let enrollment: Vec<[f32; FEATURE_DIM]> = (0..5)
            .map(|i| {
                let mut v = uniform_fv(0.0);
                v[0] = i as f32 * 0.1; // small variation
                v
            })
            .collect();

        let ae = Autoencoder::fit(&enrollment).unwrap();

        // Reconstruction error on training-like input should be low
        let mse_on_train = ae.reconstruction_mse(&uniform_fv(0.0));
        let mse_far_input = ae.reconstruction_mse(&uniform_fv(5.0));

        assert!(
            mse_on_train < mse_far_input,
            "trained AE should reconstruct near-training input better than far input \
             (mse_train={mse_on_train:.4}, mse_far={mse_far_input:.4})"
        );
    }

    #[test]
    fn fit_errors_on_empty_input() {
        assert!(Autoencoder::fit(&[]).is_err());
    }

    #[test]
    fn serialise_round_trip_preserves_reconstruction() {
        let ae = Autoencoder::new_xavier(42);
        let input = uniform_fv(0.3);
        let original = ae.reconstruct(&input);

        let bytes = ae.to_bytes().unwrap();
        let restored = Autoencoder::from_bytes(&bytes).unwrap();
        let from_restored = restored.reconstruct(&input);

        for (a, b) in original.iter().zip(from_restored.iter()) {
            assert_abs_diff_eq!(a, b, epsilon = 1e-6);
        }
    }

    #[test]
    fn z_normalize_clamps_extreme_values() {
        use crate::profile::enrollment::BaselineProfile;
        let profile = BaselineProfile {
            mean: [5.0; FEATURE_DIM],
            std: [1e-6; FEATURE_DIM], // near-floor std
            session_count: 5,
        };
        let fv = FeatureVector([1000.0; FEATURE_DIM]);
        let z = z_normalize(&fv, &profile);
        for &v in &z {
            assert!(v <= Z_CLAMP, "z should be clamped to Z_CLAMP");
        }
    }
}
