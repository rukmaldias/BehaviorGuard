---
sidebar_position: 9
---

# Troubleshooting

Common problems and how to fix them.

---

## Most sessions return `SessionOutcome.Error`

**Symptom:** `endSession()` almost always returns `SessionOutcome.Error` with a message like "insufficient events."

**Cause:** The session doesn't contain at least 5 keystrokes **or** at least 3 touch events before `endSession()` is called.

**Fixes:**

1. **Check your session boundaries.** Are you calling `startSession()` before the user starts typing, and `endSession()` after they've had a chance to interact? If you call `endSession()` too early (e.g. immediately after the user presses a submit button before the field blur event fires), events may be lost.

2. **Confirm touch events are being forwarded.** Add a temporary log in `dispatchTouchEvent` to verify it's being called:
   ```kotlin
   override fun dispatchTouchEvent(ev: MotionEvent): Boolean {
       Log.d("BG", "touch action=${ev.action}")
       manager.onTouchEvent(ev, window.decorView.width, window.decorView.height)
       return super.dispatchTouchEvent(ev)
   }
   ```

3. **Confirm keystroke events are being forwarded.** If you're using `TextWatcher`, confirm `afterTextChanged` fires on every character including corrections.

4. **Check screen dimensions.** If `window.decorView.width` or `height` is 0 (can happen if called before layout), the touch extractor may reject all events. Use `.coerceAtLeast(1)`:
   ```kotlin
   val w = window.decorView.width.coerceAtLeast(1)
   val h = window.decorView.height.coerceAtLeast(1)
   manager.onTouchEvent(ev, w, h)
   ```

---

## Enrolled user gets high risk scores

**Symptom:** You enrolled, but `SessionOutcome.Scored` returns scores above 0.6 for yourself.

**Cause:** The enrollment baseline doesn't represent your normal behaviour.

**Fixes:**

1. **Re-enroll under normal conditions.** Call `manager.reset()` and complete 5 enrollment sessions under typical conditions — same hand, same environment, similar content to your normal use case.

2. **Use longer enrollment sessions.** Enrollment sessions with only 5–10 keystrokes produce noisy feature vectors. Try to ensure your 5 enrollment sessions each have at least 20 keystrokes.

3. **Check that keystroke timing is being captured.** If your `TextWatcher` uses estimated dwell times (fixed 80 ms), verify the `flightMs` calculation is correct:
   ```kotlin
   val flight = if (lastKeyUpMs < 0) -1L else now - lastKeyUpMs
   // Make sure lastKeyUpMs is updated after every keystroke
   lastKeyUpMs = now + estimatedDwell
   ```
   A bug where `flightMs` is always 0 or always the same value will produce a flat feature vector that doesn't represent your real rhythm.

4. **Confirm IMU sensors are registering.** If the gyroscope isn't available on the test device, features 18–31 will be zero for all sessions — this can cause instability in the autoencoder. Check with:
   ```kotlin
   val hasGyro = sensorManager.getDefaultSensor(Sensor.TYPE_GYROSCOPE) != null
   Log.d("BG", "gyro available: $hasGyro")
   ```

5. **Give the autoencoder time to stabilise.** Scores immediately after the 5th enrollment session can be higher than normal as the model has only seen 5 source vectors. Scores typically converge after several scoring sessions.

---

## Confidence is always very low

**Symptom:** `outcome.confidence` is consistently below 0.1 even in long sessions.

**Explanation:** Confidence is `min(events / 50, 1.0) × min(session_count / 10, 1.0)`. Low values mean either:

- Fewer than 50 total events per session (keystrokes + touches), **or**
- Fewer than 10 scoring sessions completed so far

**Fixes:**

1. **Increase interaction richness.** Use BehaviorGuard on screens with more input — multi-field forms, message composers, or longer sequences. A single 4-character PIN entry will never reach high confidence.

2. **Let session count accumulate.** Confidence above 0.5 on the second factor requires at least 5–6 scoring sessions. This is by design — the model is still stabilising.

3. **Widen your risk threshold** while confidence is low:
   ```kotlin
   val threshold = 0.7f + (1f - outcome.confidence) * 0.2f
   ```
   Don't block or challenge users based on low-confidence scores from early sessions.

---

## Profile not persisting across app restarts

