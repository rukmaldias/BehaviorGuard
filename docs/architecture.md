---
sidebar_position: 4
---

# Architecture

BehaviorGuard is a Rust crate (`behavior_guard`) with a thin Kotlin wrapper. All biometric signal processing happens in the native layer.

---

## Module layout

```
behavior_guard (Rust)
├── signals/          Raw event types
├── features/         32-feature statistical extractor
├── profile/          Enrollment state, baseline profile, encrypted storage
├── inference/
│   ├── scorer.rs     Phase 1 (z-score) and Phase 2 (autoencoder) scorers
│   └── autoencoder.rs  Per-user autoencoder — training + inference
└── jni_api.rs        JNI exports for Android

android-app/lib/src/main/java/com/behaviorgaurd/
├── BehaviorGuard.kt           Low-level JNI wrapper
└── BehaviorGuardManager.kt    High-level manager (key lifecycle, persistence)
```

---

## Signal pipeline

```
Android UI events
        │
        │  onTouchEvent / onKeystroke / onSwipe / SensorEvent
        ▼
JNI layer (jni_api.rs)
        │
        │  nativeAdd*()
        ▼
Session buffer  [Vec<RawEvent>]
        │
        │  end_session() → features::extract()
        ▼
Feature extraction (features/extractor.rs)
        │
        │  32 f32 values
        ▼
    ┌───┴───────────────────┐
    │                       │
  Enrolling             Ready (enrolled)
    │                       │
    │  EnrollmentState       │  z_normalize(fv, profile)
    │  .add(fv)              │          │
    │                        │          ▼
    │  After 5 sessions:     │  Scorer::score_with_model(fv, profile, model)
    │  BaselineProfile       │  OR
    │  Autoencoder::fit()    │  Scorer::score(fv, profile)   ← fallback
    ▼                        ▼
  EnrollmentComplete      Scored(RiskScore)
```

---

## Raw event types

| Type | Fields | Notes |
|---|---|---|
| `KeystrokeEvent` | `down_ms`, `up_ms`, `flight_ms`, `is_correction` | `flight_ms = None` for first key |
| `TouchEvent` | `down_ms`, `up_ms`, `x`, `y`, `pressure`, `area` | x/y normalised to [0,1] |
| `SwipeEvent` | `start_ms`, `end_ms`, `start_{x,y}`, `end_{x,y}`, `peak_velocity` | coordinates normalised to [0,1] |
| `MotionEvent` | `timestamp_ms`, `gyro_{x,y,z}`, `accel_{x,y,z}` | merged from two Android sensors |

Raw events are never stored to disk. They exist only in the in-memory session buffer and are discarded after `end_session()`.

---

## Feature vector (32 dimensions)

The extractor computes a 32-element `f32` vector from a session's raw events. Features are grouped by signal type:

| Index | Signal | Feature |
|---|---|---|
| 0 | Keystroke | Dwell time mean (ms) |
| 1 | Keystroke | Dwell time std |
| 2 | Keystroke | Flight time mean (ms) |
| 3 | Keystroke | Flight time std |
| 4 | Keystroke | Correction rate [0,1] |
| 5 | Keystroke | Event count norm (capped at 1.0 for ≥ 200 keystrokes) |
| 6 | Touch | Tap duration mean (ms) |
| 7 | Touch | Tap duration std |
| 8 | Touch | Pressure mean [0,1] |
| 9 | Touch | Pressure std |
| 10 | Touch | Contact area mean [0,1] |
| 11 | Touch | Contact area std |
| 12 | Swipe | Distance mean |
| 13 | Swipe | Distance std |
| 14 | Swipe | Average velocity mean (units/s) |
| 15 | Swipe | Average velocity std |
| 16 | Swipe | Peak velocity mean |
| 17 | Swipe | Peak velocity std |
| 18 | Motion | Gyro X mean (rad/s) |
| 19 | Motion | Gyro X std |
| 20 | Motion | Gyro Y mean |
| 21 | Motion | Gyro Y std |
| 22 | Motion | Gyro Z mean |
| 23 | Motion | Gyro Z std |
| 24 | Motion | Accel X mean (m/s²) |
| 25 | Motion | Accel X std |
| 26 | Motion | Accel Y mean |
| 27 | Motion | Accel Y std |
| 28 | Motion | Accel Z mean |
| 29 | Motion | Accel Z std |
| 30 | Motion | Gyro magnitude mean |
| 31 | Motion | Accel magnitude mean |

Features for a missing signal type (e.g. no swipe events in a session) are set to 0.0.

