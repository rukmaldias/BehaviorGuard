---
sidebar_position: 1
---

# Getting Started

BehaviorGuard is an on-device behavioral biometrics SDK for Android. It models a user's typing rhythm, touch pressure, swipe velocity, and device motion to produce a continuous risk score — entirely in the native layer, with no raw data ever leaving the device.

## How it works

```
User interaction → Signal collection → 32-feature extraction → Risk score (0.0–1.0)
```

After 5 enrollment sessions, BehaviorGuard builds a per-user baseline and trains a lightweight autoencoder (32→8→32 bottleneck). From then on, each session is scored against that model. The score approaches 0.0 for the enrolled user and 1.0 for an impostor.

## Installation

### 1. Build the AAR

```bash
git clone https://github.com/rukmaldias/BehaviorGuard
cd BehaviorGuard

# Requires Rust ≥ 1.75, cargo-ndk, Android NDK r25+
./build-android.sh --publish-local
```

### 2. Add the dependency

```kotlin
// settings.gradle.kts — add mavenLocal() to repositories
// app/build.gradle.kts
dependencies {
    implementation("com.behaviorgaurd:behavior-guard:0.1.0")
}
```

### 3. Integrate

```kotlin
class MainActivity : AppCompatActivity() {
    private lateinit var manager: BehaviorGuardManager
    private val sensorManager by lazy { getSystemService(SensorManager::class.java) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        manager = BehaviorGuardManager(this)  // restores profile + model automatically
    }

    override fun dispatchTouchEvent(ev: MotionEvent): Boolean {
        manager.onTouchEvent(ev, window.decorView.width, window.decorView.height)
        return super.dispatchTouchEvent(ev)
    }

    fun startInteraction()  = manager.startSession(sensorManager)

    fun endInteraction() {
        when (val outcome = manager.endSession(sensorManager)) {
            is SessionOutcome.Enrolling         -> showProgress(outcome.sessionsRemaining)
            is SessionOutcome.EnrollmentComplete -> showReady()
            is SessionOutcome.Scored            -> handleRisk(outcome.score, outcome.confidence)
            is SessionOutcome.Error             -> { /* insufficient events — skip silently */ }
        }
    }

    override fun onDestroy() { super.onDestroy(); manager.close() }
}
```

`BehaviorGuardManager` handles Keystore key generation, sensor registration, and encrypted state persistence automatically.

## Risk score thresholds

| Score | Interpretation | Suggested response |
|---|---|---|
| 0.0 – 0.3 | Matches baseline | Allow |
| 0.3 – 0.6 | Moderate deviation | Monitor |
| 0.6 – 0.7 | Significant deviation | Soft challenge |
| 0.7 – 1.0 | High anomaly | Step-up auth / block |

Always pair the score with the `confidence` field — widen your acceptance band when confidence is low (short session, few events).

## Next steps

- **[Integration Guide](integration-guide.md)** — sensor lifecycle, keystroke events, swipe events, multi-user setup
- **[API Reference](api-reference.md)** — full docs for `BehaviorGuardManager`, `BehaviorGuard`, `SessionOutcome`
- **[Architecture](architecture.md)** — feature vector layout, enrollment math, autoencoder training details
- **[Threat Model](threat-model.md)** — what BehaviorGuard does and does not protect against
