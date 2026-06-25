use serde::{Deserialize, Serialize};

/// A swipe or scroll gesture captured as start/end points and timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwipeEvent {
    /// Milliseconds from session start when the gesture began.
    pub start_ms: u64,
    /// Milliseconds from session start when the finger was lifted.
    pub end_ms: u64,
    /// Normalised start X [0.0, 1.0].
    pub start_x: f32,
    /// Normalised start Y [0.0, 1.0].
    pub start_y: f32,
    /// Normalised end X [0.0, 1.0].
    pub end_x: f32,
    /// Normalised end Y [0.0, 1.0].
    pub end_y: f32,
    /// Peak velocity in normalised units per second during the gesture.
    pub peak_velocity: f32,
}

impl SwipeEvent {
    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }

    /// Euclidean distance of the swipe in normalised screen units.
    pub fn distance(&self) -> f32 {
        let dx = self.end_x - self.start_x;
        let dy = self.end_y - self.start_y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Average velocity: distance / duration.
    pub fn avg_velocity(&self) -> f32 {
        let dur_s = self.duration_ms() as f32 / 1000.0;
        if dur_s < f32::EPSILON {
            return 0.0;
        }
        self.distance() / dur_s
    }
}