**Minimum requirements:** `extract()` returns `None` — causing `SessionOutcome.Error` — if the session has fewer than 5 keystrokes **and** fewer than 3 touch events.

---

## Enrollment and BaselineProfile

`EnrollmentState` accumulates feature vectors from the first 5 sessions. Once complete, `BaselineProfile::from_vectors()` computes per-feature mean and standard deviation:

```
mean[i]  = Σ fv[j][i] / n
std[i]   = sqrt(Σ (fv[j][i] - mean[i])² / n)
         = max(computed_std, 1e-6)   ← floored to prevent division-by-zero
```

The profile is the only persistent artefact of Phase 1 scoring — the raw feature vectors are discarded after `build_profile()`.

---

## Phase 1 scorer — z-score

```
mean_z = (1/32) × Σ |( fv[i] - profile.mean[i] ) / profile.std[i]|
score  = min(mean_z / 3.0, 1.0)
```

Interpretation: `mean_z = 1` → score = 0.33 (one standard deviation from baseline); `mean_z = 3` → score = 1.0 (three standard deviations, maximally anomalous).

This scorer is fast and requires no additional parameters. It is used as a fallback if the autoencoder model is not available.

**Limitation:** z-score treats each feature independently. It misses joint correlations — e.g. a fast typist has *both* short dwell and short flight time. An impostor who matches one dimension but not the other may score lower than expected.

---

## Phase 2 scorer — autoencoder

### Architecture

```
Input  (32)  →  ReLU(16)  →  ReLU(8)  →  ReLU(16)  →  Linear(32)  = Reconstruction
```

1,352 parameters (~5 KB serialised as JSON). Small enough to train in < 1 second on a mid-range phone.

### Training

Training fires automatically at the end of the 5th enrollment session:

1. The 5 enrollment feature vectors are **z-normalised** against the completed `BaselineProfile` (clamped to ±8 to prevent blow-up when `std` is near its 1e-6 floor).
2. Each z-vector is augmented with 80 noisy copies (Gaussian noise σ = 0.12), giving 405 training samples.
3. A 300-epoch batch gradient descent loop with cosine-annealed learning rate (8×10⁻⁴ → 0) trains the autoencoder to reconstruct the enrolled user's z-vectors.
4. Weights are initialised with Xavier normal init (seed fixed for reproducibility).

### Scoring

At scoring time:
1. The test feature vector is z-normalised using the enrolled profile.
2. The autoencoder reconstructs the z-vector.
3. Mean squared reconstruction error is computed: `MSE = mean((z - reconstruct(z))²)`.
4. Score: `min(MSE / MAX_RECONSTRUCTION_MSE, 1.0)` where `MAX_RECONSTRUCTION_MSE = 3.5`.

### Why autoencoder beats z-score

Enrolled users have correlated features: fast typists have *both* short dwell and short flight time. The autoencoder learns this joint structure. An impostor who matches one dimension but deviates in another will produce high reconstruction error because the autoencoder cannot map their input to a plausible enrolled-user vector through the 8-dim bottleneck.

### Calibration

`MAX_RECONSTRUCTION_MSE = 3.5` is calibrated for z-normalised features (typical range ±3σ). Run the validation script to see the enrolled vs impostor MSE distributions for your specific user population and tune accordingly:

```sh
python scripts/validate_autoencoder.py
```

---

## Profile encryption

```
seal(profile, key):
    nonce  ← SecureRandom (12 bytes)
    ct     ← AES-256-GCM-Encrypt(key, nonce, JSON(profile))
    return "BGPROF01" || nonce || ct

open(blob, key):
    assert blob[0..8] == "BGPROF01"
    nonce ← blob[8..20]
    ct    ← blob[20..]
    return AES-256-GCM-Decrypt(key, nonce, ct)
```

The `key` is a 32-byte value held in Android Keystore (hardware-backed on supported devices). `BehaviorGuardManager` generates it with `SecureRandom` and stores it in `EncryptedSharedPreferences` — itself Keystore-backed.

The autoencoder model weights are serialised as JSON and passed through the same encryption path via `exportModel` / `importModel`.

---

## Memory safety

- All weights are zeroed on `Autoencoder::drop()` via the `zeroize` crate.
- Session buffers are `Vec<RawEvent>` owned by `BehaviorGuard` and freed on `close()` or on `end_session()`.
- The JNI handle is a raw pointer to a `Box<Mutex<BehaviorGuard>>`. `nativeDestroy` reconstructs and drops the `Box`, ensuring all memory is freed even if `close()` is called from JNI rather than Kotlin.
