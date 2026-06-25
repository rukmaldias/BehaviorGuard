---
sidebar_position: 5
---

# Threat Model

BehaviorGuard is a **continuous authentication signal**, not a primary authentication factor. Understanding what it does and does not protect against is essential before making security decisions based on its output.

---

## What BehaviorGuard protects against

### Account takeover after credential theft

An attacker who obtains a victim's password (phishing, credential stuffing, data breach) will not have the victim's typing rhythm, touch pressure profile, or device handling pattern. After a login, BehaviorGuard can detect that the user's subsequent behaviour does not match the enrolled baseline and trigger step-up authentication before a sensitive action completes.

**Effectiveness:** High, especially against remote attackers who have credentials but not physical access to the device.

### Session hijacking (post-authentication anomaly detection)

If an authenticated session is taken over mid-flow (e.g. by a malicious browser extension, a shoulder surfer, or a borrowed-device scenario), the new actor's behaviour will differ from the enrolled user's baseline. BehaviorGuard can flag this mid-session, not just at login.

**Effectiveness:** Moderate to high, depending on how different the interloper's typing pattern is.

### Automated bots and scripted attacks

Bots exhibit near-zero dwell time variance, uniform flight times, and no IMU signal. These patterns are far outside any human baseline and will produce maximum anomaly scores.

**Effectiveness:** Very high against unsophisticated bots; lower against bots that inject realistic human-like noise.

---

## What BehaviorGuard does NOT protect against

### Physical device theft with an active session

If an attacker has the unlocked device in hand and interacts with it naturally, BehaviorGuard will see a new user's biometric pattern and may score it as anomalous — but only after a complete session ends. A short interaction within an existing session will not be caught until `endSession` is called.

**Mitigation:** Call `endSession` frequently (e.g. on each screen transition or form submission, not just on `onPause`).

### Trained imitation

A determined attacker who has spent significant time studying and practising a victim's typing rhythm could reduce BehaviorGuard's detection rate. The autoencoder captures joint feature correlations that are hard to imitate consciously, but a highly motivated and well-resourced attacker may be able to pass at scores below 0.7.

**Mitigation:** Use BehaviorGuard as one layer of a defence-in-depth strategy, not as the sole authentication factor.

### Legitimate user behaviour change

Users' typing patterns change due to injury, illness, keyboard layout changes, or device type change (phone ↔ tablet). These legitimate changes will cause elevated risk scores until the profile is re-enrolled.

**Mitigation:** Implement a re-enrollment flow triggered after repeated false positives. Use the `confidence` field to be more lenient for short or signal-poor sessions.

### Rooted or compromised devices

On a rooted device, an attacker with system-level access could inject fake JNI calls, replace the `.so`, or read the native memory containing the profile. BehaviorGuard does not implement root detection — that is the responsibility of the host app. Consider integrating with Android Play Integrity API or a device attestation solution.

### Physical keyboard or external input

If the user types via a Bluetooth keyboard or voice input, the timing signals differ significantly from touch-screen typing. BehaviorGuard will score these sessions as anomalous even for legitimate users.

**Mitigation:** Detect external keyboard usage via `InputDevice.getSources()` and skip BehaviorGuard scoring for those sessions, or maintain separate profiles per input method.

---

## Data security properties

| Property | Guarantee |
|---|---|
| Raw events leave the device | Never — events are discarded after feature extraction |
| Feature vectors leave the device | Never — only the f32 risk score is returned to Kotlin |
| Profile leaves the device | Only if the app explicitly exports it (e.g. to sync across devices) |
| Autoencoder weights leave the device | Only on explicit export |
| Profile readable without the Keystore key | No — AES-256-GCM with authenticated encryption |
| Risk score is linkable to identity | No — it is a floating-point anomaly value with no user identifier |

---

## Confidence and false positive rate

The `confidence` field returned with each `RiskScore` indicates how much signal was present in the session. At low confidence:
- Fewer events were collected (short session)
- The score is less reliable

**Recommended practice:** Apply a more lenient threshold at low confidence rather than blocking the user on a high-risk score from a 3-keystroke session.

A calibrated threshold depends on your user population and risk appetite. For reference, the validation script (`scripts/validate_autoencoder.py`) reports Equal Error Rate (EER) across 50 synthetic users with the default hyperparameters. Production EER will differ based on session length, device type, and user demographics.

---

## Recommended deployment pattern

BehaviorGuard is most effective as one component of a layered security strategy:

```
Primary authentication      →  Password / PIN / biometric (FaceID, fingerprint)
                                       │
Continuous verification     →  BehaviorGuard (every session)
                                       │
                         ┌─────────────┴──────────────┐
                         │                            │
                   score < 0.3                  score ≥ 0.7
                   Allow silently           Step-up authentication
                                            (OTP, biometric re-prompt)
                                                       │
                                               Failed step-up?
                                               → Lock session
                                               → Notify user
```

Never block a user solely on a BehaviorGuard score without a step-up path. False positives are unavoidable — a blocked user with no recourse creates a significant UX and support burden.
