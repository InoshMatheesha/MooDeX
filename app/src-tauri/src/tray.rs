// tray.rs - Tray icon: left click toggles the window; the menu launches games (with their icons), like Steam.

use crate::state::AppState;
use crate::tray_icons::{self, Source};
use moodex_core::{ContentHash, GameFilter, GameId, SortOrder};
use moodex_images::Variant;
use moodex_library::GameView;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::image::Image;
use tauri::menu::{IconMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
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
// Clicking the tray takes focus from the window first; a blur this recent still counts as "in front".
const BLUR_GRACE_MS: u64 = 600;

// Bumped on every refresh request so a burst of library events rebuilds the menu once.
static GENERATION: AtomicU64 = AtomicU64::new(0);
static LAST_BLUR_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let res = w.unminimize().and_then(|()| w.show()).and_then(|()| w.set_focus());
        if let Err(e) = res {
            tracing::warn!(error = %e, "could not show main window");
        }
    }
}

// Called when the main window loses focus.
pub fn note_focus_lost() {
    LAST_BLUR_MS.store(now_ms(), Ordering::Relaxed);
}

// Left click: hide the window when it is open in front, otherwise bring it up.
fn toggle_main(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else { return };
    let shown = w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false);
    let recent_blur = now_ms().saturating_sub(LAST_BLUR_MS.load(Ordering::Relaxed)) < BLUR_GRACE_MS;
    let in_front = w.is_focused().unwrap_or(false) || recent_blur;
    if shown && in_front {
        if let Err(e) = w.hide() {
            tracing::warn!(error = %e, "could not hide main window");
        }
    } else {
        show_main(app);
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app, &[], &[], 0, &[])?;
    let mut b = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("MooDeX")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| on_menu(app, ev.id().as_ref()))
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                toggle_main(tray.app_handle());
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

// A game with its pre-rendered 16 px icon.
pub struct Entry {
    view: GameView,
    icon: Vec<u8>,
}

fn game_item(app: &AppHandle, e: &Entry) -> tauri::Result<IconMenuItem<Wry>> {
    let running = e.view.running_since.is_some();
    let text = if running { format!("{}  -  playing", label(&e.view.title)) } else { label(&e.view.title) };
    let icon = Image::new_owned(e.icon.clone(), tray_icons::SIZE, tray_icons::SIZE);
    IconMenuItem::with_id(app, format!("{PLAY_PREFIX}{}", e.view.id), text, !running, Some(icon), None::<&str>)
}

// Recent games on top, installed games (up to ALL_MAX of `total`) in a submenu, then Show and Quit.
fn build_menu(
    app: &AppHandle,
    recent: &[Entry],
    shown: &[Entry],
    total: usize,
    app_icon: &[u8],
) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    if shown.is_empty() {
        menu.append(&MenuItem::with_id(app, "none", "No installed games yet", false, None::<&str>)?)?;
    } else {
        if !recent.is_empty() {
            menu.append(&MenuItem::with_id(app, "recent", "Recent games", false, None::<&str>)?)?;
            for e in recent {
                menu.append(&game_item(app, e)?)?;
            }
            menu.append(&PredefinedMenuItem::separator(app)?)?;
        }
        let all = Submenu::with_id(app, "all", format!("Installed games ({total})"), true)?;
        for e in shown {
            all.append(&game_item(app, e)?)?;
        }
        if total > shown.len() {
            all.append(&PredefinedMenuItem::separator(app)?)?;
            all.append(&MenuItem::with_id(app, "more", "More in MooDeX...", true, None::<&str>)?)?;
        }
        menu.append(&all)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    let show_icon =
        (!app_icon.is_empty()).then(|| Image::new_owned(app_icon.to_vec(), tray_icons::SIZE, tray_icons::SIZE));
    menu.append(&IconMenuItem::with_id(app, "show", "Show MooDeX", true, show_icon, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit MooDeX", true, None::<&str>)?)?;
    Ok(menu)
}

// Everything the icon renderer may draw from for one game; the cover is read only if already cached.
async fn source_for(state: &AppState, g: &GameView) -> Source {
    let exe = g.install.as_ref().and_then(|i| i.effective_detect().cloned()).filter(|p| p.exists());
    let key = format!("{}|{}|{}", g.id, exe.as_ref().map_or("", |p| p.as_str()), g.cover_key.as_deref().unwrap_or(""));
    let mut cover = None;
    if exe.is_none() && !tray_icons::is_cached(&key) {
        if let Some(k) = &g.cover_key {
            if state.images.is_cached(&ContentHash(k.clone())) {
                cover = state.images.load(k, Variant::Thumb).await.ok().map(|b| b.to_vec());
            }
        }
    }
    Source { key, title: g.title.clone(), exe, cover }
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

        // Icons for every game the menu shows, rendered off the async runtime.
        let mut wanted: Vec<GameView> = recent.clone();
        let seen: HashSet<GameId> = recent.iter().map(|g| g.id).collect();
        wanted.extend(installed.iter().take(ALL_MAX).filter(|g| !seen.contains(&g.id)).cloned());
        let mut sources = Vec::with_capacity(wanted.len());
        for g in &wanted {
            sources.push((source_for(&state, g).await, g.running_since.is_some()));
        }
        let app_icon = app.default_window_icon().map(|i| (i.rgba().to_vec(), i.width(), i.height()));
        let rendered = tauri::async_runtime::spawn_blocking(move || {
            let icons: Vec<Vec<u8>> = sources.iter().map(|(s, running)| tray_icons::render(s, *running)).collect();
            let app_icon = app_icon.and_then(|(px, w, h)| tray_icons::app_icon(&px, w, h)).unwrap_or_default();
            (icons, app_icon)
        })
        .await;
        let Ok((icons, app_icon)) = rendered else {
            tracing::warn!("tray: icon rendering failed");
            return;
        };
        let by_id: std::collections::HashMap<GameId, Vec<u8>> = wanted.iter().map(|g| g.id).zip(icons).collect();
        let entry = |g: &GameView| Entry { view: g.clone(), icon: by_id.get(&g.id).cloned().unwrap_or_default() };
        let recent: Vec<Entry> = recent.iter().map(entry).collect();
        let all: Vec<Entry> = installed.iter().take(ALL_MAX).map(entry).collect();
        let total = installed.len();

        let menu = match build_menu(&app, &recent, &all, total, &app_icon) {
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
