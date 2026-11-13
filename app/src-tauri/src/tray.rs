// tray.rs - Tray icon: show/hide, quit, and a tooltip naming what is being played.

use crate::state::AppState;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const TRAY_ID: &str = "moodex-tray";

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let res = w.unminimize().and_then(|()| w.show()).and_then(|()| w.set_focus());
        if let Err(e) = res {
            tracing::warn!(error = %e, "could not show main window");
        }
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show MooDeX", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit MooDeX", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &sep, &quit])?;
    let mut b = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("MooDeX")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "show" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.build(app)?;
    Ok(())
}

// "MooDeX - playing Hades" while a session is open.
pub fn refresh_tooltip(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some(state) = app.try_state::<AppState>() else { return };
        let running = state.library.running().await.unwrap_or_default();
        let mut names = Vec::new();
        for id in running {
            if let Ok(Some(g)) = state.library.get_game(id).await {
                names.push(g.title);
            }
        }
        let tip =
            if names.is_empty() { "MooDeX".to_string() } else { format!("MooDeX - playing {}", names.join(", ")) };
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            if let Err(e) = tray.set_tooltip(Some(tip)) {
                tracing::debug!(error = %e, "tray tooltip");
            }
        }
    });
}
