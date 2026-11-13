// data.rs - v1 import (plan -> review -> apply) and JSON export/import (spec 15).

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use camino::Utf8PathBuf;
use moodex_library::{plan_import, ExportDoc, ImportPlan, ImportReport};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct V1Status {
    pub path: Option<String>,
    pub already_imported: bool,
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn v1_status(state: State<'_, AppState>) -> ApiResult<V1Status> {
    let s = state.library.settings().await?;
    Ok(V1Status { path: state.dirs.v1_file().map(|p| p.to_string()), already_imported: s.v1_import_done })
}

// Read-only: the user reviews this before anything is written.
#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn v1_plan(state: State<'_, AppState>, path: Option<String>) -> ApiResult<ImportPlan> {
    let p = match path {
        Some(p) => Utf8PathBuf::from(p),
        None => state.dirs.v1_file().ok_or_else(|| ApiError::new("not_found", "no MooDeX v1 library was found"))?,
    };
    tokio::task::spawn_blocking(move || plan_import(&p))
        .await
        .map_err(|e| ApiError::new("error", e.to_string()))?
        .map_err(Into::into)
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn v1_apply(
    state: State<'_, AppState>,
    plan: ImportPlan,
    accepted: Vec<(String, String)>,
) -> ApiResult<ImportReport> {
    Ok(state.library.import_v1(plan, accepted).await?)
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn export_library(state: State<'_, AppState>, path: String) -> ApiResult<usize> {
    let doc = state.library.export_doc().await?;
    let n = doc.games.len();
    let bytes = serde_json::to_vec_pretty(&doc).map_err(|e| ApiError::new("error", e.to_string()))?;
    let dest = Utf8PathBuf::from(path);
    tokio::task::spawn_blocking(move || moodex_store::write_atomic(&dest, &bytes))
        .await
        .map_err(|e| ApiError::new("error", e.to_string()))?
        .map_err(|e| ApiError::new("io", e.to_string()))?;
    Ok(n)
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn import_library(state: State<'_, AppState>, path: String) -> ApiResult<ImportReport> {
    let text = tokio::fs::read_to_string(&path).await.map_err(|e| ApiError::new("io", e.to_string()))?;
    let doc: ExportDoc =
        serde_json::from_str(&text).map_err(|e| ApiError::new("import", format!("not a MooDeX export: {e}")))?;
    Ok(state.library.import_doc(doc).await?)
}
