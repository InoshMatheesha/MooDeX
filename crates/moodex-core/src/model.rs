// model.rs - Domain types shared by every crate (spec 6.2).

use crate::CoreError;
use camino::Utf8PathBuf;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::str::FromStr;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

// Stable game identity - UUIDv7, never a title (spec 1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GameId(pub Uuid);

impl GameId {
    // New time-sortable id.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for GameId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for GameId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for GameId {
    type Err = CoreError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s.trim()).map(GameId).map_err(|_| CoreError::InvalidId(s.to_string()))
    }
}

// Library status of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Playing,
    Completed,
    #[default]
    Backlog,
    Dropped,
}

impl Status {
    pub const ALL: [Status; 4] = [Status::Playing, Status::Completed, Status::Backlog, Status::Dropped];

    // Storage string, matches the CHECK constraint.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Playing => "playing",
            Status::Completed => "completed",
            Status::Backlog => "backlog",
            Status::Dropped => "dropped",
        }
    }
}

impl FromStr for Status {
    type Err = CoreError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "playing" => Ok(Status::Playing),
            "completed" => Ok(Status::Completed),
            "backlog" => Ok(Status::Backlog),
            "dropped" => Ok(Status::Dropped),
            other => Err(CoreError::InvalidStatus(other.to_string())),
        }
    }
}

// User rating 1..=5, validated on construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct Rating(u8);

impl Rating {
    pub fn new(v: i64) -> Result<Self, CoreError> {
        if (1..=5).contains(&v) {
            Ok(Rating(v as u8))
        } else {
            Err(CoreError::InvalidRating(v))
        }
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<i64> for Rating {
    type Error = CoreError;
    fn try_from(v: i64) -> Result<Self, Self::Error> {
        Rating::new(v)
    }
}

impl From<Rating> for i64 {
    fn from(r: Rating) -> i64 {
        i64::from(r.0)
    }
}

// Hex SHA-256 content address into the image cache.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentHash(pub String);

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// What we execute when the user presses Play (spec 8.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum LaunchTarget {
    Exe {
        path: Utf8PathBuf,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        working_dir: Option<Utf8PathBuf>,
    },
    Uri {
        uri: String,
    },
    Shortcut {
        path: Utf8PathBuf,
    },
}

impl LaunchTarget {
    // Reject obviously unusable targets before they reach storage.
    pub fn validate(&self) -> Result<(), CoreError> {
        match self {
            LaunchTarget::Exe { path, .. } | LaunchTarget::Shortcut { path } => {
                if path.as_str().trim().is_empty() || !path.is_absolute() {
                    return Err(CoreError::InvalidLaunchTarget(format!("path must be absolute: {path}")));
                }
                Ok(())
            }
            LaunchTarget::Uri { uri } => {
                let scheme_ok = uri
                    .split_once("://")
                    .map(|(s, _)| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c)))
                    .unwrap_or(false);
                if !scheme_ok {
                    return Err(CoreError::InvalidLaunchTarget(format!("not a URI: {uri}")));
                }
                let scheme = uri.split("://").next().unwrap_or("").to_ascii_lowercase();
                // Never let a stored URI become a generic web or file opener.
                if matches!(scheme.as_str(), "http" | "https" | "file" | "javascript" | "data") {
                    return Err(CoreError::InvalidLaunchTarget(format!("scheme not allowed: {scheme}")));
                }
                Ok(())
            }
        }
    }
}

// Launch/detect split - the fix for launcher-stub games (spec 8.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Install {
    pub launch: LaunchTarget,
    #[serde(default)]
    pub detect_exe: Option<Utf8PathBuf>,
}

impl Install {
    // For a plain exe the detect target defaults to the launched exe.
    pub fn effective_detect(&self) -> Option<&Utf8PathBuf> {
        match (&self.detect_exe, &self.launch) {
            (Some(p), _) => Some(p),
            (None, LaunchTarget::Exe { path, .. }) => Some(path),
            _ => None,
        }
    }
}

// Where a cover comes from and whether it is cached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind")]
pub enum CoverRef {
    #[default]
    None,
    Remote {
        url: String,
    },
    Cached {
        hash: ContentHash,
        url: String,
    },
}

impl CoverRef {
    pub fn url(&self) -> Option<&str> {
        match self {
            CoverRef::None => None,
            CoverRef::Remote { url } | CoverRef::Cached { url, .. } => Some(url),
        }
    }

    pub fn from_parts(url: Option<String>, hash: Option<String>) -> Self {
        match (url, hash) {
            (Some(url), Some(h)) if !url.is_empty() => CoverRef::Cached { hash: ContentHash(h), url },
            (Some(url), _) if !url.is_empty() => CoverRef::Remote { url },
            _ => CoverRef::None,
        }
    }
}

// One game - the unified record; "PC Games" is a filter (spec 6.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub id: GameId,
    pub rawg_id: Option<i64>,
    pub title: String,
    pub cover: CoverRef,
    pub release_date: Option<Date>,
    pub status: Status,
    pub rating: Option<Rating>,
    pub notes: String,
    pub is_goat: bool,
    pub install: Option<Install>,
    pub genres: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub added_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

// Derived playtime - built from play_sessions, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct PlayStats {
    pub minutes: f64,
    pub play_count: u32,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_played: Option<OffsetDateTime>,
}

