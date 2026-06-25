package com.example.behaviorgaurd

import android.content.Context
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.view.MotionEvent

/**
 * BehaviorGuard — on-device behavioral biometrics.
 *
 * ## Lifecycle
 * ```kotlin
 * val guard = BehaviorGuard()
 *
 * // Start a session when the user begins interacting
 * guard.startSession()
 *
 * // Feed events from your Activity / Fragment
 * guard.onTouchEvent(motionEvent, screenWidth, screenHeight)
 * guard.onKeystroke(downMs, upMs, flightMs, isCorrection)
 *
 * // End the session (e.g. when user navigates away or submits a form)
 * val outcome = guard.endSession()
 * when (outcome) {
 *     is SessionOutcome.Enrolling -> showProgress(outcome.sessionsRemaining)
 *     is SessionOutcome.EnrollmentComplete -> showReady()
 *     is SessionOutcome.Scored -> handleRisk(outcome.score, outcome.confidence)
 * }
 * ```
 *
 * ## Profile persistence
 * Export the encrypted profile blob before the app is killed and import it on
 * the next launch:
 * ```kotlin
 * val blob = guard.exportProfile(keystoreKey) ?: return
 * prefs.edit().putString("bg_profile", Base64.encodeToString(blob, NO_WRAP)).apply()
 *
 * // On next launch:
 * val blob = Base64.decode(prefs.getString("bg_profile", null) ?: return, NO_WRAP)
 * guard.importProfile(blob, keystoreKey)
 * ```
 */
class BehaviorGuard : AutoCloseable {

    private val handle: Long = nativeCreate()
    private var sessionStartMs: Long = 0

    // ── Session ───────────────────────────────────────────────────────────────

    fun startSession() {
        sessionStartMs = System.currentTimeMillis()
        nativeStartSession(handle)
    }

    /**
     * Ends the active session and returns the outcome.
     *
     * Raw return values from native:
     *   < -1  → enrolling, abs value = sessions remaining
     *   -1    → session error / insufficient events
     *    0    → enrollment just completed
     *   >0    → risk score * 1000 (e.g. 750 = score 0.75)
     */
    fun endSession(): SessionOutcome {
        val raw = nativeEndSession(handle)
        return when {
            raw < -1 -> SessionOutcome.Enrolling(sessionsRemaining = -raw)
            raw == 0 -> SessionOutcome.EnrollmentComplete
            raw > 0  -> SessionOutcome.Scored(
                score = raw / 1000f,
                confidence = 1.0f  // TODO: expose from native
            )
            else -> SessionOutcome.Error("Insufficient events or no session active")
        }
    }

    // ── Event ingestion ───────────────────────────────────────────────────────

    /**
     * Feed a [MotionEvent] from your Activity's `onTouchEvent` or a
     * `View.OnTouchListener`. Call this for every event — the native layer
     * separates taps from swipes based on duration and distance.
     */
    fun onTouchEvent(event: MotionEvent, screenWidth: Int, screenHeight: Int) {
        val relMs = System.currentTimeMillis() - sessionStartMs
        when (event.actionMasked) {
            MotionEvent.ACTION_UP -> {
                val x = event.x / screenWidth
                val y = event.y / screenHeight
                val pressure = event.pressure.coerceIn(0f, 1f)
                val area = event.size.coerceIn(0f, 1f)
                // Duration tracked by native via down/up timestamps
                nativeAddTouch(handle, relMs, relMs, x, y, pressure, area)
            }
        }
    }

    /**
     * Feed a keystroke from your `InputConnection` or `KeyEvent` listener.
     *
     * @param downMs   Epoch ms when the key was pressed.
     * @param upMs     Epoch ms when the key was released.
     * @param flightMs Ms since the previous key was released, or -1 for first key.
     * @param isCorrection True if this was a backspace/delete.
     */
    fun onKeystroke(downMs: Long, upMs: Long, flightMs: Long, isCorrection: Boolean) {
        val relDown = downMs - sessionStartMs
        val relUp = upMs - sessionStartMs
        nativeAddKeystroke(handle, relDown, relUp, flightMs, if (isCorrection) 1 else 0)
    }

    /**
     * Feed a swipe gesture. Call this when you have complete start/end data
     * (i.e. on ACTION_UP after determining this was a scroll/swipe).
     */
    fun onSwipe(
        startMs: Long, endMs: Long,
        startX: Float, startY: Float,
        endX: Float, endY: Float,
        peakVelocity: Float,
    ) {
        val relStart = startMs - sessionStartMs
        val relEnd = endMs - sessionStartMs
        nativeAddSwipe(handle, relStart, relEnd, startX, startY, endX, endY, peakVelocity)
    }

    /**
     * Feed a motion sensor snapshot. Call this from your [SensorEventListener]
     * with `TYPE_GYROSCOPE` and `TYPE_LINEAR_ACCELERATION` merged.
     */
    fun onMotion(
        gyroX: Float, gyroY: Float, gyroZ: Float,
        accelX: Float, accelY: Float, accelZ: Float,
    ) {
        val relMs = System.currentTimeMillis() - sessionStartMs
        nativeAddMotion(handle, relMs, gyroX, gyroY, gyroZ, accelX, accelY, accelZ)
    }

