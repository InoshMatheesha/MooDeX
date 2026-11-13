// library.rs - Library and play commands.

use crate::error::ApiResult;
use crate::state::AppState;
use moodex_core::{
    DuplicateCandidate, GameDraft, GameFilter, GameId, GamePatch, Install, LaunchTarget, SessionRecord, SortOrder,
};
use moodex_library::{game_folder_for, DashboardView, GameView, PlayOutcome, ScanReport};
use tauri::State;

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn list_ids(state: State<'_, AppState>, filter: GameFilter, sort: SortOrder) -> ApiResult<Vec<GameId>> {
    Ok(state.library.list_ids(filter, sort).await?)
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn get_games(state: State<'_, AppState>, ids: Vec<GameId>) -> ApiResult<Vec<GameView>> {
    Ok(state.library.get_games(ids).await?)
}

#[tracing::instrument(level = "debug", skip_all, fields(game_id = %id))]
#[tauri::command]
pub async fn get_game(state: State<'_, AppState>, id: GameId) -> ApiResult<Option<GameView>> {
    Ok(state.library.get_game(id).await?)
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn add_game(state: State<'_, AppState>, draft: GameDraft) -> ApiResult<GameId> {
    Ok(state.library.add_game(draft).await?)
}

#[tracing::instrument(level = "debug", skip_all, fields(game_id = %id))]
#[tauri::command]
pub async fn update_game(state: State<'_, AppState>, id: GameId, patch: GamePatch) -> ApiResult<()> {
    Ok(state.library.update_game(id, patch).await?)
}

#[tracing::instrument(level = "debug", skip_all, fields(game_id = %id))]
#[tauri::command]
pub async fn delete_game(state: State<'_, AppState>, id: GameId) -> ApiResult<()> {
    let install_path = state.library.get_raw(id).await?.and_then(|g| g.install).and_then(|i| match i.launch {
        LaunchTarget::Exe { path, .. } | LaunchTarget::Shortcut { path } => Some(path),
        LaunchTarget::Uri { .. } => i.detect_exe,
    });
    state.library.delete_game(id).await?;
    // A deleted auto-detected game must not come back on the next scan.
    if let Some(path) = install_path {
        let s = state.library.settings().await?;
        if let Some(folder) = game_folder_for(&path, &s.scan_folders) {
            let mut ignored = s.scan_ignored.clone();
            ignored.push(folder.to_string());
            state.library.set_setting("scan_ignored".into(), serde_json::json!(ignored)).await?;
        }
    }
    Ok(())
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn scan_now(state: State<'_, AppState>) -> ApiResult<ScanReport> {
    state
        .scanner
        .scan_now()
        .await
        .ok_or_else(|| crate::error::ApiError::new("error", "the game scanner is not running"))
}

#[tracing::instrument(level = "debug", skip_all, fields(game_id = %id))]
#[tauri::command]
pub async fn set_install(state: State<'_, AppState>, id: GameId, install: Option<Install>) -> ApiResult<()> {
    Ok(state.library.set_install(id, install).await?)
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn set_goat(state: State<'_, AppState>, id: Option<GameId>) -> ApiResult<()> {
    Ok(state.library.set_goat(id).await?)
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn merge_games(state: State<'_, AppState>, keep: GameId, absorb: GameId) -> ApiResult<()> {
    Ok(state.library.merge_games(keep, absorb).await?)
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn duplicates(state: State<'_, AppState>) -> ApiResult<Vec<DuplicateCandidate>> {
    Ok(state.library.duplicates().await?)
}

#[tracing::instrument(level = "debug", skip_all, fields(game_id = %id))]
#[tauri::command]
pub async fn sessions(state: State<'_, AppState>, id: GameId) -> ApiResult<Vec<SessionRecord>> {
    Ok(state.library.sessions(id).await?)
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn dashboard(state: State<'_, AppState>) -> ApiResult<DashboardView> {
    Ok(state.library.dashboard().await?)
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn running(state: State<'_, AppState>) -> ApiResult<Vec<GameId>> {
    Ok(state.library.running().await?)
}

#[tracing::instrument(level = "debug", skip_all, fields(game_id = %id))]
#[tauri::command]
pub async fn play_game(state: State<'_, AppState>, id: GameId) -> ApiResult<PlayOutcome> {
    Ok(state.player.play(id).await?)
}
