pub mod error;
pub mod features;
pub mod inference;
pub mod profile;
pub mod signals;

#[cfg(feature = "jni")]
pub mod jni_api;

pub use error::{BgError, Result};
pub use features::{extract, FeatureVector, FEATURE_DIM};
pub use inference::{RiskScore, Scorer};
pub use profile::{EnrollmentState, ProfileStore, SESSIONS_REQUIRED};
pub use signals::{KeystrokeEvent, MotionEvent, RawEvent, SwipeEvent, TouchEvent};

use profile::enrollment::BaselineProfile;

/// The top-level BehaviorGuard session manager.
///
/// Lifecycle:
/// 1. `new()` — create an instance (one per user, persisted across app launches)
/// 2. `start_session()` — begin collecting events
/// 3. `add_event()` — called on each input event from the Android layer
/// 4. `end_session()` — finalise; returns `SessionOutcome`
/// 5. `score()` — available once enrollment is complete
pub struct BehaviorGuard {
    state: State,
    active_events: Option<Vec<RawEvent>>,
}

enum State {
    Enrolling(EnrollmentState),
    Ready(BaselineProfile),
}

/// Outcome returned by `end_session`.
#[derive(Debug)]
pub enum SessionOutcome {
    /// Enrollment in progress. Shows how many more sessions are needed.
    Enrolling { sessions_remaining: usize },
    /// Enrollment just completed. Profile is now ready for scoring.
    EnrollmentComplete,
    /// A risk score was computed for this session.
    Scored(RiskScore),
}

impl BehaviorGuard {
    pub fn new() -> Self {
        Self {
            state: State::Enrolling(EnrollmentState::default()),
            active_events: None,
        }
    }

    /// Starts a new collection session. Returns an error if one is already active.
    pub fn start_session(&mut self) -> Result<()> {
        if self.active_events.is_some() {
            return Err(BgError::SessionActive);
        }
        self.active_events = Some(Vec::new());
        Ok(())
    }

    /// Adds a raw event to the active session.
    pub fn add_event(&mut self, event: RawEvent) -> Result<()> {
        self.active_events
            .as_mut()
            .ok_or(BgError::NoSession)?
            .push(event);
        Ok(())
    }

    /// Ends the session, extracts features, and returns a `SessionOutcome`.
    pub fn end_session(&mut self) -> Result<SessionOutcome> {
        let events = self.active_events.take().ok_or(BgError::NoSession)?;
        let event_count = events.len();

        let fv = features::extract(&events).ok_or(BgError::InsufficientEvents {
            got: event_count,
            need: 5,
        })?;

        match &mut self.state {
            State::Enrolling(enrollment) => {
                enrollment.add(fv);
                if enrollment.is_complete() {
                    let profile = enrollment.build_profile().unwrap();
                    self.state = State::Ready(profile);
                    Ok(SessionOutcome::EnrollmentComplete)
                } else {
                    let remaining = match &self.state {
                        State::Enrolling(e) => e.sessions_remaining(),
                        _ => 0,
                    };
                    Ok(SessionOutcome::Enrolling { sessions_remaining: remaining })
                }
            }
            State::Ready(profile) => {
                let score = Scorer::score(&fv, profile, event_count);
                Ok(SessionOutcome::Scored(score))
            }
        }
    }

    /// Returns `true` if enrollment is complete and scoring is available.
    pub fn is_enrolled(&self) -> bool {
        matches!(self.state, State::Ready(_))
    }

    /// Serialises and encrypts the profile to bytes for persistent storage.
    /// Returns `None` if enrollment is not yet complete.
    pub fn export_profile(&self, key: &[u8; 32]) -> Result<Option<Vec<u8>>> {
        match &self.state {
            State::Ready(profile) => Ok(Some(ProfileStore::seal(profile, key)?)),
            State::Enrolling(_) => Ok(None),
        }
    }

    /// Restores a previously exported profile.
    pub fn import_profile(&mut self, blob: &[u8], key: &[u8; 32]) -> Result<()> {
        let profile = ProfileStore::open(blob, key)?;
        self.state = State::Ready(profile);
        Ok(())
    }
}

impl Default for BehaviorGuard {
    fn default() -> Self {
        Self::new()
    }
}
