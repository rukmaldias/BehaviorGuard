---
sidebar_position: 2
---

# Integration Guide

This guide walks through integrating BehaviorGuard into an Android app using `BehaviorGuardManager`, the recommended high-level API.

---

## Prerequisites

- Android SDK 24+ (API level 24 / Android 7.0)
- Kotlin 1.9+
- AndroidX Security Crypto (`androidx.security:security-crypto:1.1.0-alpha06+`)
- The `behavior-guard` AAR — see [Build from source](https://github.com/rukmaldias/BehaviorGuard#build-from-source)

---

## Step 1 — Add the dependency

After publishing to Maven Local with `./build-android.sh --publish-local`:

```kotlin
// settings.gradle.kts
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        mavenLocal()
        google()
        mavenCentral()
    }
}

// app/build.gradle.kts
dependencies {
    implementation("com.behaviorgaurd:behavior-guard:0.1.0")
}
```

---

## Step 2 — Create `BehaviorGuardManager`

Create one instance per user and keep it alive for the duration of the app session (a `ViewModel` is the right home for it):

```kotlin
class MainViewModel(application: Application) : AndroidViewModel(application) {
    val manager = BehaviorGuardManager(application)

    override fun onCleared() {
        super.onCleared()
        manager.close()
    }
}
```

On construction, `BehaviorGuardManager` automatically:
- Generates a 32-byte AES key in Android Keystore (or reuses the existing one)
- Decrypts and restores the enrolled profile from `SharedPreferences`
- Restores the Phase 2 autoencoder model weights

---

## Step 3 — Start a session

Call `startSession` when the user begins an interaction period — e.g. when a login screen appears, a checkout flow starts, or the user returns from background:

```kotlin
override fun onResume() {
    super.onResume()
    manager.startSession(sensorManager)
}
```

`startSession` registers gyroscope and linear acceleration listeners at `SENSOR_DELAY_GAME` (~50 Hz). Motion events are merged and fed to the native layer automatically.

---

## Step 4 — Feed events

### Touch events

Forward every `MotionEvent` from `dispatchTouchEvent`. BehaviorGuard distinguishes taps from swipes internally based on duration and movement:

```kotlin
override fun dispatchTouchEvent(ev: MotionEvent): Boolean {
    val w = window.decorView.width.coerceAtLeast(1)
    val h = window.decorView.height.coerceAtLeast(1)
    manager.onTouchEvent(ev, w, h)
    return super.dispatchTouchEvent(ev)
}
```

Screen dimensions normalise touch coordinates to [0, 1] for model stability across device sizes.

### Keystroke events

Hook into your `EditText` via `TextWatcher`. For best accuracy, supply exact key-down and key-up timestamps from `InputConnection` or `KeyEvent`; fall back to a fixed 80 ms estimate for soft keyboards:

```kotlin
private var lastKeyUpMs = -1L

editText.addTextChangedListener(object : TextWatcher {
    override fun beforeTextChanged(s: CharSequence, start: Int, count: Int, after: Int) {}
    override fun onTextChanged(s: CharSequence, start: Int, before: Int, count: Int) {}

    override fun afterTextChanged(s: Editable) {
        val now          = System.currentTimeMillis()
        val isCorrection = s.length < previousLength  // backspace / delete
        val flight       = if (lastKeyUpMs < 0) -1L else now - lastKeyUpMs
        val estimatedDwell = 80L

        manager.onKeystroke(
            downMs       = now,
            upMs         = now + estimatedDwell,
            flightMs     = flight,
            isCorrection = isCorrection,
        )
        lastKeyUpMs = now + estimatedDwell
    }
})
```

**Note:** Keystroke timing is the strongest single signal. Even approximate dwell times meaningfully improve accuracy over touch-only data.

### Swipe events

If your app uses custom swipe tracking (e.g. with `GestureDetector` or `VelocityTracker`), forward completed swipes:

```kotlin
manager.onSwipe(
    startMs       = gestureStartMs,
    endMs         = System.currentTimeMillis(),
    startX        = startX / screenWidth,
    startY        = startY / screenHeight,
    endX          = endX / screenWidth,
    endY          = endY / screenHeight,
    peakVelocity  = tracker.xVelocity,   // px/s
)
```

Swipe events are optional — `onTouchEvent` already captures touch pressure and duration.

---

## Step 5 — End the session and handle the outcome

```kotlin
override fun onPause() {
    super.onPause()
    handleOutcome(manager.endSession(sensorManager))
}

private fun handleOutcome(outcome: SessionOutcome) {
    when (outcome) {

        is SessionOutcome.Enrolling -> {
            // Still building the baseline — no score available yet.
            showEnrollmentProgress(sessionsRemaining = outcome.sessionsRemaining)
        }

        is SessionOutcome.EnrollmentComplete -> {
            // The 5-session baseline is ready. The Phase 2 autoencoder
            // was trained automatically and is now active.
            showEnrolledState()
        }

        is SessionOutcome.Scored -> {
            // Apply your policy:
            when {
                outcome.score < 0.3f -> allow()
                outcome.score < 0.6f -> monitor(outcome.score)
                outcome.score < 0.7f -> softChallenge()
                else                 -> stepUpAuth()
            }
            // outcome.confidence: how much signal contributed [0, 1].
            // Low confidence = short session or few events — consider
            // being more lenient at low confidence values.
        }

        is SessionOutcome.Error -> {
            // Insufficient events (< 5 keystrokes or < 3 touches).
            // Silently discard — don't penalise the user.
            log("BG session skipped: ${outcome.message}")
        }
    }
}
```

`endSession` automatically:
- Unregisters IMU sensors
- Saves the updated profile and autoencoder model weights (encrypted)

---

## State persistence

Everything is handled by `BehaviorGuardManager` internally. You do not need to call `exportProfile` or `exportModel` manually.

Under the hood:
1. A 32-byte AES key is generated by `SecureRandom` and stored in `EncryptedSharedPreferences` backed by Android Keystore (`AES256_GCM`).
2. After each `endSession`, the profile blob (magic `BGPROF01` + AES-256-GCM ciphertext) and the autoencoder JSON weights are both Base64-encoded and written to a plain `SharedPreferences` file.
3. On the next `BehaviorGuardManager` construction, both are decrypted and loaded into the native layer.

**To use your own storage** (encrypted database, secure file), use the lower-level `BehaviorGuard` class directly and call `exportProfile`/`importProfile` and `exportModel`/`importModel` yourself.

---

## Resetting enrollment

```kotlin
// Wipes profile, model, and Keystore key.
// Create a new BehaviorGuardManager instance after calling this.
manager.reset()
```

Typical triggers: user logs out, device is transferred to a new user, or the user explicitly requests re-enrollment.

---

## Minimum session requirements

A session must contain at least one of:
- 5 or more keystroke events, **or**
- 3 or more touch events

Sessions below this threshold return `SessionOutcome.Error`. This is intentional — a 2-event session provides no meaningful signal and would corrupt the baseline.

---

## Multiple users on one device

Create one `BehaviorGuardManager` per user and prefix the `SharedPreferences` keys with the user ID. The current implementation uses a single shared preferences file (`bg_state`); for multi-user support, subclass `BehaviorGuardManager` and override the preferences name.

---

## Testing your integration

The demo app in `android-app/app/` demonstrates the complete flow — enrollment, scoring, profile persistence, and step-up auth decision. Run it on a physical device (emulators lack IMU sensors and produce less representative touch data).

To force re-enrollment in the demo, press "Clear Profile" and restart the app.
