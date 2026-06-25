use approx::assert_abs_diff_eq;
use behavior_guard::{
    extract, FeatureVector, KeystrokeEvent, MotionEvent, RawEvent, SwipeEvent, TouchEvent,
    FEATURE_DIM,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn keystroke(down: u64, up: u64, flight: Option<u64>, correction: bool) -> RawEvent {
    RawEvent::Keystroke(KeystrokeEvent { down_ms: down, up_ms: up, flight_ms: flight, is_correction: correction })
}

fn touch(down: u64, up: u64, x: f32, y: f32, pressure: f32, area: f32) -> RawEvent {
    RawEvent::Touch(TouchEvent { down_ms: down, up_ms: up, x, y, pressure, area })
}

fn swipe(start: u64, end: u64, x0: f32, y0: f32, x1: f32, y1: f32, peak: f32) -> RawEvent {
    RawEvent::Swipe(SwipeEvent { start_ms: start, end_ms: end, start_x: x0, start_y: y0, end_x: x1, end_y: y1, peak_velocity: peak })
}

fn motion(ts: u64, gx: f32, gy: f32, gz: f32, ax: f32, ay: f32, az: f32) -> RawEvent {
    RawEvent::Motion(MotionEvent { timestamp_ms: ts, gyro_x: gx, gyro_y: gy, gyro_z: gz, accel_x: ax, accel_y: ay, accel_z: az })
}

/// Build N identical keystrokes with dwell=100ms, flight=200ms.
fn uniform_keystrokes(n: usize, dwell: u64, flight: u64) -> Vec<RawEvent> {
    (0..n)
        .map(|i| {
            let down = i as u64 * (dwell + flight);
            let f = if i == 0 { None } else { Some(flight) };
            keystroke(down, down + dwell, f, false)
        })
        .collect()
}

// ── Threshold tests ───────────────────────────────────────────────────────────

#[test]
fn returns_none_with_no_events() {
    assert!(extract(&[]).is_none());
}

#[test]
fn returns_none_with_four_keystrokes_and_no_touches() {
    let events = uniform_keystrokes(4, 80, 150);
    assert!(extract(&events).is_none());
}

#[test]
fn returns_some_with_five_keystrokes() {
    let events = uniform_keystrokes(5, 80, 150);
    assert!(extract(&events).is_some());
}

#[test]
fn returns_some_with_three_touches_and_no_keystrokes() {
    let events = vec![
        touch(0, 80, 0.5, 0.5, 0.6, 0.4),
        touch(200, 280, 0.4, 0.6, 0.5, 0.3),
        touch(400, 480, 0.6, 0.4, 0.7, 0.5),
    ];
    assert!(extract(&events).is_some());
}

// ── Feature value correctness ─────────────────────────────────────────────────

#[test]
fn keystroke_dwell_mean_is_correct() {
    // 5 keystrokes all with dwell=100ms → feature[0] = 100.0
    let events = uniform_keystrokes(5, 100, 200);
    let FeatureVector(f) = extract(&events).unwrap();
    assert_abs_diff_eq!(f[0], 100.0, epsilon = 1e-3);
}

#[test]
fn keystroke_dwell_std_zero_for_uniform_input() {
    let events = uniform_keystrokes(5, 100, 200);
    let FeatureVector(f) = extract(&events).unwrap();
    assert_abs_diff_eq!(f[1], 0.0, epsilon = 1e-3); // std of identical dwells = 0
}

#[test]
fn keystroke_flight_mean_is_correct() {
    // keystrokes 1..4 have flight=200 → feature[2] = 200.0
    let events = uniform_keystrokes(5, 100, 200);
    let FeatureVector(f) = extract(&events).unwrap();
    assert_abs_diff_eq!(f[2], 200.0, epsilon = 1e-3);
}

#[test]
fn correction_rate_is_correct() {
    // 5 keystrokes, 2 are corrections → rate = 0.4
    let events = vec![
        keystroke(0,   80,  None,       false),
        keystroke(280, 360, Some(200),  true),   // correction
        keystroke(560, 640, Some(200),  false),
        keystroke(840, 920, Some(200),  true),   // correction
        keystroke(1120, 1200, Some(200), false),
    ];
    let FeatureVector(f) = extract(&events).unwrap();
    assert_abs_diff_eq!(f[4], 0.4, epsilon = 1e-5);
}

#[test]
fn count_norm_capped_at_one() {
    // 300 keystrokes — norm = min(300/200, 1.0) = 1.0
    let events = uniform_keystrokes(300, 80, 150);
    let FeatureVector(f) = extract(&events).unwrap();
    assert_abs_diff_eq!(f[5], 1.0, epsilon = 1e-5);
}

#[test]
fn touch_pressure_mean_is_correct() {
    let events = vec![
        touch(0,   80,  0.5, 0.5, 0.8, 0.4),
        touch(200, 280, 0.5, 0.5, 0.8, 0.4),
        touch(400, 480, 0.5, 0.5, 0.8, 0.4),
    ];
    let FeatureVector(f) = extract(&events).unwrap();
    assert_abs_diff_eq!(f[8], 0.8, epsilon = 1e-5); // pressure mean = 0.8
}

#[test]
fn swipe_avg_velocity_is_correct() {
    // distance = sqrt(0.3^2 + 0.4^2) = 0.5; duration = 500ms = 0.5s → avg_vel = 1.0
    // Add 3 touches to meet the minimum event threshold (5 keystrokes OR 3 touches).
    let events = vec![
        touch(0,   80,  0.5, 0.5, 0.6, 0.4),
        touch(200, 280, 0.4, 0.6, 0.5, 0.3),
        touch(400, 480, 0.6, 0.4, 0.7, 0.5),
        swipe(600,  1100, 0.0, 0.0, 0.3, 0.4, 2.0),
        swipe(1200, 1700, 0.0, 0.0, 0.3, 0.4, 2.0),
        swipe(1800, 2300, 0.0, 0.0, 0.3, 0.4, 2.0),
    ];
    let FeatureVector(f) = extract(&events).unwrap();
    assert_abs_diff_eq!(f[14], 1.0, epsilon = 1e-4); // avg_vel mean = 1.0
}

#[test]
fn motion_gyro_x_mean_is_correct() {
    let events: Vec<RawEvent> = (0..5)
        .map(|i| motion(i * 20, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0))
        .chain(uniform_keystrokes(5, 80, 150))
        .collect();
    let FeatureVector(f) = extract(&events).unwrap();
    assert_abs_diff_eq!(f[18], 0.5, epsilon = 1e-5); // gyro_x mean = 0.5
}

// ── Output validity ───────────────────────────────────────────────────────────

#[test]
fn all_features_are_finite() {
    let mut events = uniform_keystrokes(10, 80, 150);
    events.push(touch(2000, 2080, 0.5, 0.5, 0.6, 0.4));
    events.push(swipe(3000, 3500, 0.1, 0.1, 0.4, 0.5, 1.2));
    events.push(motion(4000, 0.1, 0.2, 0.05, 0.3, 0.1, 9.8));

    let FeatureVector(f) = extract(&events).unwrap();
    for (i, &v) in f.iter().enumerate() {
        assert!(v.is_finite(), "feature[{i}] = {v} is not finite");
    }
}

#[test]
fn feature_vector_has_correct_dimension() {
    let events = uniform_keystrokes(5, 80, 150);
    let FeatureVector(f) = extract(&events).unwrap();
    assert_eq!(f.len(), FEATURE_DIM);
}

#[test]
fn missing_signal_types_produce_zero_features() {
    // Only keystrokes — swipe and motion features should be 0
    let events = uniform_keystrokes(5, 80, 150);
    let FeatureVector(f) = extract(&events).unwrap();
    // Swipe features [12..17] should be 0 (no swipes)
    for i in 12..18 {
        assert_abs_diff_eq!(f[i], 0.0, epsilon = 1e-6);
    }
}
