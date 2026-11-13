// scan.rs - One-off snapshot of running processes (startup, or right after an install changes).

use crate::handle::OwnedHandle;
use crate::watch::{match_process, ProcEvent, ProcEventKind};
use moodex_core::paths::DetectRegistry;
use moodex_core::GameId;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

// (pid, exe name) of running processes whose file name is registered; RAII snapshot handle.
pub(crate) fn registered_processes(reg: &DetectRegistry) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    if reg.is_empty() {
        return out;
    }
    // SAFETY: snapshot of all processes; the returned handle is owned and closed by OwnedHandle.
    let snap = match unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) } {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!(error = %e, "process snapshot failed");
            return out;
        }
    };
    let Some(snap) = OwnedHandle::new(snap) else { return out };
    let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
    // SAFETY: `entry` is a correctly sized struct with dwSize set, valid for writes.
    let mut ok = unsafe { Process32FirstW(snap.raw(), &mut entry) }.is_ok();
    while ok {
        let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
        if reg.contains_file_name(&name) {
            out.push((entry.th32ProcessID, name));
        }
        // SAFETY: same snapshot handle and struct as above.
        ok = unsafe { Process32NextW(snap.raw(), &mut entry) }.is_ok();
    }
    out
}

// Running processes that belong to registered games: (pid, game). Full-path match only.
pub fn scan_running(reg: &DetectRegistry) -> Vec<(u32, GameId)> {
    registered_processes(reg)
        .into_iter()
        .filter_map(|(pid, name)| {
            let ev = ProcEvent { kind: ProcEventKind::Started, pid, name, exe_path: None };
            match_process(&ev, reg).map(|id| (pid, id))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon::{canonicalize, process_image_path};
    use camino::Utf8Path;

    #[test]
    fn finds_this_test_process() {
        let me = process_image_path(std::process::id()).unwrap();
        let canon = canonicalize(Utf8Path::new(&me)).unwrap();
        let id = GameId::new();
        let reg = DetectRegistry::from_entries([(canon.as_str(), id)]);
        let found = scan_running(&reg);
        assert!(found.contains(&(std::process::id(), id)), "{found:?}");
    }
}
