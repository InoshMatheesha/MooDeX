// actor.rs - Actor round-trips against a real store: events, sessions, recovery, merge, import.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test code (spec 4.2)

use camino::Utf8PathBuf;
use moodex_core::paths::DetectRegistry;
use moodex_core::{GameDraft, GameFilter, GamePatch, Rating, SessionSource, SortOrder, Status};
use moodex_images::ImageCache;
use moodex_library::{plan_from_str, Deps, LibraryEvent, LibraryHandle, PlayEvent};
use moodex_store::Store;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::broadcast;

struct Env {
    _dir: tempfile::TempDir,
    root: Utf8PathBuf,
}

fn env() -> Env {
    let d = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(d.path().to_path_buf()).unwrap();
    Env { _dir: d, root }
}

async fn spawn(e: &Env) -> LibraryHandle {
    let store = Store::open_rw(&e.root.join("moodex.db")).unwrap();
    let deps = Deps {
        images: ImageCache::new(e.root.join("covers"), 10 * 1024 * 1024).unwrap(),
        registry: Arc::new(RwLock::new(DetectRegistry::new())),
        backup_dir: Some(e.root.join("backups")),
    };
    LibraryHandle::spawn(store, deps).await.unwrap()
}

fn draft(t: &str) -> GameDraft {
    GameDraft { title: t.into(), ..GameDraft::default() }
}

async fn next_changed(rx: &mut broadcast::Receiver<LibraryEvent>) -> Vec<moodex_core::GameId> {
    loop {
        match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await.unwrap().unwrap() {
            LibraryEvent::GamesChanged { ids } => return ids,
            _ => continue,
        }
    }
}

fn ago(secs: i64) -> OffsetDateTime {
    let t = OffsetDateTime::now_utc() - time::Duration::seconds(secs);
    t.replace_nanosecond(0).unwrap()
}

#[tokio::test]
async fn crud_emits_exactly_the_affected_ids() {
    let e = env();
    let lib = spawn(&e).await;
    let mut rx = lib.subscribe();
    let a = lib.add_game(draft("Alpha")).await.unwrap();
    assert_eq!(next_changed(&mut rx).await, vec![a]);
    let b = lib.add_game(draft("Beta")).await.unwrap();
    assert_eq!(next_changed(&mut rx).await, vec![b]);

    let patch = GamePatch {
        status: Some(Status::Completed),
        rating: Some(Some(Rating::new(4).unwrap())),
        ..GamePatch::default()
    };
    lib.update_game(a, patch).await.unwrap();
    assert_eq!(next_changed(&mut rx).await, vec![a]);
    let v = lib.get_game(a).await.unwrap().unwrap();
    assert_eq!(v.status, Status::Completed);
    assert_eq!(v.rating, Some(4));

    let empty = GamePatch { title: Some("  ".into()), ..GamePatch::default() };
    assert!(lib.update_game(a, empty).await.is_err());

    lib.delete_game(b).await.unwrap();
    assert_eq!(next_changed(&mut rx).await, vec![b]);
    let ids = lib.list_ids(GameFilter::default(), SortOrder::Title).await.unwrap();
    assert_eq!(ids, vec![a]);
}

#[tokio::test]
async fn goat_is_exclusive_and_dashboard_reflects_it() {
    let e = env();
    let lib = spawn(&e).await;
    let a = lib.add_game(draft("A")).await.unwrap();
    let b = lib.add_game(draft("B")).await.unwrap();
    lib.set_goat(Some(a)).await.unwrap();
    lib.set_goat(Some(b)).await.unwrap();
    let d = lib.dashboard().await.unwrap();
    assert_eq!(d.goat.unwrap().id, b);
    assert!(!lib.get_game(a).await.unwrap().unwrap().is_goat);
}

#[tokio::test]
async fn sessions_accumulate_playtime_and_noise_is_discarded() {
    let e = env();
    let lib = spawn(&e).await;
    let g = lib.add_game(draft("Game")).await.unwrap();

    lib.play_event(PlayEvent::Started { game_id: g, source: SessionSource::JobObject, at: ago(600) }).await.unwrap();
    lib.play_event(PlayEvent::Heartbeat { game_id: g, at: ago(590) }).await.unwrap();
    assert_eq!(lib.running().await.unwrap(), vec![g]);
    lib.play_event(PlayEvent::Ended { game_id: g, at: ago(0) }).await.unwrap();
    let v = lib.get_game(g).await.unwrap().unwrap();
    assert!((v.minutes - 10.0).abs() < 0.05, "{}", v.minutes);
    assert_eq!(v.play_count, 1);
    assert!(v.running_since.is_none());

    // A 5-second launch is noise (min_session_seconds = 60).
    lib.play_event(PlayEvent::Started { game_id: g, source: SessionSource::Wmi, at: ago(5) }).await.unwrap();
    lib.play_event(PlayEvent::Ended { game_id: g, at: ago(0) }).await.unwrap();
    assert_eq!(lib.get_game(g).await.unwrap().unwrap().play_count, 1);

    // Duplicate Started (job + WMI) never opens a second session.
    lib.play_event(PlayEvent::Started { game_id: g, source: SessionSource::JobObject, at: ago(300) }).await.unwrap();
    lib.play_event(PlayEvent::Started { game_id: g, source: SessionSource::Wmi, at: ago(200) }).await.unwrap();
    lib.play_event(PlayEvent::Ended { game_id: g, at: ago(0) }).await.unwrap();
    let s = lib.sessions(g).await.unwrap();
    assert_eq!(s.len(), 2);
    assert_eq!(s[0].source, SessionSource::JobObject);

    // Abandoned launches leave no trace.
    lib.play_event(PlayEvent::Started { game_id: g, source: SessionSource::JobObject, at: ago(100) }).await.unwrap();
    lib.play_event(PlayEvent::Abandoned { game_id: g }).await.unwrap();
    assert_eq!(lib.sessions(g).await.unwrap().len(), 2);
}

