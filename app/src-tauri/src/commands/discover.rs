// discover.rs - RAWG browsing, cancel-on-keystroke search, and exe-to-cover matching (spec 9).

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use camino::Utf8PathBuf;
use moodex_core::matching::rank_matches;
use moodex_core::text::title_from_exe_path;
use moodex_core::{GameDraft, GameId, Install, LaunchTarget, RawgGame, Status};
use moodex_rawg::{Endpoint, Page, RawgGameDetail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct DiscoverGame {
    #[serde(flatten)]
    pub game: RawgGame,
    pub cover_key: Option<String>,
    pub library_id: Option<GameId>,
}

#[derive(Debug, Serialize)]
pub struct DiscoverPage {
    pub results: Vec<DiscoverGame>,
    pub has_next: bool,
    pub count: i64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Feed {
    Trending,
    Upcoming,
    Popular,
}

async fn decorate(state: &AppState, page: Page<RawgGame>) -> ApiResult<DiscoverPage> {
    let owned: HashMap<i64, GameId> = state.library.rawg_ids().await?.into_iter().collect();
    let results = page
        .results
        .into_iter()
        .map(|g| DiscoverGame {
            cover_key: g.background_image.as_deref().map(|u| state.images.register(u).0),
            library_id: owned.get(&g.id).copied(),
            game: g,
        })
        .collect();
    Ok(DiscoverPage { results, has_next: page.has_next, count: page.count })
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn discover_feed(state: State<'_, AppState>, feed: Feed, page: u32) -> ApiResult<DiscoverPage> {
    let ep = match feed {
        Feed::Trending => Endpoint::Trending { page },
        Feed::Upcoming => Endpoint::Upcoming { page },
        Feed::Popular => Endpoint::Popular { page },
    };
    let p = state.rawg.page(ep).await?;
    decorate(&state, p).await
}

// A new search aborts the one in flight (drop-cancellation, spec 9.3).
#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn discover_search(state: State<'_, AppState>, query: String, page: u32) -> ApiResult<DiscoverPage> {
    let rawg = state.rawg.clone();
    let q = query.chars().take(200).collect::<String>();
    let task = tokio::spawn(async move { rawg.search(&q, page).await });
    {
        let mut slot = match state.search.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(prev) = slot.replace(task.abort_handle()) {
            prev.abort();
        }
    }
    match task.await {
        Ok(r) => decorate(&state, r?).await,
        Err(e) if e.is_cancelled() => Err(ApiError::cancelled()),
        Err(e) => Err(ApiError::new("error", e.to_string())),
    }
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn rawg_details(state: State<'_, AppState>, id: i64) -> ApiResult<RawgGameDetail> {
    Ok(state.rawg.details(id).await?)
}

#[derive(Debug, Serialize)]
pub struct ExeMatch {
    pub title: String,
    pub candidates: Vec<DiscoverGame>,
    pub error: Option<ApiError>,
}

// Guess a title from the exe, then rank RAWG results for the cover picker.
#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn match_title(
    state: State<'_, AppState>,
    path: Option<String>,
    query: Option<String>,
) -> ApiResult<ExeMatch> {
    let title = match (query.filter(|q| !q.trim().is_empty()), path) {
        (Some(q), _) => q.trim().to_string(),
        (None, Some(p)) => title_from_exe_path(&Utf8PathBuf::from(p)),
        (None, None) => return Err(ApiError::new("invalid", "nothing to match")),
    };
    match state.rawg.search(&title, 1).await {
        Ok(page) => {
            let ranked = rank_matches(&title, &page.results).into_iter().take(8).map(|s| s.item).collect();
            let d = decorate(&state, Page { results: ranked, has_next: false, count: 0 }).await?;
            Ok(ExeMatch { title, candidates: d.results, error: None })
        }
        Err(e) => Ok(ExeMatch { title, candidates: vec![], error: Some(e.into()) }),
    }
}

fn draft_from_rawg(g: &RawgGame, status: Status) -> GameDraft {
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    GameDraft {
        title: g.name.clone(),
        rawg_id: Some(g.id),
        cover_url: g.background_image.clone(),
        release_date: g.released.as_deref().and_then(|d| time::Date::parse(d, &fmt).ok()),
        status,
        genres: g.genres.clone(),
        ..GameDraft::default()
    }
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn add_rawg_game(state: State<'_, AppState>, game: RawgGame, status: Option<Status>) -> ApiResult<GameId> {
    Ok(state.library.add_game(draft_from_rawg(&game, status.unwrap_or_default())).await?)
}

// Add a PC game from an exe, optionally with the RAWG match the user picked.
#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn add_pc_game(
    state: State<'_, AppState>,
    path: String,
    title: String,
    game: Option<RawgGame>,
) -> ApiResult<GameId> {
    let install =
        Install { launch: LaunchTarget::Exe { path: path.into(), args: vec![], working_dir: None }, detect_exe: None };
    let mut draft = match &game {
        Some(g) => draft_from_rawg(g, Status::Playing),
        None => GameDraft { title, status: Status::Playing, ..GameDraft::default() },
    };
    draft.install = Some(install);
    Ok(state.library.add_game(draft).await?)
}
