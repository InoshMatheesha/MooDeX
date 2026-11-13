// logging.rs - Rolling file log, panic hook, and recent warnings for Diagnostics (spec 11).

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use std::io::Write;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

const DEFAULT_FILTER: &str = if cfg!(debug_assertions) { "info,moodex=debug" } else { "info" };

// Daily file `moodex.YYYY-MM-DD.log`, seven kept; RUST_LOG honoured.
pub fn init_logging(dir: &Utf8Path) -> Result<WorkerGuard> {
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("moodex")
        .filename_suffix("log")
        .max_log_files(7)
        .build(dir.as_std_path())
        .context("creating log appender")?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(writer).with_ansi(false).with_target(true))
        .try_init()
        .context("installing tracing subscriber")?;
    install_panic_hook(dir.to_owned());
    Ok(guard)
}

// Panics are written synchronously before the abort (release uses panic = "abort").
fn install_panic_hook(dir: Utf8PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        let msg =
            format!("PANIC in MooDeX {} ({}): {info}\n{bt}\n", env!("CARGO_PKG_VERSION"), env!("MOODEX_BUILD_HASH"));
        tracing::error!("{msg}");
        let crash = dir.join("crash.log");
        let res = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crash)
            .and_then(|mut f| f.write_all(msg.as_bytes()));
        if res.is_err() {
            eprintln!("{msg}");
        }
        previous(info);
    }));
}

// Last N WARN/ERROR lines across the newest log files.
pub fn recent_problems(dir: &Utf8Path, n: usize) -> Vec<String> {
    let mut files: Vec<Utf8PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
                .filter(|p| p.file_name().is_some_and(|f| f.starts_with("moodex.") && f.ends_with(".log")))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let mut out: Vec<String> = Vec::new();
    for f in files.iter().rev().take(2) {
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        let mut lines: Vec<String> =
            text.lines().filter(|l| l.contains(" WARN ") || l.contains(" ERROR ")).map(str::to_string).collect();
        lines.append(&mut out);
        out = lines;
        if out.len() >= n {
            break;
        }
    }
    let skip = out.len().saturating_sub(n);
    out.into_iter().skip(skip).collect()
}
