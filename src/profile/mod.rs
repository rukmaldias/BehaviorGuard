/// User behavioral profile — enrollment and baseline storage.
///
/// The profile is the statistical summary of a user's enrolled sessions.
/// It is stored encrypted on-device; only the risk scorer reads it.
pub mod enrollment;
pub mod storage;

pub use enrollment::{EnrollmentState, SESSIONS_REQUIRED};
pub use storage::ProfileStore;
