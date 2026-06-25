"""
BehaviorGuard — autoencoder validation script.

Mirrors the Rust autoencoder logic in Python/NumPy to let you:
  1. Tune hyperparameters (hidden dims, epochs, augmentation noise) without
     recompiling Rust.
  2. Inspect per-epoch loss curves.
  3. Confirm that enrolled vs impostor reconstruction MSE is well-separated.
  4. Calibrate MAX_RECONSTRUCTION_MSE for the desired EER threshold.

Usage:
  pip install -r requirements.txt
  python scripts/validate_autoencoder.py

The script does NOT produce a model file — the Rust side trains on-device.
"""

import numpy as np
import matplotlib
matplotlib.use("Agg")   # headless
import matplotlib.pyplot as plt

# ── Architecture (must match src/inference/autoencoder.rs) ───────────────────

FEATURE_DIM = 32
L1 = 16     # encoder/decoder hidden
L2 = 8      # bottleneck
TRAIN_EPOCHS = 300
LR = 8e-4
AUGMENT_COUNT = 80
AUGMENT_NOISE = 0.12
MAX_RECONSTRUCTION_MSE = 3.5
Z_CLAMP = 8.0

# ── NumPy autoencoder (mirrors Rust) ─────────────────────────────────────────

def xavier(inp, out, rng):
    std = np.sqrt(2.0 / (inp + out))
    return rng.uniform(-std, std, (out, inp)).astype(np.float32)

def relu(x):
    return np.maximum(x, 0)

def relu_grad(z):
    return (z > 0).astype(np.float32)

class Autoencoder:
    def __init__(self, seed=0xAE00):
        rng = np.random.RandomState(seed)
        self.W1 = xavier(FEATURE_DIM, L1, rng);  self.b1 = np.zeros(L1, np.float32)
        self.W2 = xavier(L1, L2, rng);            self.b2 = np.zeros(L2, np.float32)
        self.W3 = xavier(L2, L1, rng);            self.b3 = np.zeros(L1, np.float32)
        self.W4 = xavier(L1, FEATURE_DIM, rng);  self.b4 = np.zeros(FEATURE_DIM, np.float32)

    def forward(self, X):
        """X: [N, 32] → returns (z1,h1,z2,h2,z3,h3,out)."""
        z1 = X @ self.W1.T + self.b1;  h1 = relu(z1)
        z2 = h1 @ self.W2.T + self.b2; h2 = relu(z2)
        z3 = h2 @ self.W3.T + self.b3; h3 = relu(z3)
        out = h3 @ self.W4.T + self.b4
        return z1, h1, z2, h2, z3, h3, out

    def train_epoch(self, X, lr):
        N = len(X)
        z1, h1, z2, h2, z3, h3, out = self.forward(X)

        loss = np.mean((out - X) ** 2)

        d_out = 2 * (out - X) / FEATURE_DIM        # [N, 32]
        dW4 = d_out.T @ h3 / N;  db4 = d_out.mean(0)
        dh3 = d_out @ self.W4
        dz3 = dh3 * relu_grad(z3)
        dW3 = dz3.T @ h2 / N;    db3 = dz3.mean(0)
        dh2 = dz3 @ self.W3
        dz2 = dh2 * relu_grad(z2)
        dW2 = dz2.T @ h1 / N;    db2 = dz2.mean(0)
        dh1 = dz2 @ self.W2
        dz1 = dh1 * relu_grad(z1)
        dW1 = dz1.T @ X / N;     db1 = dz1.mean(0)

        self.W4 -= lr * dW4; self.b4 -= lr * db4
        self.W3 -= lr * dW3; self.b3 -= lr * db3
        self.W2 -= lr * dW2; self.b2 -= lr * db2
        self.W1 -= lr * dW1; self.b1 -= lr * db1
        return loss

    def mse(self, X):
        *_, out = self.forward(X)
        return np.mean((out - X) ** 2, axis=1)

    def risk_score(self, X):
        return np.clip(self.mse(X) / MAX_RECONSTRUCTION_MSE, 0, 1)

# ── Synthetic data ────────────────────────────────────────────────────────────

def make_user(rng, n_sessions=5):
    """Generate N z-normalised feature vectors for one synthetic user."""
    # Each user has a fixed typing "signature" (mean in raw-feature space)
    # The z-normalisation makes their own sessions cluster near 0.
    raw_mean = rng.uniform(0.3, 0.8, FEATURE_DIM).astype(np.float32)
    raw_std  = rng.uniform(0.05, 0.15, FEATURE_DIM).astype(np.float32)
    sessions = rng.normal(raw_mean, raw_std * 0.3, (n_sessions, FEATURE_DIM)).astype(np.float32)

    # Compute profile mean/std (mirrors Rust's BaselineProfile::from_vectors)
    profile_mean = sessions.mean(0)
    profile_std  = sessions.std(0).clip(1e-6)

    z_sessions = (sessions - profile_mean) / profile_std
    return z_sessions, profile_mean, profile_std

def make_impostor_z(profile_mean, profile_std, rng, n=20):
    """Generate z-scores for an impostor (random typing, not the enrolled user)."""
    raw_mean = rng.uniform(0.1, 0.9, FEATURE_DIM).astype(np.float32)
    raw_std  = rng.uniform(0.05, 0.2, FEATURE_DIM).astype(np.float32)
    sessions = rng.normal(raw_mean, raw_std * 0.3, (n, FEATURE_DIM)).astype(np.float32)
    z = (sessions - profile_mean) / profile_std
    return np.clip(z, -Z_CLAMP, Z_CLAMP)

