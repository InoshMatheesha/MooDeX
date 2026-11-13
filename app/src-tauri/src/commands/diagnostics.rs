// diagnostics.rs - Version, build hash, folders and recent problems (spec 11).

use crate::error::{ApiError, ApiResult};
use crate::logging::recent_problems;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

#[derive(Debug, Serialize)]
pub struct Diagnostics {
    pub version: String,
    pub build_hash: String,
    pub webview: String,
    pub data_dir: String,
    pub log_dir: String,
    pub games: usize,
    pub installed: usize,
    pub open_sessions: usize,
    pub schema_version: i64,
    pub detect_registry: usize,
    pub recent_problems: Vec<String>,
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn diagnostics(state: State<'_, AppState>) -> ApiResult<Diagnostics> {
    let d = state.library.diagnostics().await?;
    let logs = state.dirs.logs.clone();
    let problems = tokio::task::spawn_blocking(move || recent_problems(&logs, 10)).await.unwrap_or_default();
    Ok(Diagnostics {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_hash: env!("MOODEX_BUILD_HASH").to_string(),
        webview: tauri::webview_version().unwrap_or_else(|_| "unknown".into()),
        data_dir: state.dirs.root.to_string(),
        log_dir: state.dirs.logs.to_string(),
        games: d.games,
        installed: d.installed,
        open_sessions: d.open_sessions,
        schema_version: d.schema_version,
        detect_registry: d.detect_registry,
        recent_problems: problems,
    })
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn open_folder(app: tauri::AppHandle, state: State<'_, AppState>, which: String) -> ApiResult<()> {
    let dir = match which.as_str() {
        "logs" => state.dirs.logs.clone(),
        "data" => state.dirs.root.clone(),
        "backups" => state.dirs.backups.clone(),
        _ => return Err(ApiError::new("invalid", "unknown folder")),
    };
    app.opener().open_path(dir.as_str(), None::<&str>).map_err(|e| ApiError::new("io", e.to_string()))
}