// A heartbeat gap far beyond the interval means the PC slept: split, don't count the sleep.
#[tokio::test]
async fn sleep_gap_splits_session() {
    let e = env();
    let lib = spawn(&e).await;
    let g = lib.add_game(draft("Sleepy")).await.unwrap();
    let start = 4 * 3600;
    lib.play_event(PlayEvent::Started { game_id: g, source: SessionSource::Wmi, at: ago(start) }).await.unwrap();
    // 30 minutes of normal 30-second heartbeats.
    for i in 1..=60 {
        lib.play_event(PlayEvent::Heartbeat { game_id: g, at: ago(start - i * 30) }).await.unwrap();
    }
    // Slept ~3 hours; heartbeats resume for the last 30 minutes.
    for i in (0..60).rev() {
        lib.play_event(PlayEvent::Heartbeat { game_id: g, at: ago(i * 30 + 30) }).await.unwrap();
    }
    lib.play_event(PlayEvent::Ended { game_id: g, at: ago(0) }).await.unwrap();
    let v = lib.get_game(g).await.unwrap().unwrap();
    assert!((v.minutes - 60.0).abs() < 0.6, "counted {} minutes", v.minutes);
    assert_eq!(v.play_count, 2);
}

// Kill the process mid-session: next start closes the orphan at its last heartbeat (spec 7.4).
#[tokio::test]
async fn crash_recovery_closes_orphans_at_heartbeat() {
    let e = env();
    let game = draft("Crashy").into_game(OffsetDateTime::now_utc()).unwrap();
    {
        // The on-disk state a killed process leaves behind: an open session with a heartbeat.
        let mut store = Store::open_rw(&e.root.join("moodex.db")).unwrap();
        store.insert_game(&game).unwrap();
        let sid = store.open_session(game.id, ago(3600), SessionSource::Wmi).unwrap();
        store.heartbeat(sid, ago(3600 - 20 * 60)).unwrap();
        std::mem::forget(store);
    }
    let lib = spawn(&e).await;
    let v = lib.get_game(game.id).await.unwrap().unwrap();
    assert!(v.running_since.is_none());
    assert!((v.minutes - 20.0).abs() < 0.1, "{}", v.minutes);
    assert!(lib.running().await.unwrap().is_empty());
    assert!(e.root.join("backups").read_dir().unwrap().count() >= 1, "daily backup written");
}

// Dropping every handle is a graceful stop: live sessions are closed, not orphaned.
#[tokio::test]
async fn graceful_stop_closes_live_sessions() {
    let e = env();
    let id;
    {
        let lib = spawn(&e).await;
        id = lib.add_game(draft("Clean")).await.unwrap();
        lib.play_event(PlayEvent::Started { game_id: id, source: SessionSource::Wmi, at: ago(600) }).await.unwrap();
        lib.shutdown().await.unwrap();
    }
    let lib = spawn(&e).await;
    let v = lib.get_game(id).await.unwrap().unwrap();
    assert!((v.minutes - 10.0).abs() < 0.1, "{}", v.minutes);
}

#[tokio::test]
async fn merge_requires_two_games_and_keeps_sessions() {
    let e = env();
    let lib = spawn(&e).await;
    let keep = lib.add_game(draft("Witcher 3")).await.unwrap();
    let absorb = lib
        .add_game(GameDraft {
            title: "The Witcher 3".into(),
            rawg_id: Some(3328),
            notes: "pc".into(),
            ..GameDraft::default()
        })
        .await
        .unwrap();
    lib.play_event(PlayEvent::Started { game_id: absorb, source: SessionSource::Wmi, at: ago(1200) }).await.unwrap();
    lib.play_event(PlayEvent::Ended { game_id: absorb, at: ago(0) }).await.unwrap();
    let dups = lib.duplicates().await.unwrap();
    assert_eq!(dups.len(), 1, "suggested, not merged: {dups:?}");
    assert_eq!(lib.list_ids(GameFilter::default(), SortOrder::Title).await.unwrap().len(), 2);
    assert!(lib.merge_games(keep, keep).await.is_err());
    lib.merge_games(keep, absorb).await.unwrap();
    let v = lib.get_game(keep).await.unwrap().unwrap();
    assert_eq!(v.rawg_id, Some(3328));
    assert_eq!(v.notes, "pc");
    assert!((v.minutes - 20.0).abs() < 0.1);
    assert!(lib.get_game(absorb).await.unwrap().is_none());
}

