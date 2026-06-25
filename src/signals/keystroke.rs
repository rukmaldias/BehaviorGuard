use serde::{Deserialize, Serialize};

/// A single key press/release cycle captured from a soft keyboard.
///
/// Collect via `InputConnection` callbacks or `KeyEvent` listeners.
/// Dwell + flight times are the two most discriminating keystroke features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeystrokeEvent {
    /// Milliseconds from session start when the key was pressed.
    pub down_ms: u64,
    /// Milliseconds from session start when the key was released.
    pub up_ms: u64,
    /// Milliseconds between the previous key-up and this key-down (flight time).
    /// `None` for the first key in a session.
    pub flight_ms: Option<u64>,
    /// Whether this key was a backspace / delete correction.
    pub is_correction: bool,
}

impl KeystrokeEvent {
    /// Key hold duration in milliseconds.
    pub fn dwell_ms(&self) -> u64 {
        self.up_ms.saturating_sub(self.down_ms)
    }
}
