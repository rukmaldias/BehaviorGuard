---
sidebar_position: 7
---

# Glossary

Quick definitions for every term used across the BehaviorGuard documentation.

---

### Autoencoder

A type of neural network that learns to compress its input into a smaller representation (the **bottleneck**) and then reconstruct the original. BehaviorGuard uses a 32→16→8→16→32 architecture. After training on a user's enrollment data, the autoencoder can reconstruct that user's feature vectors accurately but fails to reconstruct an impostor's, producing high **reconstruction error** and therefore a high risk score.

See [Architecture → Phase 2 scorer](architecture.md#phase-2-scorer--autoencoder).

---

### Baseline profile

The statistical summary of a user's behaviour built from their 5 **enrollment** sessions. Stored as 64 numbers: one mean and one standard deviation for each of the 32 **features**. Used by both the Phase 1 z-score scorer and as the normalization reference for the autoencoder.

---

### Behavioral biometrics

Measurements of *how* a person interacts with a device — typing rhythm, touch pressure, swipe speed, device handling — as opposed to *who* they physically are (fingerprint, face). Behavioral patterns are difficult to consciously imitate because they emerge from ingrained motor habits.

---

### Bottleneck

The middle layer of the autoencoder, with only 8 values. The network must compress 32 feature values into 8 and back out to 32. This forces the network to learn the most important relationships between features, making it sensitive to impostors who don't share those relationships.

---

### Confidence

A `Float` between 0.0 and 1.0 returned alongside the **risk score**. Reflects how much signal was present in the session and how mature the enrollment baseline is. Low confidence means the score is less reliable and you should apply a more lenient **risk threshold**.

Formula: `min(events / 50, 1.0) × min(session_count / 10, 1.0)`

---

### Enrollment

The learning phase before BehaviorGuard can produce scores. Requires 5 **sessions**. During enrollment, each session returns `SessionOutcome.Enrolling`. After the 5th session, a **baseline profile** and **autoencoder** model are built automatically. Enrollment is permanent until `reset()` is called.

---

### Feature

A single statistical measurement extracted from a session's raw events — for example, "mean keystroke dwell time" or "standard deviation of touch pressure." BehaviorGuard extracts 32 features per session, producing a **feature vector**.

---

### Feature vector

The 32-element array of `f32` values that summarises one session's interaction. Features cover keystroke timing, touch characteristics, swipe dynamics, and IMU motion. The feature vector is the unit of input for both the baseline profile and the autoencoder.

---

### Flight time

The time between releasing one key and pressing the next. Together with **dwell time**, it characterises typing rhythm. Flight time is one of the strongest behavioral signals — people with similar typing speed can have very different flight time distributions.

---

### Dwell time

The duration a key is held down (from key-down to key-up event). Measured in milliseconds. A fast typist might have a dwell time of 50–80 ms; a careful typist 100–150 ms.

---

### IMU

Inertial Measurement Unit — the gyroscope and accelerometer sensors in a mobile device. BehaviorGuard reads both at ~50 Hz during a session to capture how the user holds and moves the device while interacting. IMU data complements keystroke and touch signals and is especially useful for detecting bots (which produce no IMU signal) or large environmental changes (device on a table vs. held in hand).

---

### Impostor

A person other than the enrolled user interacting with the device. The goal of BehaviorGuard is to detect impostors by observing that their behavioral signals deviate from the enrolled user's baseline.

---

### Phase 1 scorer

The simpler of the two scorers. Computes the mean absolute **z-score** across all 32 features and maps it to a risk score. Used as a fallback when the autoencoder model is not available. Treats features independently, which is its main limitation.

---

### Phase 2 scorer

The default scorer after enrollment. Uses the **autoencoder's** reconstruction error as the risk score. Captures joint relationships between features that the Phase 1 scorer misses.

---

### Reconstruction error

The mean squared difference between the autoencoder's input (the z-normalised feature vector) and its output (the reconstruction). Low for the enrolled user; high for impostors. Mapped to the range [0, 1] to produce the **risk score**.

---

### Risk score

A `Float` between 0.0 and 1.0 representing how anomalous the current session's behaviour is relative to the enrolled baseline. 0.0 = matches the baseline exactly; 1.0 = maximally anomalous. Used to make an authentication policy decision (allow / challenge / block).

---

### Risk threshold

The score value above which your app takes action. Not a fixed number — it should be tuned for your app's security requirements and adjusted downward when **confidence** is low. Typical starting values: 0.7 for step-up auth, 0.9 for hard block.

---

### Session

One interaction period, bounded by `startSession()` and `endSession()`. Can correspond to filling a form, completing a flow, or any interaction period your app defines. Must contain at least 5 keystrokes **or** 3 touch events to produce a result; shorter sessions return `SessionOutcome.Error`.

---

### Signal

A category of raw events collected during a session. BehaviorGuard collects four signal types: keystroke events, touch events, swipe events, and motion events (IMU). Each signal type contributes a subset of the 32 **features**.

---

### Z-normalization

The process of converting a raw feature value into a z-score: `(value - mean) / std`. Makes features from different scales comparable. BehaviorGuard clamps z-normalised values to ±8 to prevent numerical blow-up when the enrolled standard deviation is very small.

---

### Z-score

The number of standard deviations a value is from the mean. A z-score of 0 means "exactly average." A z-score of 2 means "two standard deviations above average." Used in the **Phase 1 scorer** to measure how far a session's features are from the enrolled baseline.
