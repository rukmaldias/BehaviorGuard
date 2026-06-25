use crate::signals::{KeystrokeEvent, MotionEvent, RawEvent, SwipeEvent, TouchEvent};
use super::{stats, FEATURE_DIM};

/// A fixed-length feature vector extracted from one session's raw events.
///
/// Layout (32 features):
///   [0..5]   keystroke: dwell mean/std, flight mean/std, correction_rate, count_norm
///   [6..11]  touch: duration mean/std, pressure mean/std, area mean/std
///   [12..17] swipe: distance mean/std, avg_vel mean/std, peak_vel mean/std
///   [18..23] motion/gyro: x mean/std, y mean/std, z mean/std
///   [24..29] motion/accel: x mean/std, y mean/std, z mean/std
///   [30]     gyro magnitude mean
///   [31]     accel magnitude mean
#[derive(Debug, Clone)]
pub struct FeatureVector(pub [f32; FEATURE_DIM]);

impl FeatureVector {
    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }
}

/// Extracts a `FeatureVector` from a slice of raw events.
///
/// Returns `None` if there are not enough events to compute meaningful features
/// (fewer than 5 keystrokes or 3 touch events).
pub fn extract(events: &[RawEvent]) -> Option<FeatureVector> {
    let keystrokes: Vec<&KeystrokeEvent> = events
        .iter()
        .filter_map(|e| if let RawEvent::Keystroke(k) = e { Some(k) } else { None })
        .collect();
    let touches: Vec<&TouchEvent> = events
        .iter()
        .filter_map(|e| if let RawEvent::Touch(t) = e { Some(t) } else { None })
        .collect();
    let swipes: Vec<&SwipeEvent> = events
        .iter()
        .filter_map(|e| if let RawEvent::Swipe(s) = e { Some(s) } else { None })
        .collect();
    let motions: Vec<&MotionEvent> = events
        .iter()
        .filter_map(|e| if let RawEvent::Motion(m) = e { Some(m) } else { None })
        .collect();

    if keystrokes.len() < 5 && touches.len() < 3 {
        return None;
    }

    let mut f = [0.0f32; FEATURE_DIM];

    // ── Keystroke features [0..5] ────────────────────────────────────────────
    if !keystrokes.is_empty() {
        let dwells: Vec<f32> = keystrokes.iter().map(|k| k.dwell_ms() as f32).collect();
        let (dm, ds) = stats::mean_std(&dwells);
        f[0] = dm;
        f[1] = ds;

        let flights: Vec<f32> = keystrokes
            .iter()
            .filter_map(|k| k.flight_ms.map(|v| v as f32))
            .collect();
        let (fm, fs) = stats::mean_std(&flights);
        f[2] = fm;
        f[3] = fs;

        let corrections = keystrokes.iter().filter(|k| k.is_correction).count();
        f[4] = corrections as f32 / keystrokes.len() as f32;
        // Normalise count to [0,1] assuming sessions rarely exceed 200 keystrokes
        f[5] = (keystrokes.len() as f32 / 200.0).min(1.0);
    }

    // ── Touch features [6..11] ───────────────────────────────────────────────
    if !touches.is_empty() {
        let durations: Vec<f32> = touches.iter().map(|t| t.duration_ms() as f32).collect();
        let (dm, ds) = stats::mean_std(&durations);
        f[6] = dm;
        f[7] = ds;

        let pressures: Vec<f32> = touches.iter().map(|t| t.pressure).collect();
        let (pm, ps) = stats::mean_std(&pressures);
        f[8] = pm;
        f[9] = ps;

        let areas: Vec<f32> = touches.iter().map(|t| t.area).collect();
        let (am, as_) = stats::mean_std(&areas);
        f[10] = am;
        f[11] = as_;
    }

    // ── Swipe features [12..17] ──────────────────────────────────────────────
    if !swipes.is_empty() {
        let distances: Vec<f32> = swipes.iter().map(|s| s.distance()).collect();
        let (dm, ds) = stats::mean_std(&distances);
        f[12] = dm;
        f[13] = ds;

        let avg_vels: Vec<f32> = swipes.iter().map(|s| s.avg_velocity()).collect();
        let (avm, avs) = stats::mean_std(&avg_vels);
        f[14] = avm;
        f[15] = avs;

        let peak_vels: Vec<f32> = swipes.iter().map(|s| s.peak_velocity).collect();
        let (pvm, pvs) = stats::mean_std(&peak_vels);
        f[16] = pvm;
        f[17] = pvs;
    }

    // ── Motion / gyroscope features [18..23] ────────────────────────────────
    if !motions.is_empty() {
        let gx: Vec<f32> = motions.iter().map(|m| m.gyro_x).collect();
        let gy: Vec<f32> = motions.iter().map(|m| m.gyro_y).collect();
        let gz: Vec<f32> = motions.iter().map(|m| m.gyro_z).collect();
        let (gxm, gxs) = stats::mean_std(&gx);
        let (gym, gys) = stats::mean_std(&gy);
        let (gzm, gzs) = stats::mean_std(&gz);
        f[18] = gxm; f[19] = gxs;
        f[20] = gym; f[21] = gys;
        f[22] = gzm; f[23] = gzs;

        // ── Motion / accelerometer features [24..29] ────────────────────────
        let ax: Vec<f32> = motions.iter().map(|m| m.accel_x).collect();
        let ay: Vec<f32> = motions.iter().map(|m| m.accel_y).collect();
        let az: Vec<f32> = motions.iter().map(|m| m.accel_z).collect();
        let (axm, axs) = stats::mean_std(&ax);
        let (aym, ays) = stats::mean_std(&ay);
        let (azm, azs) = stats::mean_std(&az);
        f[24] = axm; f[25] = axs;
        f[26] = aym; f[27] = ays;
        f[28] = azm; f[29] = azs;

        let gmags: Vec<f32> = motions.iter().map(|m| m.gyro_magnitude()).collect();
        f[30] = stats::mean_std(&gmags).0;

        let amags: Vec<f32> = motions.iter().map(|m| m.accel_magnitude()).collect();
        f[31] = stats::mean_std(&amags).0;
    }

    Some(FeatureVector(f))
}
