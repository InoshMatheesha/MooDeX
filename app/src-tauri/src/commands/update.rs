// update.rs - In-app updates from GitHub Releases; every package is minisign-verified before install (spec 13.7).

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current: String,
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

fn update_err(e: impl std::fmt::Display) -> ApiError {
    let m = e.to_string();
    let friendly = if m.to_lowercase().contains("signature") {
        "The update's signature did not verify, so it was not installed.".to_string()
    } else if m.contains("error sending request") || m.contains("dns") || m.contains("connect") {
        "Could not reach GitHub - check your internet connection.".to_string()
    } else {
        format!("Update failed: {m}")
    };
    ApiError::new("update", friendly)
}

// Ask the release feed whether a newer version exists.
pub async fn check(app: &AppHandle) -> ApiResult<UpdateInfo> {
    let current = app.package_info().version.to_string();
    let update = app.updater().map_err(update_err)?.check().await.map_err(update_err)?;
    Ok(match update {
        Some(u) => UpdateInfo {
            current,
            available: true,
            version: Some(u.version.clone()),
            notes: u.body.clone(),
            date: u.date.map(|d| d.to_string()),
        },
        None => UpdateInfo { current, available: false, version: None, notes: None, date: None },
    })
}

#[tracing::instrument(level = "info", skip_all)]
#[tauri::command]
pub async fn check_update(app: AppHandle) -> ApiResult<UpdateInfo> {
    check(&app).await
}

// Download (with progress events), close sessions cleanly, then run the installer.
#[tracing::instrument(level = "info", skip_all)]
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> ApiResult<()> {
    let Some(update) = app.updater().map_err(update_err)?.check().await.map_err(update_err)? else {
        return Err(ApiError::new("update", "You're already on the latest version."));
    };
    tracing::info!(version = %update.version, "downloading update");
    let progress_app = app.clone();
    let mut downloaded: u64 = 0;
    let bytes = update
        .download(
            move |chunk, total| {
                downloaded += chunk as u64;
                if let Err(e) = progress_app.emit("update-progress", UpdateProgress { downloaded, total }) {
                    tracing::debug!(error = %e, "progress event");
                }
            },
            || tracing::info!("update downloaded and verified"),
        )
        .await
        .map_err(update_err)?;
    // Live play sessions are closed before the installer replaces the app.
    if let Err(e) = state.library.shutdown().await {
        tracing::warn!(error = %e, "library shutdown before update");
    }
    update.install(bytes).map_err(update_err)?;
    // Windows exits inside install(); elsewhere, restart into the new version.
    app.restart();
}

// Quiet check at startup; tells the UI only when something new exists.
pub fn spawn_startup_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(12)).await;
        let enabled = match app.try_state::<AppState>() {
            Some(s) => s.library.settings().await.map(|s| s.auto_update_check).unwrap_or(true),
            None => return,
        };
        if !enabled {
            return;
        }
        match check(&app).await {
            Ok(info) if info.available => {
                tracing::info!(version = ?info.version, "update available");
                if let Err(e) = app.emit("update-available", &info) {
                    tracing::debug!(error = %e, "update event");
                }
            }
            Ok(_) => tracing::info!("MooDeX is up to date"),
            Err(e) => tracing::info!(error = %e.message, "update check skipped"),
        }
    });
}
