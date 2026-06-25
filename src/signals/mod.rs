/// Raw behavioral events collected from Android input APIs.
///
/// All timestamps are milliseconds since session start (relative, not Unix).
/// This keeps values small and avoids leaking device clock information.
use serde::{Deserialize, Serialize};

pub mod keystroke;
pub mod motion;
pub mod swipe;
pub mod touch;

pub use keystroke::KeystrokeEvent;
pub use motion::MotionEvent;
pub use swipe::SwipeEvent;
pub use touch::TouchEvent;

/// A single raw event from any signal source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RawEvent {
    Keystroke(KeystrokeEvent),
    Touch(TouchEvent),
    Swipe(SwipeEvent),
    Motion(MotionEvent),
}

impl RawEvent {
    pub fn timestamp_ms(&self) -> u64 {
        match self {
            RawEvent::Keystroke(e) => e.down_ms,
            RawEvent::Touch(e) => e.down_ms,
            RawEvent::Swipe(e) => e.start_ms,
            RawEvent::Motion(e) => e.timestamp_ms,
        }
    }
}
