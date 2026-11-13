// wire.rs - Typed RAWG response shapes with defaults for optional fields (spec 9.3).

use moodex_core::RawgGame;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Page<T> {
    pub results: Vec<T>,
    pub count: i64,
    pub has_next: bool,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct WireNamed {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct WireGame {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub background_image: Option<String>,
    #[serde(default)]
    pub released: Option<String>,
    #[serde(default)]
    pub rating: Option<f64>,
    #[serde(default)]
    pub metacritic: Option<i64>,
    #[serde(default)]
    pub added: Option<i64>,
    #[serde(default)]
    pub genres: Option<Vec<WireNamed>>,
}

impl From<WireGame> for RawgGame {
    fn from(w: WireGame) -> Self {
        RawgGame {
            id: w.id,
            name: w.name,
            background_image: w.background_image.filter(|s| !s.is_empty()),
            released: w.released,
            rating: w.rating.unwrap_or(0.0),
            metacritic: w.metacritic,
            added: w.added.unwrap_or(0),
            genres: w.genres.unwrap_or_default().into_iter().map(|g| g.name).filter(|n| !n.is_empty()).collect(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct WirePage {
    #[serde(default)]
    pub count: i64,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub results: Vec<WireGame>,
}

impl From<WirePage> for Page<RawgGame> {
    fn from(w: WirePage) -> Self {
        Page {
            count: w.count,
            has_next: w.next.is_some_and(|n| !n.is_empty()),
            results: w.results.into_iter().map(RawgGame::from).collect(),
        }
    }
}

// Detail view of one game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawgGameDetail {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub background_image: Option<String>,
    pub released: Option<String>,
    pub rating: f64,
    pub metacritic: Option<i64>,
    pub playtime_hours: i64,
    pub website: Option<String>,
    pub genres: Vec<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub platforms: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct WirePlatformWrap {
    #[serde(default)]
    pub platform: WireNamed,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct WireDetail {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description_raw: Option<String>,
    #[serde(default)]
    pub background_image: Option<String>,
    #[serde(default)]
    pub released: Option<String>,
    #[serde(default)]
    pub rating: Option<f64>,
    #[serde(default)]
    pub metacritic: Option<i64>,
    #[serde(default)]
    pub playtime: Option<i64>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub genres: Option<Vec<WireNamed>>,
    #[serde(default)]
    pub developers: Option<Vec<WireNamed>>,
    #[serde(default)]
    pub publishers: Option<Vec<WireNamed>>,
    #[serde(default)]
    pub platforms: Option<Vec<WirePlatformWrap>>,
}

fn names(v: Option<Vec<WireNamed>>) -> Vec<String> {
    v.unwrap_or_default().into_iter().map(|n| n.name).filter(|n| !n.is_empty()).collect()
}

impl From<WireDetail> for RawgGameDetail {
    fn from(w: WireDetail) -> Self {
        RawgGameDetail {
            id: w.id,
            name: w.name,
            description: w.description_raw.unwrap_or_default(),
            background_image: w.background_image.filter(|s| !s.is_empty()),
            released: w.released,
            rating: w.rating.unwrap_or(0.0),
            metacritic: w.metacritic,
            playtime_hours: w.playtime.unwrap_or(0),
            website: w.website.filter(|s| !s.is_empty()),
            genres: names(w.genres),
            developers: names(w.developers),
            publishers: names(w.publishers),
            platforms: w
                .platforms
                .unwrap_or_default()
                .into_iter()
                .map(|p| p.platform.name)
                .filter(|n| !n.is_empty())
                .collect(),
        }
    }
}
