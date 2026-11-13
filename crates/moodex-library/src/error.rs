// error.rs - Library errors; serialisable so the UI can render each kind inline.

use moodex_core::{CoreError, GameId};
use moodex_store::StoreError;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum LibraryError {
    #[error("the library is busy; try again")]
    Busy,
    #[error("the library has shut down")]
    Closed,
    #[error("game not found: {0}")]
    NotFound(GameId),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Conflict(String),
    #[error("storage error: {0}")]
    Store(String),
    #[error("file error: {0}")]
    Io(String),
    #[error("import failed: {0}")]
    Import(String),
}

impl From<StoreError> for LibraryError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::Conflict(m) => LibraryError::Conflict(m),
            StoreError::NotFound(id) => LibraryError::NotFound(id),
            other => LibraryError::Store(other.to_string()),
        }
    }
}

impl From<CoreError> for LibraryError {
    fn from(e: CoreError) -> Self {
        LibraryError::Invalid(e.to_string())
    }
}

impl From<std::io::Error> for LibraryError {
    fn from(e: std::io::Error) -> Self {
        LibraryError::Io(e.to_string())
    }
}
