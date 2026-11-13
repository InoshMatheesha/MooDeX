// error.rs - Each variant maps to a distinct UI message (spec 9.2).

use serde::Serialize;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum RawgError {
    #[error("no API key configured")]
    NoKey,
    #[error("rate limited; retry after {0:?}")]
    RateLimited(Duration),
    #[error("offline")]
    Offline,
    #[error("upstream returned {0}")]
    Upstream(u16),
    #[error("malformed response: {0}")]
    Decode(String),
    #[error("request failed: {0}")]
    Http(String),
}