def augment(z_sessions, rng):
    """Mirror Rust's augmentation: AUGMENT_COUNT noisy copies per session."""
    copies = [z_sessions]
    for _ in range(AUGMENT_COUNT):
        noise = rng.normal(0, AUGMENT_NOISE, z_sessions.shape).astype(np.float32)
        copies.append(z_sessions + noise)
    aug = np.vstack(copies)
    rng.shuffle(aug)
    return aug

# ── Training ──────────────────────────────────────────────────────────────────

def cosine_lr(epoch, max_lr=LR, total=TRAIN_EPOCHS):
    return max_lr * 0.5 * (1 + np.cos(np.pi * epoch / total))

def train(z_sessions, rng, verbose=True):
    samples = augment(z_sessions, rng)
    ae = Autoencoder()
    losses = []
    for epoch in range(TRAIN_EPOCHS):
        lr = cosine_lr(epoch)
        loss = ae.train_epoch(samples, lr)
        losses.append(loss)
    if verbose:
        print(f"  Final loss: {losses[-1]:.6f}  |  epoch 1: {losses[0]:.6f}")
    return ae, losses

# ── Evaluation ────────────────────────────────────────────────────────────────

def evaluate(n_users=50, seed=42):
    rng = np.random.RandomState(seed)
    enrolled_mses, impostor_mses = [], []

    for _ in range(n_users):
        z_sessions, p_mean, p_std = make_user(rng)
        ae, _ = train(z_sessions, rng, verbose=False)

        # Enrolled: one held-out session from the same user
        z_test = make_user(rng, n_sessions=1)[0]  # different rng state → slight drift
        enrolled_mses.append(ae.mse(z_test).mean())

        # Impostor: random different user
        z_imp = make_impostor_z(p_mean, p_std, rng)
        impostor_mses.append(ae.mse(z_imp).mean())

    enrolled_mses = np.array(enrolled_mses)
    impostor_mses = np.array(impostor_mses)

    print("\n── Reconstruction MSE statistics ────────────────────────────────")
    print(f"  Enrolled  — mean: {enrolled_mses.mean():.4f}  std: {enrolled_mses.std():.4f}  "
          f"max: {enrolled_mses.max():.4f}")
    print(f"  Impostor  — mean: {impostor_mses.mean():.4f}  std: {impostor_mses.std():.4f}  "
          f"min: {impostor_mses.min():.4f}")
    print(f"  Separation: {(impostor_mses.mean() - enrolled_mses.mean()):.4f}")
    print(f"  Current MAX_RECONSTRUCTION_MSE = {MAX_RECONSTRUCTION_MSE}")

    # EER calculation
    thresholds = np.linspace(0, MAX_RECONSTRUCTION_MSE * 2, 1000)
    far_list, frr_list = [], []
    for t in thresholds:
        far = np.mean(impostor_mses < t)   # impostors below threshold = false accept
        frr = np.mean(enrolled_mses >= t)  # enrolled above threshold = false reject
        far_list.append(far)
        frr_list.append(frr)
    far = np.array(far_list); frr = np.array(frr_list)
    eer_idx = np.argmin(np.abs(far - frr))
    print(f"\n  EER: {(far[eer_idx] + frr[eer_idx]) / 2 * 100:.1f}%  "
          f"at threshold MSE = {thresholds[eer_idx]:.4f}")

    # Plot
    _, axes = plt.subplots(1, 2, figsize=(12, 4))
    axes[0].hist(enrolled_mses, bins=15, alpha=0.6, label="Enrolled", color="steelblue")
    axes[0].hist(impostor_mses, bins=15, alpha=0.6, label="Impostor", color="tomato")
    axes[0].axvline(MAX_RECONSTRUCTION_MSE, color="k", linestyle="--",
                    label=f"MAX_MSE={MAX_RECONSTRUCTION_MSE}")
    axes[0].set_xlabel("Reconstruction MSE"); axes[0].set_ylabel("Count")
    axes[0].set_title("MSE distribution (enrolled vs impostor)")
    axes[0].legend()
    axes[1].plot(far, frr, lw=2)
    axes[1].plot([0, 1], [0, 1], "k--", alpha=0.3)
    axes[1].set_xlabel("FAR"); axes[1].set_ylabel("FRR")
    axes[1].set_title(f"FAR vs FRR  (EER ≈ {(far[eer_idx]+frr[eer_idx])/2*100:.1f}%)")
    axes[1].set_xlim(0, 1); axes[1].set_ylim(0, 1)
    plt.tight_layout()
    plt.savefig("scripts/autoencoder_validation.png", dpi=120)
    print("\n  Plot saved to scripts/autoencoder_validation.png")

# ── Entry point ───────────────────────────────────────────────────────────────

if __name__ == "__main__":
    print("BehaviorGuard autoencoder validation")
    print("=" * 50)

    # Quick single-user demo
    print("\n[Single user training demo]")
    rng = np.random.RandomState(0)
    z_sessions, _, _ = make_user(rng)
    ae, losses = train(z_sessions, rng)

    enrolled_test = make_user(rng, n_sessions=5)[0]
    rng2 = np.random.RandomState(999)
    impostor_test = make_impostor_z(*make_user(rng2)[1:], rng2)[:5]

    print(f"  Enrolled test MSE  (should be low):  {ae.mse(enrolled_test).mean():.4f}")
    print(f"  Impostor test MSE  (should be high): {ae.mse(impostor_test).mean():.4f}")
    print(f"  Enrolled risk score: {ae.risk_score(enrolled_test).mean():.3f}")
    print(f"  Impostor risk score: {ae.risk_score(impostor_test).mean():.3f}")

    # Full evaluation across many synthetic users
    print("\n[Full evaluation — 50 synthetic users]")
    evaluate(n_users=50)
