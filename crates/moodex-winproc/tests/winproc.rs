// winproc.rs - Windows integration: WMI sees a launched process start and exit.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test code (spec 4.2)

use camino::Utf8Path;
use moodex_core::paths::DetectRegistry;
use moodex_core::{GameId, LaunchTarget};
use moodex_winproc::{canonicalize, launch_tracked, match_process, watch_processes, ProcEventKind};
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn wmi_reports_start_and_exit_of_registered_exe() {
    let _ = tracing_subscriber::fmt().with_test_writer().with_env_filter("moodex_winproc=debug").try_init();
    let exe = canonicalize(Utf8Path::new("C:/Windows/System32/PING.EXE")).unwrap();
    let id = GameId::new();
    let reg = Arc::new(RwLock::new(DetectRegistry::from_entries([(exe.as_str(), id)])));
    let mut rx = watch_processes(Arc::clone(&reg)).expect("WMI available");

    let target = LaunchTarget::Exe {
        path: exe.clone(),
        args: vec!["-n".into(), "5".into(), "127.0.0.1".into()],
        working_dir: None,
    };
    let launch = launch_tracked(&target).await.unwrap();
    let pid = launch.root_pid.unwrap();

    let mut saw_start = false;
    let mut saw_exit = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(40);
    while !(saw_start && saw_exit) {
        let ev = tokio::time::timeout_at(deadline, rx.recv()).await.expect("events in time").expect("channel open");
        if ev.pid != pid {
            continue;
        }
        let snapshot = reg.read().unwrap().clone();
        assert_eq!(match_process(&ev, &snapshot), Some(id));
        match ev.kind {
            ProcEventKind::Started => saw_start = true,
            ProcEventKind::Exited => saw_exit = true,
        }
    }
}
