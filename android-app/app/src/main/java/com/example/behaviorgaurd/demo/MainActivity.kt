package com.example.behaviorgaurd.demo

import android.content.Context
import android.hardware.SensorManager
import android.os.Bundle
import android.text.Editable
import android.text.TextWatcher
import android.util.Log
import android.view.MotionEvent
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.ProgressBar
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.behaviorgaurd.BehaviorGuardManager
import com.behaviorgaurd.SessionOutcome

// ══════════════════════════════════════════════════════════════════════════════
//  BehaviorGuard Demo — MainActivity
//
//  Demonstrates the BehaviorGuardManager integration lifecycle:
//
//  ┌─────────────────────────────────────────────────────────────────────────┐
//  │  1. Create BehaviorGuardManager (one per user, long-lived).             │
//  │     • Automatically restores profile + model from encrypted storage.    │
//  │  2. On session start: manager.startSession(sensorManager)               │
//  │     • Registers gyroscope + linear acceleration sensors.                │
//  │  3. During session: forward events to manager.                          │
//  │     • Touch  → dispatchTouchEvent() → manager.onTouchEvent()            │
//  │     • Keys   → TextWatcher → manager.onKeystroke()                      │
//  │  4. On session end: manager.endSession(sensorManager) → SessionOutcome  │
//  │     • Unregisters sensors.                                               │
//  │     • Persists profile + model automatically.                           │
//  └─────────────────────────────────────────────────────────────────────────┘
//
//  RISK SCORE INTERPRETATION
//  ─────────────────────────
//  Score 0.0 – 0.3 → matches baseline → allow
//  Score 0.3 – 0.6 → moderate deviation → log / monitor
//  Score 0.6 – 0.7 → significant deviation → soft challenge
//  Score 0.7 – 1.0 → high anomaly → step-up auth or block
//
// ══════════════════════════════════════════════════════════════════════════════

private const val TAG             = "BehaviorGuardDemo"
private const val RISK_THRESHOLD  = 0.7f
private const val PREFS_NAME      = "behavior_guard_prefs"

class MainActivity : AppCompatActivity() {

    // BehaviorGuardManager: wraps BehaviorGuard with Keystore key management,
    // automatic profile + model persistence, and sensor lifecycle.
    private lateinit var manager: BehaviorGuardManager

    private val sensorManager by lazy { getSystemService(SensorManager::class.java) }

    private var sessionActive = false
    private var lastKeyUpMs   = -1L

    // ── Views ─────────────────────────────────────────────────────────────────
    private lateinit var tvEnrollStatus:  TextView
    private lateinit var progressEnroll:  ProgressBar
    private lateinit var layoutScore:     View
    private lateinit var tvScore:         TextView
    private lateinit var tvConfidence:    TextView
    private lateinit var etInput:         EditText
    private lateinit var btnStartSession: Button
    private lateinit var btnEndSession:   Button
    private lateinit var btnClearProfile: Button
    private lateinit var tvStatus:        TextView
    private lateinit var scrollStatus:    ScrollView

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        bindViews()

        // BehaviorGuardManager restores existing profile + model on construction.
        manager = BehaviorGuardManager(this)

