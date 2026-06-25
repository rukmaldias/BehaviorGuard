package com.behaviorgaurd

import android.content.Context
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.util.Base64
import android.view.MotionEvent
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import java.security.SecureRandom

/**
 * High-level BehaviorGuard manager that handles:
 * - Keystore-backed AES key lifecycle
 * - Profile and autoencoder model persistence across app launches
 * - Sensor registration and teardown per session
 *
 * ## Usage
 * ```kotlin
 * // Instantiate once per user, long-lived (e.g. in a ViewModel).
 * val manager = BehaviorGuardManager(context)
 *
 * // When the user starts interacting:
 * manager.startSession(sensorManager)
 *
 * // Forward input events:
 * manager.onTouchEvent(event, screenWidth, screenHeight)
 * manager.onKeystroke(downMs, upMs, flightMs, isCorrection)
 *
 * // When the interaction period ends:
 * val outcome = manager.endSession(sensorManager)
 * when (outcome) {
 *     is SessionOutcome.Enrolling          -> showEnrollProgress(outcome.sessionsRemaining)
 *     is SessionOutcome.EnrollmentComplete -> showReady()
 *     is SessionOutcome.Scored             -> applyPolicy(outcome.score, outcome.confidence)
 *     is SessionOutcome.Error              -> log(outcome.message)
 * }
 * ```
 *
 * ## State persistence
 * Profile and autoencoder model are automatically saved after each session and
 * restored on construction.  The AES-256 encryption key lives in Android Keystore
 * (hardware-backed, never exported).
 *
 * ## Resetting enrollment
 * Call [reset] to wipe all saved state and start a fresh enrollment.
 * The current [BehaviorGuardManager] instance is no longer usable after reset —
 * create a new one.
 */
class BehaviorGuardManager(private val context: Context) : AutoCloseable {

    private val guard = BehaviorGuard()
    private var sensorListener: SensorEventListener? = null
    private var sessionStartMs = 0L

    init {
        restoreState()
    }

    // ── Session ───────────────────────────────────────────────────────────────

    /**
     * Starts a new collection session and registers IMU sensors.
     * Must be followed by [endSession] before calling again.
     */
    fun startSession(sensorManager: SensorManager) {
        sessionStartMs = System.currentTimeMillis()
        guard.startSession()
        sensorListener = guard.registerSensors(sensorManager)
    }

    /**
     * Ends the active session, unregisters sensors, extracts features,
     * and returns a [SessionOutcome].
     *
     * State is persisted automatically — no need to call [saveState] manually.
     */
    fun endSession(sensorManager: SensorManager): SessionOutcome {
        sensorListener?.let { guard.unregisterSensors(sensorManager, it) }
        sensorListener = null
        val outcome = guard.endSession()
        saveState()
        return outcome
    }

    // ── Event forwarding ──────────────────────────────────────────────────────

    /**
     * Forward a [MotionEvent] from your Activity's `dispatchTouchEvent`.
     * Call this for every event — the SDK separates taps from swipes internally.
     */
    fun onTouchEvent(event: MotionEvent, screenWidth: Int, screenHeight: Int) =
        guard.onTouchEvent(event, screenWidth, screenHeight)

    /**
     * Forward a keystroke. Use exact key-down and key-up timestamps for best
     * accuracy; -1 for [flightMs] on the first keystroke of a session.
     */
    fun onKeystroke(downMs: Long, upMs: Long, flightMs: Long, isCorrection: Boolean) =
        guard.onKeystroke(downMs, upMs, flightMs, isCorrection)

    /**
     * Forward a completed swipe gesture (call on ACTION_UP after tracking
     * start/end positions and peak velocity).
     */
    fun onSwipe(
        startMs: Long, endMs: Long,
        startX: Float, startY: Float,
        endX: Float, endY: Float,
        peakVelocity: Float,
    ) = guard.onSwipe(startMs, endMs, startX, startY, endX, endY, peakVelocity)

    // ── State queries ─────────────────────────────────────────────────────────

    /** Returns true once the 5-session enrollment baseline is complete. */
    fun isEnrolled(): Boolean = guard.isEnrolled()

    /** Returns true once the Phase 2 autoencoder model is trained and active. */
    fun isModelReady(): Boolean = guard.isModelReady()

    // ── State management ──────────────────────────────────────────────────────

    /**
     * Clears all persisted state (profile, model, Keystore key).
     * After calling this, do not reuse this instance — create a new one.
     */
    fun reset() {
        statePrefs().edit().clear().apply()
        runCatching {
            EncryptedSharedPreferences.create(
                context, PREFS_KEY_STORE,
                masterKey(),
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
            ).edit().clear().apply()
        }
    }

    override fun close() = guard.close()

    // ── Internal: persistence ─────────────────────────────────────────────────

    private fun saveState() {
        val key = keystoreKey()
        val prefs = statePrefs().edit()
        guard.exportProfile(key)?.let { blob ->
            prefs.putString(PREF_PROFILE, Base64.encodeToString(blob, Base64.NO_WRAP))
        }
        guard.exportModel()?.let { bytes ->
            prefs.putString(PREF_MODEL, Base64.encodeToString(bytes, Base64.NO_WRAP))
        }
        prefs.apply()
    }

    private fun restoreState() {
        val p = statePrefs()
        val profileEncoded = p.getString(PREF_PROFILE, null) ?: return
        val key = keystoreKey()
        val profileBlob = Base64.decode(profileEncoded, Base64.NO_WRAP)
        if (!guard.importProfile(profileBlob, key)) return
        val modelEncoded = p.getString(PREF_MODEL, null) ?: return
        guard.importModel(Base64.decode(modelEncoded, Base64.NO_WRAP))
    }

    // ── Internal: Keystore key ────────────────────────────────────────────────

    private fun masterKey() = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()

    private fun keystoreKey(): ByteArray {
        val encPrefs = EncryptedSharedPreferences.create(
            context, PREFS_KEY_STORE, masterKey(),
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
        )
        encPrefs.getString(KEY_PROFILE_KEY, null)?.let {
            return Base64.decode(it, Base64.NO_WRAP)
        }
        val fresh = ByteArray(32).also { SecureRandom().nextBytes(it) }
        encPrefs.edit()
            .putString(KEY_PROFILE_KEY, Base64.encodeToString(fresh, Base64.NO_WRAP))
            .apply()
        return fresh
    }

    private fun statePrefs() =
        context.getSharedPreferences(PREFS_BG_STATE, Context.MODE_PRIVATE)

    // ── Constants ─────────────────────────────────────────────────────────────

    private companion object {
        const val PREFS_BG_STATE  = "bg_state"
        const val PREFS_KEY_STORE = "bg_key_store"
        const val KEY_PROFILE_KEY = "profile_key"
        const val PREF_PROFILE    = "profile"
        const val PREF_MODEL      = "model"
    }
}