#[tokio::test]
async fn settings_validate_and_persist() {
    let e = env();
    {
        let lib = spawn(&e).await;
        assert!(lib.set_setting("start_minimized".into(), serde_json::json!(true)).await.is_err());
        lib.set_setting("min_session_seconds".into(), serde_json::json!(120)).await.unwrap();
        lib.shutdown().await.unwrap();
    }
    let lib = spawn(&e).await;
    assert_eq!(lib.settings().await.unwrap().min_session_seconds, 120);
}

#[tokio::test]
async fn v1_import_round_trip_and_export() {
    let e = env();
    let lib = spawn(&e).await;
    let v1 = r#"{"games":[
        {"id": 10, "name": "Red Dead Redemption 2", "playtime": 1250, "status": "Completed", "rating": 8, "last_played": "2026-08-05"},
        {"id": 11, "name": "Hades", "playtime": 0, "status": "Like to Play"}],
      "local_games":[{"id": 10, "name": "Red Dead Redemption 2", "exe_path": "", "playtime": 1240}],
      "now_playing": null}"#;
    let src = e.root.join("games_data.json");
    std::fs::write(&src, v1).unwrap();
    let plan = plan_from_str(v1, "Red Dead Redemption 2", &src, OffsetDateTime::now_utc()).unwrap();
    assert_eq!(plan.merges.len(), 1);
    let accepted = vec![(plan.merges[0].library_key.clone(), plan.merges[0].local_key.clone())];
    let report = lib.import_v1(plan.clone(), accepted.clone()).await.unwrap();
    assert_eq!(report.imported, 2);
    assert_eq!(report.merged, 1);
    assert!(e.root.join("games_data.v1.backup.json").exists());
    let games = lib.list_games(GameFilter::default(), SortOrder::Playtime).await.unwrap();
    assert_eq!(games[0].title, "Red Dead Redemption 2");
    assert!((games[0].minutes - 1250.0).abs() < 0.1);
    assert!(games[0].has_estimate);
    assert!(games[0].is_goat);
    assert_eq!(games[0].rating, Some(4));
    assert!(lib.settings().await.unwrap().v1_import_done);

    // Re-importing does not duplicate.
    let again = lib.import_v1(plan, accepted).await.unwrap();
    assert_eq!(again.imported, 0);

    // Export -> import into a fresh library reproduces games and playtime.
    let doc = lib.export_doc().await.unwrap();
    let e2 = env();
    let lib2 = spawn(&e2).await;
    let r = lib2.import_doc(doc.clone()).await.unwrap();
    assert_eq!(r.imported, 2);
    let g2 = lib2.list_games(GameFilter::default(), SortOrder::Playtime).await.unwrap();
    assert!((g2[0].minutes - 1250.0).abs() < 0.1);
    // Importing the same export again changes nothing.
    assert_eq!(lib2.import_doc(doc).await.unwrap().imported, 0);
}

#[tokio::test]
async fn installs_feed_the_detect_registry() {
    let e = env();
    let store = Store::open_rw(&e.root.join("moodex.db")).unwrap();
    let registry = Arc::new(RwLock::new(DetectRegistry::new()));
    let deps = Deps {
        images: ImageCache::new(e.root.join("covers"), 1024 * 1024).unwrap(),
        registry: Arc::clone(&registry),
        backup_dir: None,
    };
    let lib = LibraryHandle::spawn(store, deps).await.unwrap();
    let g = lib.add_game(draft("Cmd")).await.unwrap();
    let install = moodex_core::Install {
        launch: moodex_core::LaunchTarget::Exe {
            path: "C:/Windows/System32/cmd.exe".into(),
            args: vec![],
            working_dir: None,
        },
        detect_exe: None,
    };
    lib.set_install(g, Some(install)).await.unwrap();
    assert_eq!(registry.read().unwrap().match_path("c:\\windows\\system32\\cmd.exe"), Some(g));
    let v = lib.get_game(g).await.unwrap().unwrap();
    assert!(v.installed && !v.install_missing);
    let pc =
        lib.list_ids(GameFilter { installed_only: true, ..GameFilter::default() }, SortOrder::Title).await.unwrap();
    assert_eq!(pc, vec![g]);

    // A second game may not claim the same executable.
    let h = lib.add_game(draft("Other")).await.unwrap();
    let dup = moodex_core::Install {
        launch: moodex_core::LaunchTarget::Exe {
            path: "C:/Windows/System32/cmd.exe".into(),
            args: vec![],
            working_dir: None,
        },
        detect_exe: None,
    };
    assert!(lib.set_install(h, Some(dup)).await.is_err());

    // A missing detection exe is rejected up front.
    let bad = moodex_core::Install {
        launch: moodex_core::LaunchTarget::Uri { uri: "steam://rungameid/440".into() },
        detect_exe: Some("C:/nope/hl2.exe".into()),
    };
    assert!(lib.set_install(h, Some(bad)).await.is_err());
}
