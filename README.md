# BehaviorGuard

On-device behavioral biometrics for Android. Models typing rhythm, touch patterns, and device motion to score user authenticity — entirely on-device. No raw data ever leaves the device.

---

## How it works

```
User interaction (typing, swiping, holding device)
        │
        ▼
Signal collection — KeystrokeEvent, TouchEvent, SwipeEvent, MotionEvent
        │
        ▼
Feature extraction — 32 statistical features per session
        │
        ├─ Enrollment (first 5 sessions) ──► BaselineProfile (encrypted on-device)
        │
        └─ Scoring (after enrollment) ──────► RiskScore { score: 0.0–1.0, confidence }
```

Raw events are never stored. Feature vectors are never transmitted. Only a risk score leaves the native layer.

---

## Signal sources

| Signal | Android API | Features extracted |
|---|---|---|
| Keystroke dynamics | `InputConnection` / `KeyEvent` | Dwell time, flight time, correction rate |
| Touch | `MotionEvent` | Tap duration, pressure, contact area |
| Swipe / scroll | `MotionEvent` | Distance, average velocity, peak velocity |
| Device motion | `SensorManager` (gyro + accel) | Angular velocity, linear acceleration per axis |

---

## Quickstart

### 1 — Copy the Kotlin wrapper

```sh
cp android/BehaviorGuard.kt app/src/main/java/com/example/behaviorgaurd/BehaviorGuard.kt
```

### 2 — Build the native library

```sh
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 \
  -o app/src/main/jniLibs \
  build --release --features jni
```

### 3 — Integrate in your Activity

```kotlin
class MainActivity : AppCompatActivity() {

    private val guard = BehaviorGuard()
    private lateinit var sensorListener: SensorEventListener
    private val sensorManager by lazy { getSystemService(SensorManager::class.java) }

    override fun onResume() {
        super.onResume()
        guard.startSession()
        sensorListener = guard.registerSensors(sensorManager)
    }

    override fun dispatchTouchEvent(ev: MotionEvent): Boolean {
        guard.onTouchEvent(ev, windowManager.defaultDisplay.width, windowManager.defaultDisplay.height)
        return super.dispatchTouchEvent(ev)
    }

    override fun onPause() {
        super.onPause()
        guard.unregisterSensors(sensorManager, sensorListener)
        when (val outcome = guard.endSession()) {
            is SessionOutcome.Enrolling -> {
                Log.d("BG", "Enrolling: ${outcome.sessionsRemaining} sessions left")
            }
            is SessionOutcome.EnrollmentComplete -> {
                Log.d("BG", "Enrollment complete — scoring now available")
                saveProfile()
            }
            is SessionOutcome.Scored -> {
                Log.d("BG", "Risk score: ${outcome.score} (confidence: ${outcome.confidence})")
                if (outcome.score > 0.7f) triggerStepUpAuth()
            }
            is SessionOutcome.Error -> Log.w("BG", outcome.message)
        }
    }

    private fun saveProfile() {
        val key = getKeystoreKey()  // 32-byte AES key from Android Keystore
        val blob = guard.exportProfile(key) ?: return
        prefs.edit()
            .putString("bg_profile", Base64.encodeToString(blob, Base64.NO_WRAP))
            .apply()
    }

    override fun onDestroy() {
        super.onDestroy()
        guard.close()
    }
}
```

### 4 — Keystroke events (optional, higher accuracy)

Hook into your `EditText` via `TextWatcher` or `InputConnection`:

```kotlin
editText.addTextChangedListener(object : TextWatcher {
    private var lastUpMs = -1L

    override fun beforeTextChanged(s: CharSequence, start: Int, count: Int, after: Int) {}
    override fun onTextChanged(s: CharSequence, start: Int, before: Int, count: Int) {}

    override fun afterTextChanged(s: Editable) {
        val now = System.currentTimeMillis()
        val flight = if (lastUpMs < 0) -1L else now - lastUpMs
        val isCorrection = s.length < (s.length + 1) // backspace
        guard.onKeystroke(downMs = now, upMs = now + 80L, flightMs = flight, isCorrection = isCorrection)
        lastUpMs = now + 80L
    }
})
```

---

## Risk score interpretation

| Score | Meaning | Suggested action |
|---|---|---|
| 0.0 – 0.3 | Matches enrolled baseline | Allow |
| 0.3 – 0.6 | Moderate deviation | Log, monitor |
| 0.6 – 0.7 | Significant deviation | Soft challenge (PIN prompt) |
| 0.7 – 1.0 | High anomaly | Step-up authentication or block |

Thresholds are application-dependent. Tune based on your false-positive tolerance.

---

## Enrollment

The first **5 sessions** are used to build the baseline profile. During enrollment, `endSession()` returns `SessionOutcome.Enrolling` with the number of sessions remaining. After the 5th session it returns `SessionOutcome.EnrollmentComplete` and all subsequent sessions return `SessionOutcome.Scored`.

The profile is stored encrypted (AES-256-GCM). Supply a 32-byte key from Android Keystore to `exportProfile` / `importProfile` for persistence across app launches.

---

## Phase 2 — TFLite autoencoder (upcoming)

The current scorer uses statistical z-score distance. Phase 2 replaces it with an on-device autoencoder:

```
training/
├── train_autoencoder.py   # Train in Python, export to .tflite
└── requirements.txt       # tensorflow, numpy
```

```sh
cd training
pip install -r requirements.txt
python train_autoencoder.py --data enrolled_features.json --out model.tflite
```

The exported `model.tflite` is bundled in the APK's `assets/` folder. The native layer loads it via the TFLite C API at startup.

---

## Architecture

```
behavior_guard (Rust crate)
├── signals/          KeystrokeEvent, TouchEvent, SwipeEvent, MotionEvent
├── features/         32-feature statistical extractor
├── profile/          EnrollmentState, BaselineProfile, ProfileStore (AES-256-GCM)
├── inference/        Scorer (z-score Phase 1 → TFLite Phase 2)
└── jni_api.rs        JNI exports for Android

android/
└── BehaviorGuard.kt  Kotlin wrapper + SensorManager integration

training/
└── train_autoencoder.py  Python training script → .tflite export
```

---

## Privacy

| Data | Stays on device | Can leave device |
|---|---|---|
| Raw touch / keystroke events | ✓ | — |
| Feature vectors | ✓ | — |
| Baseline profile (encrypted) | ✓ | — |
| Risk score | — | ✓ (float 0–1) |
| Anomaly flag | — | ✓ (bool) |

---

## Requirements

| Tool | Version |
|---|---|
| Rust stable | ≥ 1.75 |
| Android NDK | r25+ (r27 recommended) |
| cargo-ndk | latest |
| minSdk | 24 (Android 7.0) |
| Python (training only) | ≥ 3.10 |
