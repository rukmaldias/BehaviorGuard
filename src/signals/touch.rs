use serde::{Deserialize, Serialize};

/// A single tap event from a `MotionEvent.ACTION_DOWN` / `ACTION_UP` pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchEvent {
    /// Milliseconds from session start at ACTION_DOWN.
    pub down_ms: u64,
    /// Milliseconds from session start at ACTION_UP.
    pub up_ms: u64,
    /// Normalised X position [0.0, 1.0] relative to screen width.
    pub x: f32,
    /// Normalised Y position [0.0, 1.0] relative to screen height.
    pub y: f32,
    /// Touch pressure [0.0, 1.0] reported by the digitiser. Not available on
    /// all hardware; defaults to 0.5 when unsupported.
    pub pressure: f32,
    /// Contact area in normalised screen units. Larger values = broader finger
    /// contact or stylus. Not available on all hardware.
    pub area: f32,
}

impl TouchEvent {
    /// Tap duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.up_ms.saturating_sub(self.down_ms)
    }
}