// How a session was detected - matches the CHECK constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionSource {
    JobObject,
    Wmi,
    Manual,
}

impl SessionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionSource::JobObject => "job_object",
            SessionSource::Wmi => "wmi",
            SessionSource::Manual => "manual",
        }
    }
}

impl FromStr for SessionSource {
    type Err = CoreError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "job_object" => Ok(SessionSource::JobObject),
            "wmi" => Ok(SessionSource::Wmi),
            "manual" => Ok(SessionSource::Manual),
            other => Err(CoreError::InvalidStatus(other.to_string())),
        }
    }
}

// A session row still open (ended_at IS NULL).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenSession {
    pub id: i64,
    pub game_id: GameId,
    pub started_at: OffsetDateTime,
    pub last_heartbeat: OffsetDateTime,
    pub source: SessionSource,
}

// The decision for closing a session.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionClose {
    pub id: i64,
    pub ended_at: OffsetDateTime,
    pub minutes: f64,
}

// One historical session for the detail view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ended_at: Option<OffsetDateTime>,
    pub minutes: f64,
    pub source: SessionSource,
}

// Input for creating a game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GameDraft {
    pub title: String,
    #[serde(default)]
    pub rawg_id: Option<i64>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub release_date: Option<Date>,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub rating: Option<Rating>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub install: Option<Install>,
}

impl GameDraft {
    // Turn a draft into a full game with fresh identity and timestamps.
    pub fn into_game(self, now: OffsetDateTime) -> Result<Game, CoreError> {
        let title = self.title.trim().to_string();
        if title.is_empty() {
            return Err(CoreError::EmptyTitle);
        }
        if let Some(i) = &self.install {
            i.launch.validate()?;
        }
        Ok(Game {
            id: GameId::new(),
            rawg_id: self.rawg_id,
            title,
            cover: CoverRef::from_parts(self.cover_url, None),
            release_date: self.release_date,
            status: self.status,
            rating: self.rating,
            notes: self.notes,
            is_goat: false,
            install: self.install,
            genres: crate::text::clean_genres(self.genres),
            added_at: now,
            updated_at: now,
        })
    }
}

// Distinguish "field absent" from "field set to null" in patches.
pub fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

// Partial update; every field optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GamePatch {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<Status>,
    #[serde(default, deserialize_with = "double_option")]
    pub rating: Option<Option<Rating>>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub cover_url: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub rawg_id: Option<Option<i64>>,
    #[serde(default, deserialize_with = "double_option")]
    pub release_date: Option<Option<Date>>,
    #[serde(default)]
    pub genres: Option<Vec<String>>,
}

impl GamePatch {
    // Apply to a game in place; returns an error without touching it on bad input.
    pub fn apply(&self, g: &mut Game, now: OffsetDateTime) -> Result<(), CoreError> {
        if let Some(t) = &self.title {
            if t.trim().is_empty() {
                return Err(CoreError::EmptyTitle);
            }
        }
        if let Some(t) = &self.title {
            g.title = t.trim().to_string();
        }
        if let Some(s) = self.status {
            g.status = s;
        }
        if let Some(r) = self.rating {
            g.rating = r;
        }
        if let Some(n) = &self.notes {
            g.notes = n.clone();
        }
        if let Some(c) = &self.cover_url {
            let same = g.cover.url() == c.as_deref();
            if !same {
                g.cover = CoverRef::from_parts(c.clone(), None);
            }
        }
        if let Some(r) = self.rawg_id {
            g.rawg_id = r;
        }
        if let Some(d) = self.release_date {
            g.release_date = d;
        }
        if let Some(gs) = &self.genres {
            g.genres = crate::text::clean_genres(gs.clone());
        }
        g.updated_at = now;
        Ok(())
    }
}

// Which games a list view shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GameFilter {
    #[serde(default)]
    pub status: Option<Status>,
    #[serde(default)]
    pub installed_only: bool,
    #[serde(default)]
    pub query: Option<String>,
}

// List ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    #[default]
    Title,
    RecentlyAdded,
    Playtime,
    Rating,
    LastPlayed,
    ReleaseDate,
}

// A RAWG catalogue entry, as used by pure ranking logic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawgGame {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub background_image: Option<String>,
    #[serde(default)]
    pub released: Option<String>,
    #[serde(default)]
    pub rating: f64,
    #[serde(default)]
    pub metacritic: Option<i64>,
    #[serde(default)]
    pub added: i64,
    #[serde(default)]
    pub genres: Vec<String>,
}

// A value with a relevance score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scored<T> {
    pub score: f64,
    pub item: T,
}

// Why two games look like duplicates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DuplicateReason {
    SameTitle,
    SimilarTitle { score: f64 },
}

// A merge suggestion - never applied automatically (spec 1.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicateCandidate {
    pub a: GameId,
    pub b: GameId,
    pub reason: DuplicateReason,
}

// Aggregate numbers for the dashboard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DashboardStats {
    pub total: usize,
    pub playing: usize,
    pub completed: usize,
    pub backlog: usize,
    pub dropped: usize,
    pub installed: usize,
    pub total_minutes: f64,
    pub total_playtime: String,
    pub total_sessions: u64,
    pub average_rating: Option<f64>,
    pub most_played: Vec<GameId>,
    pub recently_played: Vec<GameId>,
    pub goat: Option<GameId>,
}
