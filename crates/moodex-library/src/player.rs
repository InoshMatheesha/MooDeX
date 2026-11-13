// player.rs - The Play button (spec 8.5): typed errors, tracked launch, hand-off to the tracker.

use crate::{LibraryError, LibraryHandle, TrackerHandle};
use camino::Utf8PathBuf;
use moodex_core::{GameId, LaunchTarget};
use moodex_winproc::{launch_tracked, LaunchError, LaunchMethod};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum PlayError {
    #[error("game not found")]
    NotFound,
    #[error("this game is not installed - link an executable first")]
    NotInstalled,
    #[error("executable not found: {path}")]
    ExecutableMissing { path: Utf8PathBuf },
    #[error("the game is already running")]
    AlreadyRunning,
    #[error("launch cancelled")]
    Cancelled,
    #[error("{0}")]
    Launch(String),
    #[error("{0}")]
    Library(String),
}

impl From<LibraryError> for PlayError {
    fn from(e: LibraryError) -> Self {
        PlayError::Library(e.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayOutcome {
    pub tracked: bool,
    pub method: String,
    pub note: Option<String>,
}

#[derive(Clone)]
pub struct Player {
    library: LibraryHandle,
    tracker: TrackerHandle,
}

impl Player {
    pub fn new(library: LibraryHandle, tracker: TrackerHandle) -> Self {
        Self { library, tracker }
    }

    pub async fn play(&self, id: GameId) -> Result<PlayOutcome, PlayError> {
        let span = tracing::info_span!("play", game_id = %id);
        tracing::Instrument::instrument(self.play_inner(id), span).await
    }

    async fn play_inner(&self, id: GameId) -> Result<PlayOutcome, PlayError> {
        let game = self.library.get_raw(id).await?.ok_or(PlayError::NotFound)?;
        let install = game.install.clone().ok_or(PlayError::NotInstalled)?;
        if self.library.running().await?.contains(&id) {
            return Err(PlayError::AlreadyRunning);
        }
        if let LaunchTarget::Exe { path, .. } | LaunchTarget::Shortcut { path } = &install.launch {
            let ok = std::fs::metadata(path).map(|m| m.is_file()).unwrap_or(false);
            if !ok {
                self.library.mark_missing(id, true).await?;
                return Err(PlayError::ExecutableMissing { path: path.clone() });
            }
        }
        self.tracker.launching(id).await;
        let launched = launch_tracked(&install.launch).await;
        if launched.is_err() {
            self.tracker.launch_failed(id).await;
        }
        let launch = match launched {
            Ok(l) => l,
            Err(LaunchError::ExecutableMissing { path }) | Err(LaunchError::NotAFile { path }) => {
                self.library.mark_missing(id, true).await?;
                return Err(PlayError::ExecutableMissing { path });
            }
            Err(LaunchError::Cancelled) => return Err(PlayError::Cancelled),
            Err(e) => return Err(PlayError::Launch(e.to_string())),
        };
        let has_detect = install.effective_detect().is_some();
        // Only a separate detect exe can outlive the launched tree (launcher stubs).
        let promotable = install.detect_exe.is_some();
        let method = launch.method;
        tracing::info!(?method, root_pid = ?launch.root_pid, has_detect, "game launched");
        self.tracker.launched(id, launch, promotable).await;
        self.library.mark_missing(id, false).await?;
        let tracked = method != LaunchMethod::Untracked || has_detect;
        let note = (!tracked).then(|| {
            "Launched, but playtime cannot be tracked: set the game's executable under Detection.".to_string()
        });
        Ok(PlayOutcome { tracked, method: format!("{method:?}"), note })
    }
}
