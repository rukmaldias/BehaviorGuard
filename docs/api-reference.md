---
sidebar_position: 3
---

# API Reference

All public types live in the `com.behaviorgaurd` package.

---

## `BehaviorGuardManager`

High-level manager. Recommended entry point for most integrations.

```kotlin
class BehaviorGuardManager(context: Context) : AutoCloseable
```

### Constructor

```kotlin
BehaviorGuardManager(context: Context)
```

Restores previously persisted profile and autoencoder model from encrypted storage. Safe to call on the main thread; disk I/O is minimal (< 1 ms typical).

### Session methods

```kotlin
fun startSession(sensorManager: SensorManager)
```
Starts a new collection session and registers IMU sensors (gyroscope + linear acceleration at `SENSOR_DELAY_GAME`). Calling this while a session is already active throws `IllegalStateException` from the native layer.

```kotlin
fun endSession(sensorManager: SensorManager): SessionOutcome
```
Ends the active session, unregisters sensors, extracts features from the collected events, and returns a `SessionOutcome`. Persists the updated profile and model automatically.

Returns `SessionOutcome.Error` if the session had insufficient events (fewer than 5 keystrokes and fewer than 3 touches).

### Event methods

```kotlin
fun onTouchEvent(event: MotionEvent, screenWidth: Int, screenHeight: Int)
```
Call from `Activity.dispatchTouchEvent` for every event. Screen dimensions normalise touch coordinates to [0, 1].

```kotlin
fun onKeystroke(downMs: Long, upMs: Long, flightMs: Long, isCorrection: Boolean)
```
Feed a keystroke event. `flightMs` is the elapsed time since the previous key was released; pass `-1` for the first key of the session. `isCorrection` is `true` for backspace/delete keys.

```kotlin
fun onSwipe(
    startMs: Long, endMs: Long,
    startX: Float, startY: Float,
    endX: Float, endY: Float,
    peakVelocity: Float,
)
```
Feed a completed swipe gesture. Coordinates should be normalised to [0, 1]. `peakVelocity` is in pixels per second. Optional — `onTouchEvent` already captures tap pressure and duration.

### State queries

```kotlin
fun isEnrolled(): Boolean
```
Returns `true` once the 5-session baseline is complete.

```kotlin
fun isModelReady(): Boolean
```
Returns `true` once the Phase 2 autoencoder is trained and active. Always `false` until enrollment is complete; may be `false` after `importProfile` if the model blob was not separately restored via `importModel`.

### State management

```kotlin
fun reset()
```
Clears all persisted state including the Keystore key. **Do not reuse the instance after calling this** — create a new `BehaviorGuardManager`.

```kotlin
override fun close()
```
Frees the native memory. Call from `onDestroy` or `ViewModel.onCleared`.

---

## `BehaviorGuard`

Low-level JNI wrapper. Use this instead of `BehaviorGuardManager` when you need custom storage, multi-user support, or direct control over when state is persisted.

```kotlin
class BehaviorGuard : AutoCloseable
```

### Session

```kotlin
fun startSession()
fun endSession(): SessionOutcome
```

### Event ingestion

```kotlin
fun onTouchEvent(event: MotionEvent, screenWidth: Int, screenHeight: Int)
fun onKeystroke(downMs: Long, upMs: Long, flightMs: Long, isCorrection: Boolean)
fun onSwipe(startMs: Long, endMs: Long, startX: Float, startY: Float,
            endX: Float, endY: Float, peakVelocity: Float)
fun onMotion(gyroX: Float, gyroY: Float, gyroZ: Float,
             accelX: Float, accelY: Float, accelZ: Float)
```

`onMotion` is called automatically by the sensor listener returned from `registerSensors`. Call it directly only if you manage sensors yourself.

### Sensor lifecycle helpers

```kotlin
fun registerSensors(sensorManager: SensorManager): SensorEventListener
fun unregisterSensors(sensorManager: SensorManager, listener: SensorEventListener)
```

