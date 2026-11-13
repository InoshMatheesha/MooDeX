// error.rs - Store failures; constraint violations are surfaced as conflicts.

use moodex_core::GameId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlite(rusqlite::Error),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("corrupt data: {0}")]
    Corrupt(String),
    #[error("game not found: {0}")]
    NotFound(GameId),
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        // UNIQUE / CHECK violations are user-facing conflicts, not crashes.
        if let rusqlite::Error::SqliteFailure(f, msg) = &e {
            if f.code == rusqlite::ErrorCode::ConstraintViolation {
                let m = msg.clone().unwrap_or_else(|| "constraint violation".to_string());
                let friendly = if m.contains("detect_exe_path") {
                    "another game already uses that executable for detection".to_string()
                } else if m.contains("rawg_id") {
                    "another game is already linked to that RAWG entry".to_string()
                } else if m.contains("idx_sessions_one_open") || m.contains("play_sessions.game_id") {
                    "the game already has an open session".to_string()
                } else {
                    m
                };
                return StoreError::Conflict(friendly);
            }
        }
        StoreError::Sqlite(e)
    }
}
