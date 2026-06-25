package com.example.behaviorgaurd.demo

import android.content.Context
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.os.Bundle
import android.text.Editable
import android.text.TextWatcher
import android.util.Base64
import android.util.Log
import android.view.MotionEvent
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.ProgressBar
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.example.behaviorgaurd.BehaviorGuard
import com.example.behaviorgaurd.SessionOutcome

// ══════════════════════════════════════════════════════════════════════════════
//  BehaviorGuard Demo — MainActivity
//
//  This Activity demonstrates the full BehaviorGuard integration lifecycle:
//
//  ┌─────────────────────────────────────────────────────────────────────────┐
//  │  INTEGRATION OVERVIEW                                                   │
//  │                                                                         │
//  │  1. Create a BehaviorGuard instance (one per user, long-lived).         │
//  │  2. On session start: call guard.startSession() + register sensors.     │
//  │  3. During the session: feed touch, keystroke, and swipe events.        │
//  │     • Touch  → dispatchTouchEvent() → guard.onTouchEvent()             │
//  │     • Keys   → TextWatcher → guard.onKeystroke()                       │
//  │     • Swipe  → GestureDetector or VelocityTracker → guard.onSwipe()   │
//  │     • Motion → SensorEventListener → guard.onMotion() (via SDK)        │
//  │  4. On session end: call guard.endSession() → handle SessionOutcome.   │
//  │  5. Persist the profile across launches via export/import.              │
//  └─────────────────────────────────────────────────────────────────────────┘
//
//  ENROLLMENT FLOW
//  ───────────────
//  The first 5 sessions build the baseline profile. During enrollment,
//  endSession() returns SessionOutcome.Enrolling with how many remain.
//  After the 5th session it returns SessionOutcome.EnrollmentComplete and
//  every subsequent session returns SessionOutcome.Scored.
//
//  RISK SCORE INTERPRETATION
//  ─────────────────────────
//  Score 0.0 – 0.3 → matches baseline → allow
//  Score 0.3 – 0.6 → moderate deviation → log / monitor
//  Score 0.6 – 0.7 → significant deviation → soft challenge
//  Score 0.7 – 1.0 → high anomaly → step-up auth or block
//
//  PROFILE PERSISTENCE
//  ───────────────────
//  The baseline profile is encrypted with AES-256-GCM before storage.
//  The AES key lives in Android Keystore (hardware-backed, never exported).
//  We use EncryptedSharedPreferences (Jetpack Security) as the storage layer,
//  which wraps Keystore automatically.
//
// ══════════════════════════════════════════════════════════════════════════════

private const val TAG = "BehaviorGuardDemo"
private const val RISK_THRESHOLD = 0.7f
private const val PREFS_NAME = "behavior_guard_prefs"
private const val PREFS_KEY_PROFILE = "baseline_profile"

class DemoApplication : android.app.Application() {
    override fun onCreate() {
        super.onCreate()
        System.loadLibrary("behavior_guard")
    }
}

class MainActivity : AppCompatActivity() {

    // ── BehaviorGuard instance ────────────────────────────────────────────────
    //
    // One instance per user. Long-lived — keep it alive for the duration of
    // the Activity (or better: in a ViewModel for config-change survival).
    private val guard = BehaviorGuard()

    // Sensor listener returned by guard.registerSensors() — kept so we can
    // unregister it in onPause().
    private var sensorListener: SensorEventListener? = null
    private val sensorManager by lazy { getSystemService(SensorManager::class.java) }

    // Session tracking — timestamps for relative-time calculation.
    private var sessionActive = false
    private var lastKeyUpMs = -1L