### State queries

```kotlin
fun isEnrolled(): Boolean
fun isModelReady(): Boolean
```

### Profile persistence

```kotlin
fun exportProfile(key: ByteArray): ByteArray?
fun importProfile(blob: ByteArray, key: ByteArray): Boolean
```

`key` must be exactly 32 bytes (AES-256). Returns `null` / `false` if enrollment is not complete or decryption fails. The blob format is `BGPROF01 || 12-byte nonce || AES-GCM ciphertext`.

### Model persistence

```kotlin
fun exportModel(): ByteArray?
fun importModel(bytes: ByteArray): Boolean
```

Exports / imports the Phase 2 autoencoder weights as JSON. Call `importProfile` before `importModel` — the model requires an enrolled profile to be present. Returns `null` / `false` if not enrolled or JSON is malformed.

### Lifecycle

```kotlin
override fun close()
```

---

## `SessionOutcome`

Sealed class returned by `endSession`.

```kotlin
sealed class SessionOutcome {

    /** Enrollment in progress. Collect [sessionsRemaining] more sessions. */
    data class Enrolling(val sessionsRemaining: Int) : SessionOutcome()

    /**
     * The 5th enrollment session just completed.
     * The Phase 2 autoencoder was trained automatically.
     * All subsequent sessions will return [Scored].
     */
    object EnrollmentComplete : SessionOutcome()

    /**
     * A risk score is available.
     *
     * @param score      Anomaly score in [0.0, 1.0].
     *                   0.0 = behaviour matches the enrolled baseline exactly.
     *                   1.0 = maximally anomalous.
     * @param confidence Signal richness in [0.0, 1.0].
     *                   Low confidence = short session or few events.
     *                   Scale your risk threshold inversely with confidence
     *                   to reduce false positives in short sessions.
     */
    data class Scored(val score: Float, val confidence: Float) : SessionOutcome()

    /**
     * The session could not be scored.
     * Most common cause: insufficient events (< 5 keystrokes and < 3 touches).
     * Do not penalise the user — silently discard and let the next session score.
     */
    data class Error(val message: String) : SessionOutcome()
}
```

### Working with confidence

Confidence is computed as:

```
confidence = min(events / 50, 1.0) × min(session_count / 10, 1.0)
```

A session with 10 events after exactly 5 enrollment sessions gives:
- `min(10/50, 1) = 0.20`
- `min(5/10, 1) = 0.50`
- `confidence = 0.10`

At low confidence, widen your acceptance band:

```kotlin
val adjustedThreshold = 0.7f + (1f - outcome.confidence) * 0.2f
if (outcome.score > adjustedThreshold) triggerStepUpAuth()
```

---

## Native JNI API

The JNI functions are exported from `libbehavior_guard.so` under the package `com.behaviorgaurd`. They are not part of the public API — use `BehaviorGuard` or `BehaviorGuardManager` instead.

| Export | Return | Description |
|---|---|---|
| `nativeCreate` | `Long` | Allocates a `BehaviorGuard` instance; returns an opaque handle |
| `nativeDestroy(handle)` | — | Frees native memory |
| `nativeStartSession(handle)` | `Boolean` | Starts a session |
| `nativeEndSession(handle)` | `Int` | < -1 = enrolling (abs = remaining), 0 = complete, > 0 = score × 1000 |
| `nativeAddKeystroke(…)` | — | |
| `nativeAddTouch(…)` | — | |
| `nativeAddSwipe(…)` | — | |
| `nativeAddMotion(…)` | — | |
| `nativeExportProfile(handle, key)` | `ByteArray?` | |
| `nativeImportProfile(handle, blob, key)` | `Boolean` | |
| `nativeIsEnrolled(handle)` | `Boolean` | |
| `nativeExportModel(handle)` | `ByteArray?` | |
| `nativeImportModel(handle, bytes)` | `Boolean` | |
| `nativeIsModelReady(handle)` | `Boolean` | |
