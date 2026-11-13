// tracker.rs - End to end on Windows: Play -> job/WMI tracking -> session rows.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test code (spec 4.2)

use camino::Utf8PathBuf;
use moodex_core::paths::DetectRegistry;
use moodex_core::{GameDraft, Install, LaunchTarget, SessionSource};
use moodex_images::ImageCache;
use moodex_library::{Deps, LibraryEvent, LibraryHandle, PlayTracker, Player};
use moodex_store::Store;
use std::sync::{Arc, RwLock};
use std::time::Duration;

async fn setup(dir: &tempfile::TempDir) -> (LibraryHandle, Player) {
    let _ = tracing_subscriber::fmt().with_test_writer().with_env_filter("debug").try_init();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let registry = Arc::new(RwLock::new(DetectRegistry::new()));
    let deps = Deps {
        images: ImageCache::new(root.join("covers"), 1 << 20).unwrap(),
        registry: Arc::clone(&registry),
        backup_dir: None,
    };
    let lib = LibraryHandle::spawn(Store::open_rw(&root.join("m.db")).unwrap(), deps).await.unwrap();
    lib.set_setting("min_session_seconds".into(), serde_json::json!(0)).await.unwrap();
    let procs = match moodex_winproc::watch_processes(Arc::clone(&registry)) {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("WATCH ERROR: {e}");
            None
        }
    };
    let tracker = PlayTracker::spawn(lib.clone(), registry, procs, Duration::from_secs(20));
    (lib.clone(), Player::new(lib, tracker))
}

async fn wait_for_end(rx: &mut tokio::sync::broadcast::Receiver<LibraryEvent>, secs: u64) -> f64 {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    loop {
        match tokio::time::timeout_at(deadline, rx.recv()).await.expect("session should end in time") {
            Ok(LibraryEvent::SessionEnded { minutes, .. }) => return minutes,
            Ok(_) => {}
            Err(e) => panic_free(e),
        }
    }
}

fn panic_free(e: tokio::sync::broadcast::error::RecvError) {
    assert!(matches!(e, tokio::sync::broadcast::error::RecvError::Lagged(_)), "{e}");
}

// Layer A: we start the process; the job object reports the whole tree's exit.
#[tokio::test(flavor = "multi_thread")]
async fn plain_exe_tracked_by_job() {
    let dir = tempfile::tempdir().unwrap();
    let (lib, player) = setup(&dir).await;
    let id = lib
        .add_game(GameDraft {
            title: "Pinger".into(),
            install: Some(Install {
                launch: LaunchTarget::Exe {
                    // waitfor.exe is almost never running, so nothing else matches its path.
                    path: "C:/Windows/System32/waitfor.exe".into(),
                    args: vec!["/T".into(), "3".into(), "MooDeXTestSignal".into()],
                    working_dir: None,
                },
                detect_exe: None,
            }),
            ..GameDraft::default()
        })
        .await
        .unwrap();
    let mut rx = lib.subscribe();
    let out = player.play(id).await.unwrap();
    assert!(out.tracked);
    assert_eq!(out.method, "JobObject");
    let minutes = wait_for_end(&mut rx, 30).await;
    assert!(minutes > 0.03 && minutes < 0.5, "{minutes}");
    let s = lib.sessions(id).await.unwrap();
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].source, SessionSource::JobObject);
}

// Layer B: the launcher exits at once and the real game runs OUTSIDE our job (the Steam case).
#[tokio::test(flavor = "multi_thread")]
async fn launcher_stub_is_promoted_to_wmi() {
    let dir = tempfile::tempdir().unwrap();
    let (lib, player) = setup(&dir).await;
    // Win32_Process.Create spawns PING via the WMI service, so it escapes our job like a Steam game.
    let script = "Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{CommandLine='ping -n 9 127.0.0.1'} | Out-Null";
    let id = lib
        .add_game(GameDraft {
            title: "Stubbed".into(),
            install: Some(Install {
                launch: LaunchTarget::Exe {
                    path: "C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe".into(),
                    args: vec!["-NoProfile".into(), "-NonInteractive".into(), "-Command".into(), script.into()],
                    working_dir: None,
                },
                detect_exe: Some("C:/Windows/System32/PING.EXE".into()),
            }),
            ..GameDraft::default()
        })
        .await
        .unwrap();
    let mut rx = lib.subscribe();
    let t0 = std::time::Instant::now();
    player.play(id).await.unwrap();
    let _minutes = wait_for_end(&mut rx, 60).await;
    // The session outlived the stub and ended only when the real game (ping, ~8 s) exited.
    assert!(t0.elapsed() >= Duration::from_secs(8), "ended after {:?}", t0.elapsed());
    let s = lib.sessions(id).await.unwrap();
    assert_eq!(s.len(), 1);
    assert!(s[0].minutes * 60.0 >= 7.0, "session was {}s", s[0].minutes * 60.0);
}