    // ── Sensor registration helper ────────────────────────────────────────────

    /**
     * Registers gyroscope + accelerometer listeners on [sensorManager].
     * Call [unregisterSensors] when done.
     */
    fun registerSensors(sensorManager: SensorManager): SensorEventListener {
        val listener = object : SensorEventListener {
            private val gyro = FloatArray(3)
            private val accel = FloatArray(3)

            override fun onSensorChanged(event: SensorEvent) {
                when (event.sensor.type) {
                    Sensor.TYPE_GYROSCOPE -> gyro[0] = event.values[0].also {
                        gyro[1] = event.values[1]; gyro[2] = event.values[2]
                    }
                    Sensor.TYPE_LINEAR_ACCELERATION -> {
                        accel[0] = event.values[0]
                        accel[1] = event.values[1]
                        accel[2] = event.values[2]
                        onMotion(gyro[0], gyro[1], gyro[2], accel[0], accel[1], accel[2])
                    }
                }
            }
            override fun onAccuracyChanged(sensor: Sensor, accuracy: Int) {}
        }
        sensorManager.getDefaultSensor(Sensor.TYPE_GYROSCOPE)?.let {
            sensorManager.registerListener(listener, it, SensorManager.SENSOR_DELAY_GAME)
        }
        sensorManager.getDefaultSensor(Sensor.TYPE_LINEAR_ACCELERATION)?.let {
            sensorManager.registerListener(listener, it, SensorManager.SENSOR_DELAY_GAME)
        }
        return listener
    }

    fun unregisterSensors(sensorManager: SensorManager, listener: SensorEventListener) {
        sensorManager.unregisterListener(listener)
    }

    // ── Profile ───────────────────────────────────────────────────────────────

    fun isEnrolled(): Boolean = nativeIsEnrolled(handle)

    /** True once the Phase 2 autoencoder has been trained and is active. */
    fun isModelReady(): Boolean = nativeIsModelReady(handle)

    /**
     * Exports the encrypted baseline profile. Returns null if not yet enrolled.
     * @param key 32-byte AES key from Android Keystore.
     */
    fun exportProfile(key: ByteArray): ByteArray? = nativeExportProfile(handle, key)

    /**
     * Imports a previously exported profile blob.
     * After importing, call [importModel] to restore the Phase 2 autoencoder.
     * @param key 32-byte AES key from Android Keystore.
     */
    fun importProfile(blob: ByteArray, key: ByteArray): Boolean =
        nativeImportProfile(handle, blob, key)

    /**
     * Exports the Phase 2 autoencoder weights (~5 KB JSON).
     * Returns null if enrollment is not complete or model training failed.
     * Save alongside the profile blob and restore with [importModel].
     */
    fun exportModel(): ByteArray? = nativeExportModel(handle)

    /**
     * Restores a previously exported autoencoder.
     * Must be called after [importProfile].
     */
    fun importModel(bytes: ByteArray): Boolean = nativeImportModel(handle, bytes)

    override fun close() {
        nativeDestroy(handle)
    }

    // ── JNI ──────────────────────────────────────────────────────────────────

    private external fun nativeCreate(): Long
    private external fun nativeDestroy(handle: Long)
    private external fun nativeStartSession(handle: Long): Boolean
    private external fun nativeEndSession(handle: Long): Int
    private external fun nativeAddKeystroke(handle: Long, downMs: Long, upMs: Long, flightMs: Long, isCorrection: Int)
    private external fun nativeAddTouch(handle: Long, downMs: Long, upMs: Long, x: Float, y: Float, pressure: Float, area: Float)
    private external fun nativeAddSwipe(handle: Long, startMs: Long, endMs: Long, startX: Float, startY: Float, endX: Float, endY: Float, peakVelocity: Float)
    private external fun nativeAddMotion(handle: Long, timestampMs: Long, gyroX: Float, gyroY: Float, gyroZ: Float, accelX: Float, accelY: Float, accelZ: Float)
    private external fun nativeExportProfile(handle: Long, key: ByteArray): ByteArray?
    private external fun nativeImportProfile(handle: Long, blob: ByteArray, key: ByteArray): Boolean
    private external fun nativeIsEnrolled(handle: Long): Boolean
    private external fun nativeIsModelReady(handle: Long): Boolean
    private external fun nativeExportModel(handle: Long): ByteArray?
    private external fun nativeImportModel(handle: Long, bytes: ByteArray): Boolean

    companion object {
        init {
            System.loadLibrary("behavior_guard")
        }
    }
}

// ── Outcome types ─────────────────────────────────────────────────────────────

sealed class SessionOutcome {
    /** Enrollment in progress. Collect [sessionsRemaining] more sessions. */
    data class Enrolling(val sessionsRemaining: Int) : SessionOutcome()

    /** Enrollment just completed. Scoring is now available. */
    object EnrollmentComplete : SessionOutcome()

    /**
     * A risk score was produced.
     * @param score      [0.0, 1.0] — 0.0 = matches baseline, 1.0 = anomalous.
     * @param confidence [0.0, 1.0] — how much signal contributed to the score.
     */
    data class Scored(val score: Float, val confidence: Float) : SessionOutcome()

    /** Session error (insufficient events, no session active, etc.). */
    data class Error(val message: String) : SessionOutcome()
}
