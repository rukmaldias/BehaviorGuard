use serde::{Deserialize, Serialize};

/// A snapshot from the device IMU sensors during an interaction.
///
/// Collect via `SensorManager` with `TYPE_GYROSCOPE` and
/// `TYPE_LINEAR_ACCELERATION`. Sample at 50 Hz during active sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionEvent {
    /// Milliseconds from session start.
    pub timestamp_ms: u64,
    /// Gyroscope — angular velocity around X axis (rad/s). Positive = tilt right.
    pub gyro_x: f32,
    /// Gyroscope — angular velocity around Y axis (rad/s). Positive = tilt up.
    pub gyro_y: f32,
    /// Gyroscope — angular velocity around Z axis (rad/s). Positive = rotate CCW.
    pub gyro_z: f32,
    /// Linear acceleration X axis (m/s²), gravity removed.
    pub accel_x: f32,
    /// Linear acceleration Y axis (m/s²), gravity removed.
    pub accel_y: f32,
    /// Linear acceleration Z axis (m/s²), gravity removed.
    pub accel_z: f32,
}

impl MotionEvent {
    /// Gyroscope magnitude — overall rotation rate.
    pub fn gyro_magnitude(&self) -> f32 {
        (self.gyro_x.powi(2) + self.gyro_y.powi(2) + self.gyro_z.powi(2)).sqrt()
    }

    /// Linear acceleration magnitude.
    pub fn accel_magnitude(&self) -> f32 {
        (self.accel_x.powi(2) + self.accel_y.powi(2) + self.accel_z.powi(2)).sqrt()
    }
}