        wireButtons()
        wireKeystrokeCapture()
        updateEnrollmentUi()
        log("Ready. Press Start Session to begin.")
        log("Signals collected: keystroke timing, touch pressure/area, swipe velocity, gyro/accel.")
        if (manager.isModelReady()) log("Phase 2 autoencoder active.")
        else if (manager.isEnrolled()) log("Phase 1 z-score active (model not restored — run one session to rebuild).")
    }

    override fun onDestroy() {
        super.onDestroy()
        manager.close()
    }

    // ── Touch event forwarding ────────────────────────────────────────────────

    override fun dispatchTouchEvent(ev: MotionEvent): Boolean {
        if (sessionActive) {
            val w = window.decorView.width.takeIf  { it > 0 } ?: 1
            val h = window.decorView.height.takeIf { it > 0 } ?: 1
            manager.onTouchEvent(ev, w, h)
        }
        return super.dispatchTouchEvent(ev)
    }

    // ── Session controls ──────────────────────────────────────────────────────

    private fun startSession() {
        manager.startSession(sensorManager)
        sessionActive = true
        lastKeyUpMs   = -1L
        etInput.text?.clear()
        btnStartSession.isEnabled = false
        btnEndSession.isEnabled   = true
        log("── Session started ──")
        log("Type in the text field and interact with the screen.")
    }

    private fun endSession() {
        sessionActive = false
        btnStartSession.isEnabled = true
        btnEndSession.isEnabled   = false

        when (val outcome = manager.endSession(sensorManager)) {

            is SessionOutcome.Enrolling -> {
                log("── Session ended (enrolling) ──")
                log("Sessions remaining: ${outcome.sessionsRemaining}")
                updateEnrollmentUi()
            }

            is SessionOutcome.EnrollmentComplete -> {
                log("── Enrollment complete ──")
                log("Baseline + Phase 2 autoencoder trained. Scoring is now active.")
                updateEnrollmentUi()
            }

            is SessionOutcome.Scored -> {
                val pct  = (outcome.score      * 100).toInt()
                val conf = (outcome.confidence * 100).toInt()
                log("── Session scored ──")
                log("Risk score:  $pct%  (0% = matches baseline, 100% = anomalous)")
                log("Confidence:  $conf%")
                log("Scorer: ${if (manager.isModelReady()) "Phase 2 autoencoder" else "Phase 1 z-score"}")
                when {
                    outcome.score < 0.3f -> log("Decision: ALLOW")
                    outcome.score < 0.6f -> log("Decision: MONITOR")
                    outcome.score < 0.7f -> log("Decision: SOFT CHALLENGE")
                    else -> {
                        log("Decision: STEP-UP AUTH (score ${pct}% ≥ ${(RISK_THRESHOLD*100).toInt()}%)")
                    }
                }
                showScore(outcome.score, outcome.confidence)
            }

            is SessionOutcome.Error -> {
                log("Session error: ${outcome.message}")
                log("Tip: type more text or interact longer before ending a session.")
            }
        }
    }

    // ── Keystroke capture ─────────────────────────────────────────────────────

    private fun wireKeystrokeCapture() {
        etInput.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence, start: Int, count: Int, after: Int) {}
            override fun onTextChanged(s: CharSequence, start: Int, before: Int, count: Int) {}
            override fun afterTextChanged(s: Editable) {
                if (!sessionActive) return
                val now         = System.currentTimeMillis()
                val isCorrection = s.length < (etInput.text?.length ?: 0) + 1
                val flight      = if (lastKeyUpMs < 0) -1L else now - lastKeyUpMs
                // Approximate dwell: 80 ms is typical for a soft-keyboard tap.
                val dwell = 80L
                manager.onKeystroke(now, now + dwell, flight, isCorrection)
                lastKeyUpMs = now + dwell
            }
        })
    }

    // ── UI helpers ────────────────────────────────────────────────────────────

    private fun updateEnrollmentUi() {
        if (manager.isEnrolled()) {
            tvEnrollStatus.text = "Enrolled — scoring active"
            progressEnroll.progress = 5
            layoutScore.visibility = View.VISIBLE
        } else {
            tvEnrollStatus.text = "Enrollment in progress — complete 5 sessions"
        }
    }

    private fun showScore(score: Float, confidence: Float) {
        layoutScore.visibility = View.VISIBLE
        tvScore.text = "${(score * 100).toInt()}%"
        tvScore.setTextColor(
            when {
                score < 0.3f -> 0xFF388E3C.toInt()
                score < 0.6f -> 0xFFF57C00.toInt()
                else         -> 0xFFD32F2F.toInt()
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
            manager.reset()
            log("Profile cleared. Restart the app to re-enroll.")
        }
    }
}
