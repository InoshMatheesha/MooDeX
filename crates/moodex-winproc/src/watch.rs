// watch.rs - WMI intrinsic process events for games we did not start (spec 8.4 layers B/C).

use crate::canon::{canonicalize, process_image_path};
use crate::scan::registered_processes;
use camino::Utf8Path;
use moodex_core::paths::DetectRegistry;
use moodex_core::GameId;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::mpsc;

// Read by watcher threads, replaced by the library actor when installs change.
pub type SharedRegistry = Arc<RwLock<DetectRegistry>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcEventKind {
    Started,
    Exited,
}

// One process creation or deletion, as reported by WMI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcEvent {
    pub kind: ProcEventKind,
    pub pid: u32,
    pub name: String,
    pub exe_path: Option<String>,
}

#[derive(Debug, Error)]
pub enum WatchError {
    #[error("WMI subscription failed: {0}")]
    Subscribe(String),
    #[error("watcher thread failed to start: {0}")]
    Thread(String),
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Process")]
#[serde(rename_all = "PascalCase")]
struct Win32Process {
    process_id: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    executable_path: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct InstanceEvent {
    #[serde(rename = "__CLASS")]
    class: String,
    target_instance: Win32Process,
}

// The matching rule (spec 1.6): canonical full-path equality only.
pub fn match_process(ev: &ProcEvent, reg: &DetectRegistry) -> Option<GameId> {
    let raw = match &ev.exe_path {
        Some(p) if !p.trim().is_empty() => p.clone(),
        _ if ev.kind == ProcEventKind::Started => process_image_path(ev.pid)?,
        _ => return None,
    };
    let canon = canonicalize(Utf8Path::new(&raw)).map(|p| p.into_string()).unwrap_or(raw);
    reg.match_path(&canon)
}

// One subscription for creation + deletion; intrinsic events need no admin (unlike Win32_ProcessStartTrace).
const QUERY: &str = concat!(
    "SELECT * FROM __InstanceOperationEvent WITHIN 2 WHERE TargetInstance ISA 'Win32_Process' ",
    "AND (__CLASS = '__InstanceCreationEvent' OR __CLASS = '__InstanceDeletionEvent')"
);

// While WMI refuses us (its polling quota is shared machine-wide), diff cheap snapshots instead.
const FALLBACK_TICK: Duration = Duration::from_secs(2);
const WMI_RETRY: Duration = Duration::from_secs(60);

// Subscribe; failure is never fatal - the watcher degrades to snapshots and keeps retrying WMI.
pub fn watch_processes(registry: SharedRegistry) -> Result<mpsc::Receiver<ProcEvent>, WatchError> {
    let (tx, rx) = mpsc::channel(256);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("moodex-wmi".into())
        .spawn(move || watch_loop(registry, tx, ready_tx))
        .map_err(|e| WatchError::Thread(e.to_string()))?;
    match ready_rx.recv_timeout(Duration::from_secs(20)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!(error = %e, "WMI process events unavailable; using snapshot fallback"),
        Err(_) => tracing::warn!("WMI slow to start; continuing in the background"),
    }
    Ok(rx)
}

// One long-lived COM apartment on a dedicated thread (spec 8.6).
fn watch_loop(reg: SharedRegistry, tx: mpsc::Sender<ProcEvent>, ready: std::sync::mpsc::Sender<Result<(), String>>) {
    let mut ready = Some(ready);
    let mut seen: HashMap<u32, String> = HashMap::new();
    loop {
        if tx.is_closed() {
            return;
        }
        // The connection must outlive its notification iterator.
        let conn = wmi::WMIConnection::new().map_err(|e| e.to_string());
        let sub = conn
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|c| c.raw_notification::<InstanceEvent>(QUERY).map_err(|e| e.to_string()));
        let iter = match sub {
            Ok(it) => it,
            Err(e) => {
                match ready.take() {
                    Some(r) => report(r, Err(e)),
                    None => tracing::debug!(error = %e, "WMI still unavailable"),
                }
                if !fallback_until(&reg, &tx, &mut seen, Instant::now() + WMI_RETRY) {
                    return;
                }
                continue;
            }
        };
        if let Some(r) = ready.take() {
            report(r, Ok(()));
        }
        tracing::info!("WMI process subscription active");
        // Catch up on anything that started or stopped while we were not subscribed.
        if !emit_snapshot_diff(&reg, &tx, &mut seen) {
            return;
        }
        seen.clear();
        for item in iter {
            let ev = match item {
                Ok(e) => e,
                Err(e) => {
                    tracing::debug!(error = %e, "undecodable WMI event");
                    continue;
                }
            };
            let kind = match ev.class.as_str() {
                "__InstanceCreationEvent" => ProcEventKind::Started,
                "__InstanceDeletionEvent" => ProcEventKind::Exited,
                _ => continue,
            };
            let p = ev.target_instance;
            // Cheap exact file-name pre-filter; the real decision is match_process.
            let interesting = match reg.read() {
                Ok(r) => r.contains_file_name(&p.name),
                Err(_) => true,
            };
            if !interesting {
                continue;
            }
            tracing::debug!(?kind, pid = p.process_id, name = %p.name, "registered process event");
            let out = ProcEvent { kind, pid: p.process_id, name: p.name, exe_path: p.executable_path };
            if tx.blocking_send(out).is_err() {
                tracing::info!("process event receiver closed; watcher exiting");
                return;
            }
        }
        tracing::warn!("WMI subscription ended; resubscribing");
        std::thread::sleep(FALLBACK_TICK);
    }
}

// Snapshot diff loop until `deadline`; false when the receiver is gone.
fn fallback_until(
    reg: &SharedRegistry,
    tx: &mpsc::Sender<ProcEvent>,
    seen: &mut HashMap<u32, String>,
    deadline: Instant,
) -> bool {
    while Instant::now() < deadline {
        if !emit_snapshot_diff(reg, tx, seen) {
            return false;
        }
        std::thread::sleep(FALLBACK_TICK);
    }
    true
}

// Emit Started/Exited for registered-name processes that appeared or vanished since the last call.
fn emit_snapshot_diff(reg: &SharedRegistry, tx: &mpsc::Sender<ProcEvent>, seen: &mut HashMap<u32, String>) -> bool {
    let snapshot = match reg.read() {
        Ok(r) => r.clone(),
        Err(p) => p.into_inner().clone(),
    };
    let now: HashMap<u32, String> = registered_processes(&snapshot).into_iter().collect();
    for (pid, name) in &now {
        if !seen.contains_key(pid) {
            let ev = ProcEvent { kind: ProcEventKind::Started, pid: *pid, name: name.clone(), exe_path: None };
            if tx.blocking_send(ev).is_err() {
                return false;
            }
        }
    }
    for (pid, name) in seen.iter() {
        if !now.contains_key(pid) {
            let ev = ProcEvent { kind: ProcEventKind::Exited, pid: *pid, name: name.clone(), exe_path: None };
            if tx.blocking_send(ev).is_err() {
                return false;
            }
        }
    }
    *seen = now;
    true
}

fn report(r: std::sync::mpsc::Sender<Result<(), String>>, v: Result<(), String>) {
    if r.send(v).is_err() {
        tracing::debug!("watch_processes caller stopped waiting");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_process_requires_full_path() {
        let id = GameId::new();
        let cmd = canonicalize(Utf8Path::new("C:\\Windows\\System32\\cmd.exe")).unwrap();
        let reg = DetectRegistry::from_entries([(cmd.as_str(), id)]);
        let ev = |path: &str| ProcEvent {
            kind: ProcEventKind::Exited,
            pid: 0,
            name: "x".into(),
            exe_path: Some(path.into()),
        };
        assert_eq!(match_process(&ev("c:\\windows\\system32\\CMD.EXE"), &reg), Some(id));
        assert_eq!(match_process(&ev("C:\\Windows\\SysWOW64\\cmd.exe"), &reg), None);
        assert_eq!(match_process(&ev("C:\\Program Files (x86)\\Steam\\GameOverlayUI.exe"), &reg), None);
        assert_eq!(match_process(&ev("C:\\Windows\\System32\\GameBarPresenceWriter.exe"), &reg), None);
        let none = ProcEvent { kind: ProcEventKind::Exited, pid: 0, name: "cmd.exe".into(), exe_path: None };
        assert_eq!(match_process(&none, &reg), None);
    }
}
