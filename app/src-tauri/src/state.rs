// state.rs - Services constructed once at startup and injected into commands (spec 2 row 13).

use crate::keys;
use crate::paths::AppDirs;
use anyhow::{Context, Result};
use moodex_core::paths::DetectRegistry;
use moodex_images::ImageCache;
use moodex_library::{
    ActorCache, Deps, LibraryHandle, PlayTracker, Player, Scanner, ScannerHandle, Settings, TrackerHandle,
};
use moodex_rawg::{RawgClient, RawgConfig};
use moodex_store::Store;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;

pub struct AppState {
    pub library: LibraryHandle,
    pub player: Player,
    pub scanner: ScannerHandle,
    pub tracker: TrackerHandle,
    pub rawg: RawgClient,
    pub images: ImageCache,
    pub dirs: AppDirs,
    pub minimize_to_tray: Arc<AtomicBool>,
    pub search: Mutex<Option<tokio::task::AbortHandle>>,
}

impl AppState {
    pub async fn init(dirs: AppDirs) -> Result<(Self, Settings)> {
        let store = Store::open_rw(&dirs.db).context("opening the library database")?;
        let settings = Settings::from_stored(&store.settings().context("reading settings")?);
        let images = ImageCache::new(dirs.covers.clone(), settings.image_cache_budget_mb * 1024 * 1024)
            .map_err(|e| anyhow::anyhow!("image cache: {e}"))?;
        let registry = Arc::new(RwLock::new(DetectRegistry::new()));
        let deps =
            Deps { images: images.clone(), registry: Arc::clone(&registry), backup_dir: Some(dirs.backups.clone()) };
        let library = LibraryHandle::spawn(store, deps).await.map_err(|e| anyhow::anyhow!("library: {e}"))?;

        let rawg =
            RawgClient::new(RawgConfig::default(), Arc::new(ActorCache(library.clone())), keys::key_source(&settings))
                .map_err(|e| anyhow::anyhow!("rawg client: {e}"))?;

        // WMI can take seconds to come up; forward its events so startup never waits on it.
        let (fwd_tx, fwd_rx) = mpsc::channel(256);
        let reg = Arc::clone(&registry);
        tokio::task::spawn_blocking(move || match moodex_winproc::watch_processes(reg) {
            Ok(mut rx) => {
                tokio::spawn(async move {
                    while let Some(ev) = rx.recv().await {
                        if fwd_tx.send(ev).await.is_err() {
                            break;
                        }
                    }
                });
            }
            Err(e) => tracing::error!(error = %e, "process watcher could not start; only launched games are tracked"),
        });
        let tracker = PlayTracker::spawn(
            library.clone(),
            registry,
            Some(fwd_rx),
            Duration::from_secs(settings.stub_threshold_seconds),
        );
        let player = Player::new(library.clone(), tracker.clone());
        // Watches the user's game folders and adds new installs.
        let scanner = Scanner::spawn(library.clone(), rawg.clone());
        let state = Self {
            library,
            player,
            scanner,
            tracker,
            rawg,
            images,
            dirs,
            minimize_to_tray: Arc::new(AtomicBool::new(settings.minimize_to_tray)),
            search: Mutex::new(None),
        };
        Ok((state, settings))
    }

    pub fn tray_enabled(&self) -> bool {
        self.minimize_to_tray.load(Ordering::Relaxed)
    }
}
