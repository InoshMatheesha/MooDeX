use thiserror::Error;

// Validation failures for domain values.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    #[error("rating must be between 1 and 5, got {0}")]
    InvalidRating(i64),
    #[error("unknown status: {0}")]
    InvalidStatus(String),
    #[error("invalid game id: {0}")]
    InvalidId(String),
    #[error("title must not be empty")]
    EmptyTitle,
    #[error("invalid launch target: {0}")]
    InvalidLaunchTarget(String),
}