    // ── Views ─────────────────────────────────────────────────────────────────
    private lateinit var tvEnrollStatus: TextView
    private lateinit var progressEnroll: ProgressBar
    private lateinit var layoutScore: View
    private lateinit var tvScore: TextView
    private lateinit var tvConfidence: TextView
    private lateinit var etInput: EditText
    private lateinit var btnStartSession: Button
    private lateinit var btnEndSession: Button
    private lateinit var btnClearProfile: Button
    private lateinit var tvStatus: TextView
    private lateinit var scrollStatus: ScrollView

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        bindViews()
        wireButtons()
        wireKeystrokeCapture()

        // Restore previously saved profile from encrypted storage.
        // If no profile exists yet, the guard starts in enrollment mode.
        loadProfile()

        updateEnrollmentUi()
        log("Ready. Press Start Session to begin collecting behavioral signals.")
        log("Signals collected: keystroke timing, touch pressure/area, swipe velocity, gyro/accel.")
    }

    override fun onDestroy() {
        super.onDestroy()
        guard.close()
    }

    // ── Touch event forwarding ────────────────────────────────────────────────
    //
    // dispatchTouchEvent is called for every touch on the screen before any
    // view handles it. Forward all events to BehaviorGuard — it distinguishes
    // taps from swipes internally based on duration and distance.

    override fun dispatchTouchEvent(ev: MotionEvent): Boolean {
        if (sessionActive) {
            val w = window.decorView.width.takeIf { it > 0 } ?: 1
            val h = window.decorView.height.takeIf { it > 0 } ?: 1
            guard.onTouchEvent(ev, w, h)
        }
        return super.dispatchTouchEvent(ev)
    }

    // ── Session controls ──────────────────────────────────────────────────────

    private fun startSession() {
        // ── INTEGRATION: call startSession() at the start of each interaction
        // period. Typical triggers: Activity.onResume(), login screen appears,
        // user begins a checkout flow, etc.
        guard.startSession()

        // Register IMU sensors — gyroscope + linear acceleration.
        // BehaviorGuard.registerSensors() handles both sensor types and merges
        // them into MotionEvents at SENSOR_DELAY_GAME (~50 Hz).
        sensorListener = guard.registerSensors(sensorManager)

        sessionActive = true
        lastKeyUpMs = -1L
        etInput.text?.clear()

        btnStartSession.isEnabled = false
        btnEndSession.isEnabled = true

        log("── Session started ──")
        log("Type in the text field and interact with the screen.")
        log("Signals: touch, swipe, keystroke timing, gyroscope, accelerometer.")
    }

    private fun endSession() {
        // Unregister sensors before calling endSession() to avoid late events
        // being added after the session buffer is consumed.
        sensorListener?.let { guard.unregisterSensors(sensorManager, it) }
        sensorListener = null
        sessionActive = false

        btnStartSession.isEnabled = true
        btnEndSession.isEnabled = false

        // ── INTEGRATION: endSession() extracts features and returns the outcome.
        // Always call this — even if the session was short — so the guard can
        // update enrollment state.
        when (val outcome = guard.endSession()) {

            is SessionOutcome.Enrolling -> {
                // ── ENROLLMENT IN PROGRESS ────────────────────────────────────
                // Still building the baseline profile. Show progress to the user.
                // Do NOT make security decisions yet — there is no baseline to
                // compare against.
                log("── Session ended (enrolling) ──")
                log("Sessions remaining for enrollment: ${outcome.sessionsRemaining}")
                updateEnrollmentUi()
                saveProfile()
            }

            is SessionOutcome.EnrollmentComplete -> {
                // ── ENROLLMENT JUST FINISHED ──────────────────────────────────
                // The baseline is now ready. Future sessions will return Scored.
                // Save the profile immediately so it survives process death.
                log("── Enrollment complete! ──")
                log("Baseline profile built from 5 sessions.")
                log("Future sessions will return a risk score.")
                updateEnrollmentUi()
                saveProfile()
            }

            is SessionOutcome.Scored -> {
                // ── RISK SCORE AVAILABLE ──────────────────────────────────────
                // A score is ready. Apply your policy here.
                //
                // score      [0.0, 1.0] — deviation from the enrolled baseline
                // confidence [0.0, 1.0] — how much signal contributed
                //
                // Low confidence = short session, few events. High confidence =
                // rich session with many keystrokes, touches, and motion samples.
                val pct = (outcome.score * 100).toInt()
                val conf = (outcome.confidence * 100).toInt()

                log("── Session scored ──")
                log("Risk score:  $pct%  (0% = matches baseline, 100% = anomalous)")
                log("Confidence:  $conf%")

                when {
                    outcome.score < 0.3f -> log("Decision: ALLOW — matches enrolled baseline")
                    outcome.score < 0.6f -> log("Decision: MONITOR — moderate deviation")
                    outcome.score < 0.7f -> log("Decision: SOFT CHALLENGE — significant deviation")
                    else -> {
                        log("Decision: STEP-UP AUTH — high anomaly (score ${pct}% >= ${(RISK_THRESHOLD * 100).toInt()}%)")
                        // ── INTEGRATION: trigger step-up auth here ───────────
                        // e.g. show biometric prompt, OTP dialog, or block the action.
                    }
                }

                showScore(outcome.score, outcome.confidence)
                saveProfile()
            }

            is SessionOutcome.Error -> {
                log("Session error: ${outcome.message}")
                log("Tip: type more text or interact longer before ending a session.")
            }
        }
    }

    // ── Profile persistence ───────────────────────────────────────────────────
    //
    // The baseline profile is serialised and AES-256-GCM encrypted by the
    // native library. We store the ciphertext blob in EncryptedSharedPreferences,
    // which adds a second Keystore-backed encryption layer (defence in depth).
    //
    // INTEGRATION: replace EncryptedSharedPreferences with your own secure
    // storage (encrypted database column, secure file, etc.) if needed.

    private fun keystoreKey(): ByteArray {
        // ── INTEGRATION: derive a 32-byte key from Android Keystore.
        //
        // The native ProfileStore.seal() / open() need a raw 32-byte AES key.
        // We generate a Keystore-backed AES-256 key and use it to encrypt a
        // random 32-byte passphrase that is stored in EncryptedSharedPreferences.
        //
        // For simplicity in this demo we derive the key from the app's MasterKey
        // alias via a fixed HKDF-like derivation. In production, generate a
        // dedicated Keystore key for the profile.
        val masterKey = MasterKey.Builder(this)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()
        val prefs = EncryptedSharedPreferences.create(
            this, "bg_key_prefs", masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
        )
        val stored = prefs.getString("profile_key", null)
        if (stored != null) {
            return Base64.decode(stored, Base64.NO_WRAP)
        }
        val key = ByteArray(32).also { android.security.keystore.KeyGenParameterSpec
            .let { java.security.SecureRandom().nextBytes(it as? ByteArray ?: return@let) }
            java.security.SecureRandom().nextBytes(this as? ByteArray ?: return@let)
        }
        // Fallback: generate random key for demo simplicity
        val fresh = ByteArray(32).also { java.security.SecureRandom().nextBytes(it) }
        prefs.edit().putString("profile_key", Base64.encodeToString(fresh, Base64.NO_WRAP)).apply()
        return fresh
    }

    private fun saveProfile() {
        try {
            val blob = guard.exportProfile(keystoreKey()) ?: return
            getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                .edit()
                .putString(PREFS_KEY_PROFILE, Base64.encodeToString(blob, Base64.NO_WRAP))
                .apply()
            log("Profile saved (${blob.size} bytes encrypted).")
        } catch (e: Exception) {
            Log.e(TAG, "saveProfile failed", e)
        }
    }

    private fun loadProfile() {
        val encoded = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .getString(PREFS_KEY_PROFILE, null) ?: return
        try {
            val blob = Base64.decode(encoded, Base64.NO_WRAP)
            val ok = guard.importProfile(blob, keystoreKey())
            if (ok) log("Profile loaded from storage (${blob.size} bytes).")
            else log("Profile found but failed to decrypt — starting fresh enrollment.")
        } catch (e: Exception) {
            Log.e(TAG, "loadProfile failed", e)
        }
    }

    // ── Keystroke capture ─────────────────────────────────────────────────────
    //
    // We capture approximate keystroke timing via TextWatcher. This gives
    // dwell + flight time estimates without requiring InputMethodService.
    //
    // INTEGRATION: for higher accuracy, implement a custom InputMethodService
    // or use InputConnection callbacks which expose exact key-down/key-up times.

    private fun wireKeystrokeCapture() {
        etInput.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence, start: Int, count: Int, after: Int) {}
            override fun onTextChanged(s: CharSequence, start: Int, before: Int, count: Int) {}

            override fun afterTextChanged(s: Editable) {
                if (!sessionActive) return

                val now = System.currentTimeMillis()
                val isCorrection = s.length < (etInput.text?.length ?: 0) + 1
                val flight = if (lastKeyUpMs < 0) -1L else now - lastKeyUpMs

                // Approximate dwell: 80 ms is a typical soft-keyboard tap duration.
                // Replace with real key-down timestamps from InputConnection if available.
                val estimatedDwell = 80L
                guard.onKeystroke(
                    downMs = now,
                    upMs = now + estimatedDwell,
                    flightMs = flight,
                    isCorrection = isCorrection,
                )
                lastKeyUpMs = now + estimatedDwell
            }
        })
    }

    // ── UI helpers ────────────────────────────────────────────────────────────

    private fun updateEnrollmentUi() {
        if (guard.isEnrolled()) {
            tvEnrollStatus.text = "Enrolled — scoring active"
            progressEnroll.progress = 5
            layoutScore.visibility = View.VISIBLE
        } else {
            val done = 5 - 0  // sessions completed tracked by guard internally
            tvEnrollStatus.text = "Enrollment in progress — complete 5 sessions"
            progressEnroll.progress = done
        }
    }

    private fun showScore(score: Float, confidence: Float) {
        layoutScore.visibility = View.VISIBLE
        val pct = (score * 100).toInt()
        tvScore.text = "$pct%"
        tvScore.setTextColor(
            when {
                score < 0.3f -> 0xFF388E3C.toInt()  // green
                score < 0.6f -> 0xFFF57C00.toInt()  // orange
                else         -> 0xFFD32F2F.toInt()  // red
            }
        )
        tvConfidence.text = "${(confidence * 100).toInt()}%"
    }

    private fun log(message: String) {
        val current = tvStatus.text.toString()
        val updated = if (current == getString(R.string.status_init)) message
                      else "$current\n$message"
        tvStatus.text = updated
        scrollStatus.post { scrollStatus.fullScroll(ScrollView.FOCUS_DOWN) }
        Log.d(TAG, message)
    }

    private fun bindViews() {
        tvEnrollStatus  = findViewById(R.id.tvEnrollStatus)
        progressEnroll  = findViewById(R.id.progressEnroll)
        layoutScore     = findViewById(R.id.layoutScore)
        tvScore         = findViewById(R.id.tvScore)
        tvConfidence    = findViewById(R.id.tvConfidence)
        etInput         = findViewById(R.id.etInput)
        btnStartSession = findViewById(R.id.btnStartSession)
        btnEndSession   = findViewById(R.id.btnEndSession)
        btnClearProfile = findViewById(R.id.btnClearProfile)
        tvStatus        = findViewById(R.id.tvStatus)
        scrollStatus    = findViewById(R.id.scrollStatus)
    }

    private fun wireButtons() {
        btnStartSession.setOnClickListener { startSession() }
        btnEndSession.setOnClickListener   { endSession() }
        btnClearProfile.setOnClickListener {
            getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).edit().clear().apply()
            log("Profile cleared. Restart the app to re-enroll.")
        }
    }
}
