# BehaviorGuard

On-device behavioral biometrics for Android. Scores user authenticity by modelling typing rhythm, touch patterns, and device motion — entirely in the native layer. No raw data ever leaves the device.

**Full technical reference:** [`documentation`](https://rukmaldias.github.io/BehaviorGuard/) 

```
User interaction → Signal collection → Feature extraction → Risk score (f32 0.0–1.0)
```

---

## At a glance

| Property | Value |
|---|---|
| Language | Rust (core) + Kotlin (wrapper) |
| Signal types | Keystroke timing · Touch pressure/area · Swipe velocity · IMU (gyro + accel) |
| Feature vector | 32 statistical features per session |
| Scorer — Phase 1 | Mean absolute z-score vs enrolled baseline |
| Scorer — Phase 2 | Per-user autoencoder trained on-device at enrollment (32→16→8→16→32) |
| Profile encryption | AES-256-GCM, key from Android Keystore |
| Enrollment sessions | 5 |
| Output | `f32` risk score + confidence, no raw events exported |
| Min SDK | 24 (Android 7.0) |
| ABI targets | `arm64-v8a` · `armeabi-v7a` · `x86_64` |

---

## Quick integration

### 1. Build the native library and AAR

```sh
git clone https://github.com/rukmaldias/BehaviorGuard
cd BehaviorGuard

# Requires: Rust stable ≥ 1.75, cargo-ndk, Android NDK r25+
./build-android.sh --publish-local
```

### 2. Add the dependency

```kotlin
// settings.gradle.kts
dependencyResolutionManagement {
    repositories {
        mavenLocal()
        // ...
    }
}

// app/build.gradle.kts
dependencies {
    implementation("com.behaviorgaurd:behavior-guard:0.1.0")
}
```

### 3. Integrate in your Activity

```kotlin
class MainActivity : AppCompatActivity() {

    // One instance per user — keep long-lived (ViewModel recommended).
    // Automatically restores profile + Phase 2 model on construction.
    private lateinit var manager: BehaviorGuardManager
    private val sensorManager by lazy { getSystemService(SensorManager::class.java) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        manager = BehaviorGuardManager(this)
    }

    // Call at the start of each interaction period
    fun onInteractionStart() {
        manager.startSession(sensorManager)   // registers gyro + accel
    }

    // Forward every touch event
    override fun dispatchTouchEvent(ev: MotionEvent): Boolean {
        val w = window.decorView.width
        val h = window.decorView.height
        manager.onTouchEvent(ev, w, h)
        return super.dispatchTouchEvent(ev)
    }

    // Call when the interaction period ends
    fun onInteractionEnd() {
        when (val outcome = manager.endSession(sensorManager)) {
            is SessionOutcome.Enrolling ->
                showProgress(outcome.sessionsRemaining)

            is SessionOutcome.EnrollmentComplete ->
                showReady()

            is SessionOutcome.Scored -> {
                if (outcome.score > 0.7f) triggerStepUpAuth()
                log("risk=${outcome.score}, confidence=${outcome.confidence}")
            }

            is SessionOutcome.Error ->
                log(outcome.message)
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        manager.close()
    }
}
```

`BehaviorGuardManager` handles key generation, profile encryption, autoencoder model persistence, and sensor lifecycle automatically.

For keystroke timing, see [Integration Guide → Keystroke events](docs/integration-guide.md#keystroke-events).

---

## Risk score

| Score | Interpretation | Suggested action |
|---|---|---|
| 0.0 – 0.3 | Matches enrolled baseline | Allow |
| 0.3 – 0.6 | Moderate deviation | Log / monitor |
| 0.6 – 0.7 | Significant deviation | Soft challenge (PIN, FaceID re-prompt) |
| 0.7 – 1.0 | High anomaly | Step-up auth or block |

Thresholds are application-dependent. Tune based on your false-positive tolerance and risk appetite.

---

## How it works

```
Session signals (touch, keys, swipe, motion)
         │
         ▼
Feature extraction — 32-dim statistical vector
         │
         ├─── First 5 sessions ──► Enrollment
         │                              │
         │              ┌───────────────┴──────────────────┐
         │              │                                  │
         │       BaselineProfile                    Autoencoder
         │       (mean + std per feature)           (trained on z-normalised
         │                                           enrollment vectors)
         │
         └─── After enrollment ──► Scoring
                                        │
                          ┌─────────────┴──────────────┐
                          │                            │
                    Phase 1 (fallback)          Phase 2 (default)
                    z-score distance            Autoencoder reconstruction
                    from baseline               error — captures joint
                                                feature correlations
```

Both the profile and the autoencoder weights are AES-256-GCM encrypted at rest. The 32-byte key lives in Android Keystore and never leaves the device.

See [Architecture](docs/architecture.md) for the feature layout and training procedure.

---

## Build from source

**Prerequisites**

| Tool | Install |
|---|---|
| Rust stable ≥ 1.75 | `rustup update stable` |
| Android NDK r25+ | Android Studio → SDK Manager → NDK |
| cargo-ndk | `cargo install cargo-ndk` |
| Android targets | `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android` |

**Build**

```sh
./build-android.sh               # build .so + assemble AAR
./build-android.sh --debug       # debug build
./build-android.sh --publish-local   # build + publish to ~/.m2/
```

The AAR is written to `android-app/lib/build/outputs/aar/`.

**Run the demo app**

Open `android-app/` in Android Studio. `:app` depends on `:lib` via a project dependency — no additional setup after running the build script.

---

## Autoencoder validation

```sh
pip install -r scripts/requirements.txt
python scripts/validate_autoencoder.py
```

Trains the autoencoder on 50 synthetic users and reports EER, FAR/FRR curve, and enrolled vs impostor MSE distributions. Useful for tuning hyperparameters before a Rust recompile.

---

## Documentation

| Document | Contents |
|---|---|
| [Integration Guide](docs/integration-guide.md) | Step-by-step Android integration, event types, persistence |
| [API Reference](docs/api-reference.md) | `BehaviorGuardManager`, `BehaviorGuard`, `SessionOutcome` |
| [Architecture](docs/architecture.md) | Signal pipeline, feature layout, Phase 1/2 scoring |
| [Threat Model](docs/threat-model.md) | What BehaviorGuard does and does not protect against |

---

## Privacy

| Data | Stays on device | Can leave device |
|---|---|---|
| Raw touch / keystroke events | ✓ | — |
| Feature vectors (32 f32) | ✓ | — |
| Baseline profile (encrypted) | ✓ | — |
| Autoencoder weights (encrypted) | ✓ | — |
| Risk score | — | ✓ (single f32) |

No network calls, no analytics, no telemetry.

---

## License

GPL-3.0 — see [LICENSE](LICENSE).
