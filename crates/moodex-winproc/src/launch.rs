// launch.rs - Start a game and learn when its whole process tree exits (spec 8.4, 8.5).

use crate::cmdline::{build_command_line, join_args};
use crate::handle::OwnedHandle;
use crate::job::JobObject;
use camino::{Utf8Path, Utf8PathBuf};
use moodex_core::LaunchTarget;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tokio::sync::oneshot;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{ERROR_CANCELLED, ERROR_ELEVATION_REQUIRED};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
use windows::Win32::System::Threading::{
    CreateProcessW, GetProcessId, ResumeThread, TerminateProcess, WaitForSingleObject, CREATE_SUSPENDED,
    CREATE_UNICODE_ENVIRONMENT, INFINITE, PROCESS_INFORMATION, STARTUPINFOW,
};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

// Why tracking of a launch finished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitReason {
    // Every process in the job has exited.
    TreeExited { processes: u32 },
    // The single process we could watch has exited.
    ProcessExited,
    // The shell gave us no process to watch (e.g. steam:// URIs).
    NoProcess,
    // Waiting failed; treated as an exit.
    WaitFailed(String),
}

// How a launch is being watched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMethod {
    JobObject,
    ProcessHandle,
    Untracked,
}

#[derive(Debug)]
pub struct TrackedLaunch {
    // RAII job; also held by the waiter thread until the tree exits.
    pub job: Option<Arc<JobObject>>,
    pub root_pid: Option<u32>,
    pub method: LaunchMethod,
    pub started: Instant,
    // Resolves when the LAST process in the tree exits.
    pub completion: oneshot::Receiver<ExitReason>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LaunchError {
    #[error("executable not found: {path}")]
    ExecutableMissing { path: Utf8PathBuf },
    #[error("not a file: {path}")]
    NotAFile { path: Utf8PathBuf },
    #[error("launch cancelled")]
    Cancelled,
    #[error("could not start the game: {0}")]
    Spawn(String),
    #[error("invalid launch target: {0}")]
    InvalidTarget(String),
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn check_file(path: &Utf8Path) -> Result<(), LaunchError> {
    match std::fs::metadata(path) {
        Ok(m) if m.is_file() => Ok(()),
        Ok(_) => Err(LaunchError::NotAFile { path: path.to_owned() }),
        Err(_) => Err(LaunchError::ExecutableMissing { path: path.to_owned() }),
    }
}

// Launch per target kind and start watching for exit.
pub async fn launch_tracked(target: &LaunchTarget) -> Result<TrackedLaunch, LaunchError> {
    target.validate().map_err(|e| LaunchError::InvalidTarget(e.to_string()))?;
    let target = target.clone();
    tokio::task::spawn_blocking(move || launch_blocking(&target))
        .await
        .map_err(|e| LaunchError::Spawn(format!("launcher task failed: {e}")))?
}

fn launch_blocking(target: &LaunchTarget) -> Result<TrackedLaunch, LaunchError> {
    match target {
        LaunchTarget::Exe { path, args, working_dir } => {
            check_file(path)?;
            let dir = working_dir.clone().or_else(|| path.parent().map(Utf8Path::to_owned));
            match spawn_suspended(path, args, dir.as_deref()) {
                Ok((process, thread)) => Ok(track_suspended(process, thread)),
                Err(SpawnFailure::NeedsElevation) => {
                    // requireAdministrator games: let the shell raise UAC, then watch the handle.
                    tracing::info!(%path, "executable requires elevation; launching via shell");
                    let params = join_args(args);
                    let h = shell_execute(path.as_str(), Some(&params), dir.as_ref().map(|d| d.as_str()))?;
                    Ok(track_existing(h))
                }
                Err(SpawnFailure::Other(e)) => Err(LaunchError::Spawn(e)),
            }
        }
        LaunchTarget::Uri { uri } => {
            let h = shell_execute(uri, None, None)?;
            Ok(track_existing(h))
        }
        LaunchTarget::Shortcut { path } => {
            check_file(path)?;
            let ext = path.extension().map(str::to_ascii_lowercase);
            if !matches!(ext.as_deref(), Some("lnk") | Some("url")) {
                return Err(LaunchError::InvalidTarget(format!("shortcut must be .lnk or .url: {path}")));
            }
            let h = shell_execute(path.as_str(), None, None)?;
            Ok(track_existing(h))
        }
    }
}

enum SpawnFailure {
    NeedsElevation,
    Other(String),
}

// CreateProcessW with CREATE_SUSPENDED so the job is assigned before the game runs.
fn spawn_suspended(
    path: &Utf8Path,
    args: &[String],
    dir: Option<&Utf8Path>,
) -> Result<(OwnedHandle, OwnedHandle), SpawnFailure> {
    let app = wide(path.as_str());
    let mut cmd = wide(&build_command_line(path.as_str(), args));
    let dir_w = dir.map(|d| wide(d.as_str()));
    let si = STARTUPINFOW { cb: std::mem::size_of::<STARTUPINFOW>() as u32, ..Default::default() };
    let mut pi = PROCESS_INFORMATION::default();
    let dir_ptr = dir_w.as_ref().map(|d| PCWSTR(d.as_ptr())).unwrap_or(PCWSTR::null());
    // SAFETY: every pointer refers to a NUL-terminated buffer that outlives the call; `cmd` is mutable
    // as CreateProcessW requires; `si` and `pi` are valid, correctly sized structs.
    let res = unsafe {
        CreateProcessW(
            PCWSTR(app.as_ptr()),
            Some(PWSTR(cmd.as_mut_ptr())),
            None,
            None,
            false,
            CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
            None,
            dir_ptr,
            &si,
            &mut pi,
        )
    };
    if let Err(e) = res {
        if e.code() == ERROR_ELEVATION_REQUIRED.to_hresult() {
            return Err(SpawnFailure::NeedsElevation);
        }
        return Err(SpawnFailure::Other(e.message()));
    }
    match (OwnedHandle::new(pi.hProcess), OwnedHandle::new(pi.hThread)) {
        (Some(p), Some(t)) => Ok((p, t)),
        _ => Err(SpawnFailure::Other("CreateProcessW returned invalid handles".into())),
    }
}

// Assign to a job, resume, and watch the whole tree.
fn track_suspended(process: OwnedHandle, thread: OwnedHandle) -> TrackedLaunch {
    let started = Instant::now();
    // SAFETY: `process` is a live handle we own.
    let pid = unsafe { GetProcessId(process.raw()) };
    let job = match JobObject::create() {
        Ok(j) => match j.assign(process.raw()) {
            Ok(()) => Some(Arc::new(j)),
            Err(e) => {
                tracing::warn!(pid, error = %e, "could not assign process to job; falling back to handle wait");
                None
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "could not create job object; falling back to handle wait");
            None
        }
    };
    // SAFETY: `thread` is the live primary thread handle of the suspended child.
    let resumed = unsafe { ResumeThread(thread.raw()) };
    drop(thread);
    if resumed == u32::MAX {
        tracing::error!(pid, "ResumeThread failed; terminating the suspended child");
        // SAFETY: `process` is live; terminating a never-resumed child is the only safe cleanup.
        if let Err(e) = unsafe { TerminateProcess(process.raw(), 1) } {
            tracing::error!(pid, error = %e, "TerminateProcess failed");
        }
        let (tx, rx) = oneshot::channel();
        send_quiet(tx, ExitReason::WaitFailed("could not resume the game".into()));
        return TrackedLaunch {
            job: None,
            root_pid: Some(pid),
            method: LaunchMethod::Untracked,
            started,
            completion: rx,
        };
    }
    match job {
        Some(job) => {
            let (tx, rx) = oneshot::channel();
            let waiter = Arc::clone(&job);
            spawn_waiter(
                move || {
                    // The process handle is not needed once the job owns the tree.
                    drop(process);
                    match waiter.wait_until_empty() {
                        Ok(n) => ExitReason::TreeExited { processes: n },
                        Err(e) => ExitReason::WaitFailed(e),
                    }
                },
                tx,
            );
            TrackedLaunch {
                job: Some(job),
                root_pid: Some(pid),
                method: LaunchMethod::JobObject,
                started,
                completion: rx,
            }
        }
        None => {
            let (tx, rx) = oneshot::channel();
            spawn_waiter(move || wait_process(&process), tx);
            TrackedLaunch {
                job: None,
                root_pid: Some(pid),
                method: LaunchMethod::ProcessHandle,
                started,
                completion: rx,
            }
        }
    }
}

// Track a process the shell started (already running); best effort job assignment.
fn track_existing(process: Option<OwnedHandle>) -> TrackedLaunch {
    let started = Instant::now();
    let (tx, rx) = oneshot::channel();
    let Some(process) = process else {
        send_quiet(tx, ExitReason::NoProcess);
        return TrackedLaunch { job: None, root_pid: None, method: LaunchMethod::Untracked, started, completion: rx };
    };
    // SAFETY: `process` is a live handle we own.
    let pid = unsafe { GetProcessId(process.raw()) };
    let job = JobObject::create().ok().and_then(|j| j.assign(process.raw()).ok().map(|()| Arc::new(j)));
    match job {
        Some(job) => {
            let waiter = Arc::clone(&job);
            spawn_waiter(
                move || {
                    drop(process);
                    match waiter.wait_until_empty() {
                        Ok(n) => ExitReason::TreeExited { processes: n },
                        Err(e) => ExitReason::WaitFailed(e),
                    }
                },
                tx,
            );
            TrackedLaunch {
                job: Some(job),
                root_pid: Some(pid),
                method: LaunchMethod::JobObject,
                started,
                completion: rx,
            }
        }
        None => {
            spawn_waiter(move || wait_process(&process), tx);
            TrackedLaunch {
                job: None,
                root_pid: Some(pid),
                method: LaunchMethod::ProcessHandle,
                started,
                completion: rx,
            }
        }
    }
}

fn wait_process(process: &OwnedHandle) -> ExitReason {
    // SAFETY: the handle is live for the duration of the wait.
    unsafe { WaitForSingleObject(process.raw(), INFINITE) };
    ExitReason::ProcessExited
}

// Waits block for hours, so each gets a named OS thread rather than a pool slot.
fn spawn_waiter<F>(f: F, tx: oneshot::Sender<ExitReason>)
where
    F: FnOnce() -> ExitReason + Send + 'static,
{
    let spawned = std::thread::Builder::new().name("moodex-launch-wait".into()).spawn(move || send_quiet(tx, f()));
    if let Err(e) = spawned {
        tracing::error!(error = %e, "could not spawn waiter thread");
    }
}

fn send_quiet(tx: oneshot::Sender<ExitReason>, r: ExitReason) {
    if tx.send(r).is_err() {
        tracing::debug!("launch completion receiver dropped");
    }
}

// ShellExecuteExW on a dedicated STA thread; returns the process handle if the shell gave one.
fn shell_execute(file: &str, params: Option<&str>, dir: Option<&str>) -> Result<Option<OwnedHandle>, LaunchError> {
    let file = file.to_string();
    let params = params.map(str::to_string);
    let dir = dir.map(str::to_string);
    let worker = std::thread::Builder::new().name("moodex-shell-exec".into()).spawn(move || {
        // SAFETY: COM init on this fresh thread is balanced by CoUninitialize below.
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        let file_w = wide(&file);
        let params_w = params.as_deref().filter(|p| !p.is_empty()).map(wide);
        let dir_w = dir.as_deref().map(wide);
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            lpFile: PCWSTR(file_w.as_ptr()),
            lpParameters: params_w.as_ref().map(|p| PCWSTR(p.as_ptr())).unwrap_or(PCWSTR::null()),
            lpDirectory: dir_w.as_ref().map(|d| PCWSTR(d.as_ptr())).unwrap_or(PCWSTR::null()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        // SAFETY: `info` is correctly sized and every string pointer outlives the call.
        let res = unsafe { ShellExecuteExW(&mut info) };
        let out = match res {
            Ok(()) => Ok(OwnedHandle::new(info.hProcess)),
            Err(e) if e.code() == ERROR_CANCELLED.to_hresult() => Err(LaunchError::Cancelled),
            Err(e) => Err(LaunchError::Spawn(e.message())),
        };
        if hr.is_ok() {
            // SAFETY: balances the successful CoInitializeEx above on the same thread.
            unsafe { CoUninitialize() };
        }
        out
    });
    match worker {
        Ok(h) => h.join().map_err(|_| LaunchError::Spawn("shell launcher thread panicked".into()))?,
        Err(e) => Err(LaunchError::Spawn(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn cmd(args: &[&str]) -> LaunchTarget {
        LaunchTarget::Exe {
            path: "C:\\Windows\\System32\\cmd.exe".into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            working_dir: None,
        }
    }

    #[tokio::test]
    async fn missing_exe_is_typed_error() {
        let t = LaunchTarget::Exe { path: "C:\\definitely\\not\\here.exe".into(), args: vec![], working_dir: None };
        assert!(matches!(launch_tracked(&t).await, Err(LaunchError::ExecutableMissing { .. })));
    }

    #[tokio::test]
    async fn job_reports_single_process_exit() {
        let l = launch_tracked(&cmd(&["/c", "exit", "0"])).await.unwrap();
        assert_eq!(l.method, LaunchMethod::JobObject);
        let r = tokio::time::timeout(Duration::from_secs(20), l.completion).await.unwrap().unwrap();
        assert!(matches!(r, ExitReason::TreeExited { .. }), "{r:?}");
    }

    // The root exits immediately but a grandchild keeps running: the job must wait for it.
    #[tokio::test]
    async fn job_waits_for_whole_tree() {
        let l = launch_tracked(&cmd(&["/c", "start", "/b", "ping", "-n", "4", "127.0.0.1"])).await.unwrap();
        let t0 = l.started;
        let r = tokio::time::timeout(Duration::from_secs(30), l.completion).await.unwrap().unwrap();
        let processes = match &r {
            ExitReason::TreeExited { processes } => *processes,
            _ => 0,
        };
        assert!(processes >= 2, "tree had {processes} processes: {r:?}");
        assert!(t0.elapsed() >= Duration::from_millis(2500), "ended after {:?}", t0.elapsed());
    }

    #[tokio::test]
    async fn web_uris_are_rejected() {
        let t = LaunchTarget::Uri { uri: "https://example.com".into() };
        assert!(matches!(launch_tracked(&t).await, Err(LaunchError::InvalidTarget(_))));
    }
}
