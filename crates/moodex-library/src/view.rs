// view.rs - What the frontend renders; every number here came from the actor (spec 12.2).

use moodex_core::playtime::format_playtime;
use moodex_core::{DashboardStats, Game, GameId, Install, PlayStats, Status};
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GameView {
    pub id: GameId,
    pub rawg_id: Option<i64>,
    pub title: String,
    pub cover_url: Option<String>,
    pub cover_key: Option<String>,
    pub cover_cached: bool,
    pub release_date: Option<String>,
    pub status: Status,
    pub rating: Option<u8>,
    pub notes: String,
    pub is_goat: bool,
    pub genres: Vec<String>,
    pub install: Option<Install>,
    pub installed: bool,
    pub install_missing: bool,
    pub added_at: String,
    pub updated_at: String,
    pub minutes: f64,
    pub playtime: String,
    pub play_count: u32,
    pub last_played: Option<String>,
    pub running_since: Option<String>,
    pub has_estimate: bool,
}

pub(crate) fn ts(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_default()
}

pub(crate) struct ViewCtx<'a> {
    pub images: &'a moodex_images::ImageCache,
}

impl ViewCtx<'_> {
    #[allow(clippy::too_many_arguments)] // flat inputs keep the actor free of view concerns
    pub fn build(
        &self,
        g: &Game,
        p: &PlayStats,
        missing: bool,
        running: Option<OffsetDateTime>,
        estimate: bool,
    ) -> GameView {
        let url = g.cover.url().map(str::to_string);
        let key = url.as_deref().map(|u| self.images.register(u).0);
        let cached = matches!(g.cover, moodex_core::CoverRef::Cached { .. });
        let release_fmt = time::macros::format_description!("[year]-[month]-[day]");
        GameView {
            id: g.id,
            rawg_id: g.rawg_id,
            title: g.title.clone(),
            cover_url: url,
            cover_key: key,
            cover_cached: cached,
            release_date: g.release_date.and_then(|d| d.format(&release_fmt).ok()),
            status: g.status,
            rating: g.rating.map(|r| r.get()),
            notes: g.notes.clone(),
            is_goat: g.is_goat,
            genres: g.genres.clone(),
            install: g.install.clone(),
            installed: g.install.is_some(),
            install_missing: missing,
            added_at: ts(g.added_at),
            updated_at: ts(g.updated_at),
            minutes: p.minutes,
            playtime: format_playtime(p.minutes),
            play_count: p.play_count,
            last_played: p.last_played.map(ts),
            running_since: running.map(ts),
            has_estimate: estimate,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DashboardView {
    pub stats: DashboardStats,
    pub most_played: Vec<GameView>,
    pub recently_played: Vec<GameView>,
    pub playing_now: Vec<GameView>,
    pub goat: Option<GameView>,
}
