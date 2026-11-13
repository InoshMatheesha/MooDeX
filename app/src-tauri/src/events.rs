// events.rs - Push actor events to the webview; the frontend patches only what changed (spec 7.1).

use moodex_library::{LibraryEvent, LibraryHandle, ScannerHandle};
use moodex_rawg::RawgClient;
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast::error::RecvError;

pub fn bridge(app: AppHandle, library: &LibraryHandle, rawg: &RawgClient, scanner: &ScannerHandle) {
    let mut scans = scanner.subscribe();
    let scan_app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            match scans.recv().await {
                Ok(ev) => {
                    if let Err(e) = scan_app.emit("scan-event", &ev) {
                        tracing::debug!(error = %e, "could not emit scan event");
                    }
                }
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => break,
            }
        }
    });
    let mut rx = library.subscribe();
    let lib_app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(LibraryEvent::RegistryChanged) => {}
                Ok(ev) => {
                    if let Err(e) = lib_app.emit("library-event", &ev) {
                        tracing::warn!(error = %e, "could not emit library event");
                    }
                    if matches!(ev, LibraryEvent::SessionStarted { .. } | LibraryEvent::SessionEnded { .. }) {
                        crate::tray::refresh_tooltip(&lib_app);
                    }
                }
                // Missed some: tell the UI to re-hydrate rather than guess.
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!(n, "frontend event bridge lagged; requesting resync");
                    if let Err(e) = lib_app.emit("library-resync", ()) {
                        tracing::warn!(error = %e, "could not emit resync");
                    }
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
    let mut updates = rawg.subscribe_updates();
    tauri::async_runtime::spawn(async move {
        loop {
            match updates.recv().await {
                Ok(ep) => {
                    if let Err(e) = app.emit("discover-updated", &ep) {
                        tracing::debug!(error = %e, "could not emit discover update");
                    }
                }
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => break,
            }
        }
    });
}
