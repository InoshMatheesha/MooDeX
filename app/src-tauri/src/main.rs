// main.rs - MooDeX entry point: logging first, then services, then the window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // no console window in release

mod commands;
mod error;
mod events;
mod keys;
mod logging;
mod paths;
mod protocol;
mod state;
mod tray;

use crate::paths::AppDirs;
use crate::state::AppState;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_window_state::StateFlags;

fn main() {
    let dirs = match AppDirs::resolve() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("MooDeX cannot create its data folder: {e:#}");
            std::process::exit(1);
        }
    };
    let _log_guard = match logging::init_logging(&dirs.logs) {
        Ok(g) => Some(g),
        Err(e) => {
            eprintln!("logging unavailable: {e:#}");
            None
        }
    };
    tracing::info!(version = env!("CARGO_PKG_VERSION"), build = env!("MOODEX_BUILD_HASH"), data = %dirs.root, "MooDeX starting");
    if let Err(e) = run(dirs) {
        tracing::error!(error = %format!("{e:#}"), "MooDeX failed");
        std::process::exit(1);
    }
}

fn run(dirs: AppDirs) -> anyhow::Result<()> {
    let started_minimized = std::env::args().any(|a| a == "--minimized");
    let app = tauri::Builder::default()
        // Must be first: a second launch just focuses the running window.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| tray::show_main(app)))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .register_asynchronous_uri_scheme_protocol("moodex-img", |ctx, request, responder| {
            let Some(state) = ctx.app_handle().try_state::<AppState>() else {
                responder.respond(tauri::http::Response::builder().status(503).body(Vec::new()).unwrap_or_default());
                return;
            };
            let images = state.images.clone();
            tauri::async_runtime::spawn(async move { responder.respond(protocol::handle(images, request).await) });
        })
        .setup(move |app| {
            let (state, settings) = tauri::async_runtime::block_on(AppState::init(dirs))?;
            events::bridge(app.handle().clone(), &state.library, &state.rawg, &state.scanner);
            app.manage(state);
            if let Err(e) = tray::create(app.handle()) {
                tracing::warn!(error = %e, "tray icon unavailable");
            }
            commands::settings::sync_autostart(app.handle(), settings.launch_at_startup);
            let hide = started_minimized && settings.start_minimized;
            if !hide {
                tray::show_main(app.handle());
            }
            commands::update::spawn_startup_check(app.handle().clone());
            tracing::info!(minimized = hide, "MooDeX ready");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let to_tray = window.app_handle().try_state::<AppState>().is_some_and(|s| s.tray_enabled());
                if to_tray {
                    api.prevent_close();
                    if let Err(e) = window.hide() {
                        tracing::warn!(error = %e, "could not hide window");
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::library::list_ids,
            commands::library::get_games,
            commands::library::get_game,
            commands::library::add_game,
            commands::library::update_game,
            commands::library::delete_game,
            commands::library::set_install,
            commands::library::set_goat,
            commands::library::merge_games,
            commands::library::duplicates,
            commands::library::sessions,
            commands::library::dashboard,
            commands::library::running,
            commands::library::play_game,
            commands::library::scan_now,
            commands::discover::discover_feed,
            commands::discover::discover_search,
            commands::discover::rawg_details,
            commands::discover::match_title,
            commands::discover::add_rawg_game,
            commands::discover::add_pc_game,
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::set_rawg_key,
            commands::settings::test_rawg,
            commands::diagnostics::diagnostics,
            commands::diagnostics::open_folder,
            commands::diagnostics::open_link,
            commands::data::v1_status,
            commands::data::v1_plan,
            commands::data::v1_apply,
            commands::data::export_library,
            commands::data::import_library,
            commands::update::check_update,
            commands::update::install_update,
        ])
        .build(tauri::generate_context!())?;

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            // Close live sessions cleanly so the next start needs no recovery.
            if let Some(state) = handle.try_state::<AppState>() {
                let lib = state.library.clone();
                if let Err(e) = tauri::async_runtime::block_on(lib.shutdown()) {
                    tracing::warn!(error = %e, "library shutdown");
                }
            }
            tracing::info!("MooDeX exited");
        }
    });
    Ok(())
}