**Symptom:** After restarting the app, `BehaviorGuardManager` doesn't restore the enrolled profile — `isEnrolled()` returns `false`.

**Fixes:**

1. **Check that `endSession()` completes before the process dies.** If the app is killed mid-`endSession()`, the `saveState()` call inside may not complete. Make sure `endSession()` is called (and awaited if on a coroutine) before `onDestroy()` returns.

2. **Verify `SharedPreferences` write permissions.** `BehaviorGuardManager` uses `Context.MODE_PRIVATE`. Confirm the `context` passed to the constructor is the application context, not a recreated Activity context:
   ```kotlin
   // In ViewModel:
   val manager = BehaviorGuardManager(application)  // ✓ Application context
   // Not:
   val manager = BehaviorGuardManager(activity)  // May cause issues on rotation
   ```

3. **Check for EncryptedSharedPreferences errors.** If the Keystore key is corrupted or the device was recently factory reset without uninstalling the app, the `EncryptedSharedPreferences` that holds the AES key may fail to open. This causes `restoreState()` to silently fail. Add a try-catch around `BehaviorGuardManager(context)` construction:
   ```kotlin
   manager = try {
       BehaviorGuardManager(this)
   } catch (e: Exception) {
       Log.e("BG", "state restore failed — starting fresh", e)
       BehaviorGuardManager(this)  // Second call will find no stored state
   }
   ```

---

## Build error: `libbehavior_guard.so` not found

**Symptom:** The app builds but crashes at launch with `java.lang.UnsatisfiedLinkError: No implementation found for ... nativeCreate`.

**Cause:** The native `.so` files aren't in `android-app/lib/src/main/jniLibs/`.

**Fix:** Run the build script before opening Android Studio or assembling the app:

```sh
# From the repo root:
./build-android.sh

# Confirm the .so files were produced:
ls android-app/lib/src/main/jniLibs/arm64-v8a/libbehavior_guard.so
```

If `cargo ndk` fails, confirm:

```sh
# Rust targets installed:
rustup target list --installed | grep android

# cargo-ndk installed:
cargo ndk --version

# NDK path set (if not auto-detected):
export ANDROID_NDK_HOME=/path/to/ndk
```

---

## Build error: `cargo ndk` missing ABI

**Symptom:** `./build-android.sh` fails with `error: target 'aarch64-linux-android' is not installed`.

**Fix:**

```sh
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
```

---

## Gradle sync fails after adding the `:lib` dependency

**Symptom:** Android Studio reports `Module ':lib' not found` or similar after adding `implementation(project(":lib"))`.

**Fix:** Confirm `settings.gradle.kts` includes `:lib` before `:app`:

```kotlin
include(":lib")
include(":app")
```

Then sync Gradle. If the error persists, invalidate caches: **File → Invalidate Caches → Invalidate and Restart**.

---

## `isModelReady()` returns false after restoring from storage

**Symptom:** `manager.isModelReady()` is `false` after a fresh `BehaviorGuardManager` construction, even though the user was enrolled.

**Cause:** The profile was restored but the model blob was missing or failed to decrypt.

**Explanation:** `BehaviorGuardManager.restoreState()` calls `importProfile()` first, then `importModel()`. If the model blob is absent in `SharedPreferences` (e.g. the app was updated from a version before Phase 2 was added), `importModel()` is skipped. The guard falls back to Phase 1 z-score scoring, which still works — `isModelReady()` just tells you which scorer is active.

**Fix:** No action needed — Phase 1 scoring is automatic. The autoencoder model will be regenerated on the next full re-enrollment. If you want to force Phase 2 mode, call `manager.reset()` and re-enroll.

---

## Getting `IllegalStateException` from `startSession()`

**Symptom:** `startSession()` throws `IllegalStateException: session already active`.

**Cause:** `startSession()` was called twice without an intervening `endSession()`.

**Fix:** Ensure session lifecycle is managed symmetrically — one `startSession()` per `endSession()`. A common mistake is calling `startSession()` in both `onCreate()` and `onResume()` without ending the session between them. Prefer `onResume()` / `onPause()` as the symmetric pair:

```kotlin
override fun onResume() {
    super.onResume()
    manager.startSession(sensorManager)
}

override fun onPause() {
    super.onPause()
    handleOutcome(manager.endSession(sensorManager))
}
```
