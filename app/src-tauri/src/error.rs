// error.rs - One error shape for every command; the UI renders it inline (spec 12.4).

use moodex_library::{LibraryError, PlayError};
use moodex_rawg::RawgError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_secs: Option<u64>,
}

impl ApiError {
    pub fn new(kind: &str, message: impl Into<String>) -> Self {
        Self { kind: kind.into(), message: message.into(), retry_after_secs: None }
    }

    pub fn cancelled() -> Self {
        Self::new("cancelled", "superseded by a newer request")
    }
}

fn kind_of<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|j| j.get("kind").and_then(|k| k.as_str()).map(str::to_string))
        .unwrap_or_else(|| "error".into())
}

impl From<LibraryError> for ApiError {
    fn from(e: LibraryError) -> Self {
        Self::new(&kind_of(&e), e.to_string())
    }
}

impl From<PlayError> for ApiError {
    fn from(e: PlayError) -> Self {
        Self::new(&kind_of(&e), e.to_string())
    }
}

impl From<RawgError> for ApiError {
    fn from(e: RawgError) -> Self {
        let message = match &e {
            RawgError::NoKey => {
                "Discover needs a RAWG source - add your proxy URL or a personal key in Settings.".to_string()
            }
            RawgError::Offline => "You're offline - showing what's cached.".to_string(),
            RawgError::RateLimited(d) => format!("RAWG is rate limiting us; try again in {}s.", d.as_secs()),
            RawgError::Upstream(401) | RawgError::Upstream(403) => "RAWG rejected the API key.".to_string(),
            RawgError::Upstream(c) => format!("RAWG returned an error ({c})."),
            RawgError::Decode(_) => "RAWG sent a response we could not read.".to_string(),
            RawgError::Http(m) => format!("Request failed: {m}"),
        };
        let mut out = Self::new(&kind_of(&e), message);
        if let RawgError::RateLimited(d) = e {
            out.retry_after_secs = Some(d.as_secs());
        }
        out
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        Self::new("error", format!("{e:#}"))
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
