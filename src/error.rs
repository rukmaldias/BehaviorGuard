use thiserror::Error;

#[derive(Debug, Error)]
pub enum BgError {
    #[error("not enrolled: need at least {0} sessions before scoring")]
    NotEnrolled(usize),

    #[error("session already active")]
    SessionActive,

    #[error("no active session")]
    NoSession,

    #[error("insufficient events in session (got {got}, need {need})")]
    InsufficientEvents { got: usize, need: usize },

    #[error("model not loaded")]
    ModelNotLoaded,

    #[error("crypto error")]
    Crypto,

    #[error("serialisation error: {0}")]
    Serialise(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

pub type Result<T> = std::result::Result<T, BgError>;
