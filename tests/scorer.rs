use approx::assert_abs_diff_eq;
use behavior_guard::{
    FeatureVector, Scorer, FEATURE_DIM,
    profile::enrollment::{BaselineProfile, SESSIONS_REQUIRED},
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn profile_with(mean: f32, std: f32) -> BaselineProfile {
    BaselineProfile {
        mean: [mean; FEATURE_DIM],
        std:  [std;  FEATURE_DIM],
        session_count: SESSIONS_REQUIRED,
    }
}

fn fv(value: f32) -> FeatureVector {
    FeatureVector([value; FEATURE_DIM])
}

// ── Score range ───────────────────────────────────────────────────────────────

#[test]
fn score_is_zero_when_fv_matches_mean_exactly() {
    let profile = profile_with(5.0, 1.0);
    let result = Scorer::score(&fv(5.0), &profile, 50);
    assert_abs_diff_eq!(result.score, 0.0, epsilon = 1e-6);
}

#[test]
fn score_increases_with_deviation() {
    let profile = profile_with(5.0, 1.0);
    let low  = Scorer::score(&fv(5.5),  &profile, 50).score; // z=0.5
    let high = Scorer::score(&fv(10.0), &profile, 50).score; // z=5.0
    assert!(high > low, "larger deviation should produce higher score");
}

#[test]
fn score_is_symmetric_around_mean() {
    let profile = profile_with(5.0, 1.0);
    let above = Scorer::score(&fv(6.0), &profile, 50).score; // z=+1
    let below = Scorer::score(&fv(4.0), &profile, 50).score; // z=-1
    assert_abs_diff_eq!(above, below, epsilon = 1e-6);
}

#[test]
fn score_is_clamped_to_one() {
    let profile = profile_with(0.0, 1.0);
    let result = Scorer::score(&fv(1000.0), &profile, 100);
    assert_abs_diff_eq!(result.score, 1.0, epsilon = 1e-6);
}

#[test]
fn score_at_three_std_devs_is_near_one() {
    // mean_z = 3.0 → score = 3.0/3.0 = 1.0
    let profile = profile_with(0.0, 1.0);
    let result = Scorer::score(&fv(3.0), &profile, 50);
    assert_abs_diff_eq!(result.score, 1.0, epsilon = 1e-5);
}

#[test]
fn score_at_one_std_dev_is_near_one_third() {
    // mean_z = 1.0 → score = 1.0/3.0 ≈ 0.333
    let profile = profile_with(0.0, 1.0);
    let result = Scorer::score(&fv(1.0), &profile, 50);
    assert_abs_diff_eq!(result.score, 1.0 / 3.0, epsilon = 1e-5);
}

#[test]
fn score_is_always_in_zero_one_range() {
    let profile = profile_with(5.0, 2.0);
    for &val in &[-100.0f32, 0.0, 5.0, 5.5, 10.0, 1000.0] {
        let s = Scorer::score(&fv(val), &profile, 20).score;
        assert!(s >= 0.0 && s <= 1.0, "score {s} out of [0,1] for fv={val}");
    }
}

// ── Confidence ────────────────────────────────────────────────────────────────

#[test]
fn confidence_is_zero_with_no_events() {
    let profile = profile_with(5.0, 1.0);
    let result = Scorer::score(&fv(5.0), &profile, 0);
    assert_abs_diff_eq!(result.confidence, 0.0, epsilon = 1e-6);
}

#[test]
fn confidence_grows_with_events() {
    let profile = profile_with(5.0, 1.0);
    let low  = Scorer::score(&fv(5.0), &profile, 10).confidence;
    let high = Scorer::score(&fv(5.0), &profile, 50).confidence;
    assert!(high > low, "more events should increase confidence");
}

#[test]
fn confidence_capped_at_one() {
    let profile = profile_with(5.0, 1.0);
    let result = Scorer::score(&fv(5.0), &profile, 10_000);
    assert!(result.confidence <= 1.0);
}

#[test]
fn events_used_is_stored_in_result() {
    let profile = profile_with(5.0, 1.0);
    let result = Scorer::score(&fv(5.0), &profile, 42);
    assert_eq!(result.events_used, 42);
}

// ── is_anomalous helper ───────────────────────────────────────────────────────

#[test]
fn is_anomalous_respects_threshold() {
    let profile = profile_with(0.0, 1.0);
    let result = Scorer::score(&fv(2.0), &profile, 50); // score ≈ 0.667
    assert!(result.is_anomalous(0.5));
    assert!(!result.is_anomalous(0.9));
}

// ── Full pipeline: enrollment → scoring ──────────────────────────────────────

#[test]
fn enrolled_from_identical_sessions_scores_match_as_zero() {
    use behavior_guard::{BehaviorGuard, RawEvent, KeystrokeEvent, SessionOutcome};

    let mut guard = BehaviorGuard::new();

    // Build enrollment sessions — 5 sessions of identical typing
    fn make_session() -> Vec<RawEvent> {
        (0..10u64).map(|i| {
            RawEvent::Keystroke(KeystrokeEvent {
                down_ms: i * 280,
                up_ms: i * 280 + 100,
                flight_ms: if i == 0 { None } else { Some(180) },
                is_correction: false,
            })
        }).collect()
    }

    for _ in 0..SESSIONS_REQUIRED {
        guard.start_session().unwrap();
        for e in make_session() { guard.add_event(e).unwrap(); }
        guard.end_session().unwrap();
    }

    assert!(guard.is_enrolled());

    // Score a session identical to enrollment — should be low risk
    guard.start_session().unwrap();
    for e in make_session() { guard.add_event(e).unwrap(); }
    let outcome = guard.end_session().unwrap();

    match outcome {
        SessionOutcome::Scored(s) => {
            assert!(s.score < 0.3, "identical session should score low risk, got {}", s.score);
        }
        other => panic!("expected Scored, got {:?}", other),
    }
}

#[test]
fn very_different_session_scores_higher_than_identical() {
    use behavior_guard::{BehaviorGuard, RawEvent, KeystrokeEvent, SessionOutcome};

    fn session(dwell: u64, flight: u64) -> Vec<RawEvent> {
        (0..10u64).map(|i| {
            RawEvent::Keystroke(KeystrokeEvent {
                down_ms: i * (dwell + flight),
                up_ms: i * (dwell + flight) + dwell,
                flight_ms: if i == 0 { None } else { Some(flight) },
                is_correction: false,
            })
        }).collect()
    }

    let mut guard = BehaviorGuard::new();

    // Enroll with fast typing: dwell=80, flight=120
    for _ in 0..SESSIONS_REQUIRED {
        guard.start_session().unwrap();
        for e in session(80, 120) { guard.add_event(e).unwrap(); }
        guard.end_session().unwrap();
    }

    // Score a session matching enrollment
    guard.start_session().unwrap();
    for e in session(80, 120) { guard.add_event(e).unwrap(); }
    let baseline_score = match guard.end_session().unwrap() {
        SessionOutcome::Scored(s) => s.score,
        other => panic!("expected Scored, got {:?}", other),
    };

    // Score a very different session: slow typing dwell=500, flight=800
    guard.start_session().unwrap();
    for e in session(500, 800) { guard.add_event(e).unwrap(); }
    let anomaly_score = match guard.end_session().unwrap() {
        SessionOutcome::Scored(s) => s.score,
        other => panic!("expected Scored, got {:?}", other),
    };

    assert!(
        anomaly_score > baseline_score,
        "anomalous session ({anomaly_score:.3}) should score higher than baseline ({baseline_score:.3})"
    );
}
