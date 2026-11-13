//! moodex-winproc - Windows process tracking: job objects, WMI events, launching (spec 8).
// The only crate allowed `unsafe`; every block carries a SAFETY comment (spec 8.6).
#![allow(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))] // tests may unwrap (spec 4.2)

mod canon;
mod cmdline;
mod handle;
mod job;
mod launch;
mod scan;
mod watch;

pub use canon::{canonicalize, is_process_alive, process_image_path};
pub use cmdline::build_command_line;
pub use handle::OwnedHandle;
pub use job::JobObject;
pub use launch::{launch_tracked, ExitReason, LaunchError, LaunchMethod, TrackedLaunch};
pub use scan::scan_running;
pub use watch::{match_process, watch_processes, ProcEvent, ProcEventKind, SharedRegistry, WatchError};
