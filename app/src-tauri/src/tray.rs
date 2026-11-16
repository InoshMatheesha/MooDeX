// tray.rs - Tray icon: launch recent and installed games straight from the menu (like Steam), show, quit.

use crate::state::AppState;
use moodex_core::{GameFilter, GameId, SortOrder};
use moodex_library::GameView;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const TRAY_ID: &str = "moodex-tray";
const PLAY_PREFIX: &str = "play:";
// Recently played games shown at the top level, and installed games listed in the submenu.
const RECENT_MAX: usize = 6;
const ALL_MAX: usize = 40;
const TITLE_MAX: usize = 42;

// Bumped on every refresh request so a burst of library events rebuilds the menu once.
static GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let res = w.unminimize().and_then(|()| w.show()).and_then(|()| w.set_focus());
        if let Err(e) = res {
            tracing::warn!(error = %e, "could not show main window");
        }
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app, &[], &[])?;
    let mut b = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("MooDeX")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| on_menu(app, ev.id().as_ref()))
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.build(app)?;
    refresh_menu(app);
    Ok(())
}

fn on_menu(app: &AppHandle, id: &str) {
    match id {
        "show" | "more" => show_main(app),
        "quit" => app.exit(0),
        _ => {
            if let Some(game) = id.strip_prefix(PLAY_PREFIX).and_then(|s| GameId::from_str(s).ok()) {
                play(app, game);
            }
        }
    }
}

// Launch through the same tracked path as the Play button; failures surface in the window.
fn play(app: &AppHandle, id: GameId) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some(state) = app.try_state::<AppState>() else { return };
        if let Err(e) = state.player.play(id).await {
            tracing::warn!(game_id = %id, error = %e, "tray launch failed");
            show_main(&app);
            if let Err(err) = app.emit("tray-play-error", e.to_string()) {
                tracing::debug!(error = %err, "could not emit tray play error");
            }
        }
    });
}

// Menu labels: '&' would become a mnemonic, and very long titles would widen the menu.
fn label(title: &str) -> String {
    let mut t: String = title.chars().take(TITLE_MAX).collect();
    if title.chars().count() > TITLE_MAX {
        t = format!("{}...", t.trim_end());
    }
    t.replace('&', "&&")
}

fn game_item(app: &AppHandle, g: &GameView) -> tauri::Result<MenuItem<Wry>> {
    let running = g.running_since.is_some();
    let text = if running { format!("{}  (playing)", label(&g.title)) } else { label(&g.title) };
    MenuItem::with_id(app, format!("{PLAY_PREFIX}{}", g.id), text, !running, None::<&str>)
}

// Recent games on top, every installed game in a submenu, then Show and Quit.
fn build_menu(app: &AppHandle, recent: &[GameView], installed: &[GameView]) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    if installed.is_empty() {
        menu.append(&MenuItem::with_id(app, "none", "No installed games yet", false, None::<&str>)?)?;
    } else {
        if !recent.is_empty() {
            menu.append(&MenuItem::with_id(app, "recent", "Recent games", false, None::<&str>)?)?;
            for g in recent {
                menu.append(&game_item(app, g)?)?;
            }
            menu.append(&PredefinedMenuItem::separator(app)?)?;
        }
        let all = Submenu::with_id(app, "all", format!("Installed games ({})", installed.len()), true)?;
        for g in installed.iter().take(ALL_MAX) {
            all.append(&game_item(app, g)?)?;
        }
        if installed.len() > ALL_MAX {
            all.append(&PredefinedMenuItem::separator(app)?)?;
            all.append(&MenuItem::with_id(app, "more", "More in MooDeX...", true, None::<&str>)?)?;
        }
        menu.append(&all)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "show", "Show MooDeX", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit MooDeX", true, None::<&str>)?)?;
    Ok(menu)
}

// Rebuild the game list shortly after the library changes (debounced).
pub fn refresh_menu(app: &AppHandle) {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(400)).await;
        if GENERATION.load(Ordering::SeqCst) != generation {
            return;
        }
        let Some(state) = app.try_state::<AppState>() else { return };
        let filter = GameFilter { installed_only: true, ..GameFilter::default() };
        let installed = match state.library.list_games(filter, SortOrder::Title).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "tray: could not list installed games");
                return;
            }
        };
        let mut recent: Vec<GameView> = installed.iter().filter(|g| g.last_played.is_some()).cloned().collect();
        let when = |g: &GameView| g.last_played.as_deref().and_then(|s| OffsetDateTime::parse(s, &Rfc3339).ok());
        recent.sort_by_key(|g| std::cmp::Reverse(when(g)));
        recent.truncate(RECENT_MAX);
        let menu = match build_menu(&app, &recent, &installed) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "tray: could not build menu");
                return;
            }
        };
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            if let Err(e) = tray.set_menu(Some(menu)) {
                tracing::warn!(error = %e, "tray: could not set menu");
            }
        }
    });
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
