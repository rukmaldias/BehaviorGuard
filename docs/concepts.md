---
sidebar_position: 2
---

# Core Concepts

This page explains the ideas behind BehaviorGuard in plain language — no prior knowledge of machine learning or biometrics required.

---

## What is behavioral biometrics?

Every person interacts with a device in a subtly unique way:

- A fast typist consistently taps keys for ~60 ms and releases them. A slow typist holds them for ~120 ms.
- An anxious user makes more corrections (backspace) per sentence than a calm one.
- A large-handed user applies more pressure and covers more screen area per touch than a small-handed one.
- Someone holding their phone while walking introduces gyroscope noise that someone sitting still does not.

These patterns are **behavioral biometrics** — measurable characteristics of how you interact with a device, rather than who you physically are (fingerprint, face). They are difficult to consciously control because they emerge from motor habits built up over years of typing and phone use.

BehaviorGuard captures these patterns, learns what is normal for a specific user, and flags when subsequent behaviour diverges from that baseline.

---

## What is a session?

A **session** is one interaction period — the time between calling `startSession()` and `endSession()`. It could be:

- Filling in a login form
- Completing a checkout flow
- Writing a message
- Any other screen where the user types or touches

A session must contain at least 5 keystrokes **or** 3 touch events to produce enough signal. Shorter interactions return `SessionOutcome.Error` and are silently skipped — this is intentional.

---

## What is enrollment?

Before BehaviorGuard can score a user, it needs to learn what that user's normal behaviour looks like. This learning phase is called **enrollment**.

BehaviorGuard requires **5 sessions** to build the baseline. During enrollment, each session returns `SessionOutcome.Enrolling` with a count of how many more sessions are needed. No risk score is available yet.

After the 5th session, enrollment is complete. Two things happen automatically:

1. A **baseline profile** is saved — the per-feature mean and standard deviation across all 5 sessions.
2. An **autoencoder model** is trained on those sessions — a small neural network that learns the joint pattern of all 32 features together.

From this point on, every session returns `SessionOutcome.Scored`.

---

## What is a feature vector?

Raw events (keystrokes, touches, swipes, motion samples) are too noisy and variable in count to compare directly. Instead, BehaviorGuard distills each session into a **feature vector**: 32 numbers that summarise the statistical shape of the interaction.

Examples:
- Feature 0: average keystroke dwell time (how long keys are held down)
- Feature 8: average touch pressure
- Feature 18: average gyroscope rotation on the X axis

These 32 numbers describe *what the session looked like on average*, stripping away the specific words typed or buttons tapped. Two sessions where you type the same things will produce similar feature vectors. A session where someone else types will produce a different one.

---

## How does scoring work?

BehaviorGuard has two scoring modes that work together.

### Phase 1 — Z-score (fallback)

The simplest approach: for each of the 32 features, measure how many standard deviations the current session is from the enrolled average. Average those 28 distances. The further away, the higher the risk score.

This works well but has a blind spot: it treats each feature independently. A real impostor might accidentally match your dwell time while completely missing your touch pressure — and the independent averaging would partially cancel those signals out.

### Phase 2 — Autoencoder (default)

An **autoencoder** is a small neural network trained to compress and reconstruct its input. BehaviorGuard trains one on your enrollment feature vectors.

The key insight: the autoencoder learns not just your average values, but the *relationships between features*. A fast typist doesn't just have short dwell times in isolation — they also have short flight times, more events per session, and a particular rhythm. The autoencoder encodes all of these dependencies into an 8-number bottleneck.

At scoring time:
1. The session's feature vector is fed into the autoencoder.
2. The autoencoder tries to reconstruct it through the bottleneck.
3. If you are the enrolled user, reconstruction is accurate (low error).
4. If you are an impostor, the autoencoder tries to map your pattern through a bottleneck shaped for someone else — reconstruction error is high.
5. Reconstruction error → risk score (0.0 = low error = matches baseline, 1.0 = high error = anomalous).

The autoencoder is trained **on-device** at the end of the 5th enrollment session. Training takes under one second. The weights (~5 KB) are saved to encrypted storage so they only need to be trained once.

---

## What does the risk score mean?

The risk score is a single `Float` between 0.0 and 1.0:

```
0.0 ──────────────── 0.3 ──────── 0.6 ──── 0.7 ──────── 1.0
│   matches baseline  │  moderate  │ deviant │   anomaly  │
```

It is **not a probability** — it does not mean "70% chance this is an impostor." It is a distance measure: how far this session's behaviour is from the enrolled baseline.

The score should be used to make a **policy decision**, not a definitive identity determination:

- Below 0.3: normal — let the interaction continue
- 0.3–0.6: worth logging — no action required in most apps
- 0.6–0.7: notable deviation — consider a soft challenge (re-enter PIN)
- Above 0.7: strong anomaly — step-up authentication or block

The exact thresholds depend on your app's risk tolerance. A banking app should be more aggressive; a notes app might not need to challenge at all.

---

## What is confidence?

The `confidence` value returned alongside the score tells you how much signal contributed to that score. It accounts for two factors:

1. **Session length** — a 200-keystroke session provides much richer signal than a 5-keystroke one
2. **Enrollment maturity** — a user who enrolled 50 sessions ago has a more stable baseline than one who just finished their 5th

A low-confidence score is less reliable. The recommended approach is to be more lenient (raise your threshold) when confidence is low:

```kotlin
val threshold = 0.7f + (1f - outcome.confidence) * 0.2f
if (outcome.score > threshold) triggerStepUpAuth()
```

This prevents penalising users for short but legitimate interactions.

---

## What data is stored?

BehaviorGuard stores two things after enrollment:

1. **Baseline profile** — 32 mean values and 32 standard deviations. 64 numbers. This is enough to reconstruct how you interact with the device statistically, but cannot be reversed into your actual keystrokes or words typed.
2. **Autoencoder weights** — ~1,352 numbers describing the neural network. These encode correlations between your features but contain no raw interaction data.

Both are encrypted with AES-256-GCM before being written to disk. The encryption key lives in Android Keystore and never leaves the device.

Raw events (the actual keystrokes, touch coordinates, motion samples) are held only in memory during a session and discarded immediately after the feature vector is extracted. They are never stored to disk.

---

## How is this different from a password or fingerprint?

| Property | Password | Fingerprint | BehaviorGuard |
|---|---|---|---|
| Can be stolen | Yes (phishing, breach) | Partially (template theft) | No (model is not the interaction) |
| Can be forgotten | Yes | No | No |
| Requires explicit user action | Yes | Yes | No — passive |
| Detects mid-session takeover | No | No | Yes |
| Changes over time | No | Rarely | Slowly (natural drift) |
| Works offline | Yes | Yes | Yes |

BehaviorGuard is not a replacement for passwords or biometrics — it is a continuous, passive layer that catches attacks that slip past the primary authentication gate.
