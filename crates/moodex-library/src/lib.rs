//! moodex-library - the actor that solely owns library state (spec 7), plus play tracking glue.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))] // tests may unwrap (spec 4.2)

mod actor;
mod cache_bridge;
mod error;
mod import;
mod player;
mod scanner;
mod settings;
mod tracker;
mod view;

pub use actor::{Deps, LibraryEvent, LibraryHandle, PlayEvent};
pub use cache_bridge::ActorCache;
pub use error::LibraryError;
pub use import::{
    plan_from_str, plan_import, ExportDoc, ImportGame, ImportPlan, ImportReport, ImportWarning, ProposedMerge,
};
pub use player::{PlayError, PlayOutcome, Player};
pub use scanner::{game_folder_for, game_folders, ScanEvent, ScanReport, Scanner, ScannerHandle};
pub use settings::Settings;
pub use tracker::{PlayTracker, TrackerHandle};
pub use view::{DashboardView, GameView};

// Truncate to millisecond precision so memory and SQLite agree exactly.
pub fn now_ms() -> time::OffsetDateTime {
    let t = time::OffsetDateTime::now_utc();
    t.replace_nanosecond(t.nanosecond() / 1_000_000 * 1_000_000).unwrap_or(t)
}
