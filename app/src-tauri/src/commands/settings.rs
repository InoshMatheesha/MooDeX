// settings.rs - Settings with their side effects (autostart, RAWG route, tracker threshold).

use crate::error::{ApiError, ApiResult};
use crate::keys;
use crate::state::AppState;
use moodex_library::Settings;
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

#[derive(Debug, Serialize)]
pub struct SettingsView {
    pub settings: Settings,
    pub rawg_key_set: bool,
    pub rawg_active: String,
    pub autostart_registered: bool,
}

fn view(app: &AppHandle, state: &AppState, settings: Settings) -> SettingsView {
    SettingsView {
        rawg_key_set: keys::get_user_key().is_some(),
        rawg_active: state.rawg.source_kind().to_string(),
        autostart_registered: app.autolaunch().is_enabled().unwrap_or(false),
        settings,
    }
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn get_settings(app: AppHandle, state: State<'_, AppState>) -> ApiResult<SettingsView> {
    let s = state.library.settings().await?;
    Ok(view(&app, &state, s))
}

// Keep OS registration in line with the stored preference.
pub fn sync_autostart(app: &AppHandle, want: bool) {
    let al = app.autolaunch();
    let have = al.is_enabled().unwrap_or(false);
    let res = match (want, have) {
        (true, false) => al.enable(),
        (false, true) => al.disable(),
        _ => Ok(()),
    };
    if let Err(e) = res {
        tracing::warn!(error = %e, want, "could not update autostart registration");
    }
}

#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn set_setting(
    app: AppHandle,
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> ApiResult<SettingsView> {
    let s = state.library.set_setting(key.clone(), value).await?;
    match key.as_str() {
        "launch_at_startup" => sync_autostart(&app, s.launch_at_startup),
        "minimize_to_tray" => state.minimize_to_tray.store(s.minimize_to_tray, Ordering::Relaxed),
        "rawg_key_source" | "rawg_proxy_url" => state.rawg.set_source(keys::key_source(&s)),
        "stub_threshold_seconds" => {
            state.tracker.set_stub_threshold(Duration::from_secs(s.stub_threshold_seconds)).await
        }
        _ => {}
    }
    Ok(view(&app, &state, s))
}

// Store or clear the personal key in Credential Manager.
#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn set_rawg_key(app: AppHandle, state: State<'_, AppState>, key: Option<String>) -> ApiResult<SettingsView> {
    keys::set_user_key(key.as_deref()).map_err(|e| ApiError::new("credential", e))?;
    let mut s = state.library.settings().await?;
    if key.as_deref().is_some_and(|k| !k.trim().is_empty()) && s.rawg_key_source != "user" {
        s = state.library.set_setting("rawg_key_source".into(), serde_json::json!("user")).await?;
    }
    state.rawg.set_source(keys::key_source(&s));
    Ok(view(&app, &state, s))
}

// One cheap request so the user knows the route works.
#[tracing::instrument(level = "debug", skip_all)]
#[tauri::command]
pub async fn test_rawg(state: State<'_, AppState>) -> ApiResult<usize> {
    Ok(state.rawg.popular(1).await?.results.len())
}
