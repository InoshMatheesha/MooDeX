// tests.rs - Store tests: migrations, constraints, crash recovery, backups.

use super::*;
use moodex_core::{GameDraft, Rating, Status};
use time::macros::datetime;
use time::Duration;

fn now() -> OffsetDateTime {
    datetime!(2026-03-01 12:00 UTC)
}

fn draft(title: &str) -> Game {
    GameDraft { title: title.into(), genres: vec!["Action".into(), "RPG".into()], ..GameDraft::default() }
        .into_game(now())
        .unwrap()
}

fn exe_install(path: &str) -> Install {
    Install { launch: LaunchTarget::Exe { path: path.into(), args: vec![], working_dir: None }, detect_exe: None }
}

use moodex_core::LaunchTarget;

#[test]
fn migrates_from_empty() {
    let s = Store::open_in_memory().unwrap();
    assert_eq!(s.schema_version().unwrap(), 1);
    assert!(s.load_games().unwrap().is_empty());
}

#[test]
fn reopening_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let p = Utf8PathBuf::from_path_buf(dir.path().join("m.db")).unwrap();
    {
        let mut s = Store::open_rw(&p).unwrap();
        s.insert_game(&draft("Halo")).unwrap();
    }
    let s = Store::open_rw(&p).unwrap();
    assert_eq!(s.schema_version().unwrap(), 1);
    assert_eq!(s.load_games().unwrap().len(), 1);
}

#[test]
fn round_trips_a_game() {
    let mut s = Store::open_in_memory().unwrap();
    let mut g = draft("The Witcher 3");
    g.rating = Some(Rating::new(5).unwrap());
    g.status = Status::Playing;
    g.rawg_id = Some(3328);
    g.install = Some(exe_install("C:/Games/W3/witcher3.exe"));
    s.insert_game(&g).unwrap();
    let back = s.get_game(g.id).unwrap().unwrap();
    assert_eq!(back, g);
}

// Two games with the same title stay two games (spec 2 row 5).
#[test]
fn same_title_never_merges() {
    let mut s = Store::open_in_memory().unwrap();
    s.insert_game(&draft("Doom")).unwrap();
    s.insert_game(&draft("Doom")).unwrap();
    assert_eq!(s.load_games().unwrap().len(), 2);
}

#[test]
fn detect_path_is_unique() {
    let mut s = Store::open_in_memory().unwrap();
    let mut a = draft("A");
    a.install = Some(exe_install("C:/x/a.exe"));
    let mut b = draft("B");
    b.install = Some(exe_install("C:/x/a.exe"));
    s.insert_game(&a).unwrap();
    let err = s.insert_game(&b).unwrap_err();
    assert!(matches!(err, StoreError::Conflict(_)), "{err}");
    // Failed transaction left nothing behind.
    assert_eq!(s.load_games().unwrap().len(), 1);
}

#[test]
fn at_most_one_open_session_per_game() {
    let mut s = Store::open_in_memory().unwrap();
    let g = draft("A");
    s.insert_game(&g).unwrap();
    s.open_session(g.id, now(), SessionSource::Wmi).unwrap();
    let err = s.open_session(g.id, now(), SessionSource::Wmi).unwrap_err();
    assert!(matches!(err, StoreError::Conflict(_)));
}

#[test]
fn playtime_is_derived_from_sessions() {
    let mut s = Store::open_in_memory().unwrap();
    let g = draft("A");
    s.insert_game(&g).unwrap();
    let id = s.open_session(g.id, now(), SessionSource::JobObject).unwrap();
    s.heartbeat(id, now() + Duration::minutes(30)).unwrap();
    // Open session counts up to its heartbeat.
    let p = s.play_stats(g.id).unwrap();
    assert!((p.minutes - 30.0).abs() < 0.01, "{}", p.minutes);
    s.close_session(id, now() + Duration::minutes(90)).unwrap();
    let p = s.play_stats(g.id).unwrap();
    assert!((p.minutes - 90.0).abs() < 0.01);
    assert_eq!(p.play_count, 1);
    assert_eq!(p.last_played, Some(now() + Duration::minutes(90)));
    let all = s.play_stats_all().unwrap();
    assert!((all[&g.id].minutes - 90.0).abs() < 0.01);
}

#[test]
fn close_never_precedes_start() {
    let mut s = Store::open_in_memory().unwrap();
    let g = draft("A");
    s.insert_game(&g).unwrap();
    let id = s.open_session(g.id, now(), SessionSource::Wmi).unwrap();
    s.close_session(id, now() - Duration::hours(1)).unwrap();
    assert!(s.play_stats(g.id).unwrap().minutes.abs() < 1e-9);
}

#[test]
fn delete_cascades_sessions_and_genres() {
    let mut s = Store::open_in_memory().unwrap();
    let g = draft("A");
    s.insert_game(&g).unwrap();
    s.open_session(g.id, now(), SessionSource::Wmi).unwrap();
    s.delete_game(g.id).unwrap();
    assert!(s.open_sessions().unwrap().is_empty());
    assert!(matches!(s.delete_game(g.id), Err(StoreError::NotFound(_))));
}

#[test]
fn goat_is_exclusive() {
    let mut s = Store::open_in_memory().unwrap();
    let a = draft("A");
    let b = draft("B");
    s.insert_game(&a).unwrap();
    s.insert_game(&b).unwrap();
    assert_eq!(s.set_goat(Some(a.id), now()).unwrap(), vec![a.id]);
    let changed = s.set_goat(Some(b.id), now()).unwrap();
    assert!(changed.contains(&a.id) && changed.contains(&b.id));
    let games = s.load_games().unwrap();
    assert_eq!(games.iter().filter(|g| g.is_goat).count(), 1);
}

#[test]
fn merge_moves_sessions() {
    let mut s = Store::open_in_memory().unwrap();
    let keep = draft("Keep");
    let mut absorb = draft("Absorb");
    absorb.install = Some(exe_install("C:/g/x.exe"));
    s.insert_game(&keep).unwrap();
    s.insert_game(&absorb).unwrap();
    let a = s.open_session(absorb.id, now(), SessionSource::Wmi).unwrap();
    s.close_session(a, now() + Duration::minutes(10)).unwrap();
    s.open_session(absorb.id, now() + Duration::hours(1), SessionSource::Wmi).unwrap();
    let mut merged = keep.clone();
    merged.install = absorb.install.clone();
    s.merge_games(&merged, absorb.id).unwrap();
    let games = s.load_games().unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].install, absorb.install);
    let p = s.play_stats(keep.id).unwrap();
    assert_eq!(p.play_count, 2);
    assert!(s.open_sessions().unwrap().is_empty());
}

#[test]
fn import_is_all_or_nothing() {
    let mut s = Store::open_in_memory().unwrap();
    let mut a = draft("A");
    a.rawg_id = Some(1);
    let mut b = draft("B");
    b.rawg_id = Some(1);
    assert!(s.import_batch(&[a, b], &[]).is_err());
    assert!(s.load_games().unwrap().is_empty());
}

#[test]
fn settings_and_cache_round_trip() {
    let mut s = Store::open_in_memory().unwrap();
    s.set_setting("min_session_seconds", &serde_json::json!(60)).unwrap();
    s.set_setting("min_session_seconds", &serde_json::json!(90)).unwrap();
    assert_eq!(s.settings().unwrap()["min_session_seconds"], serde_json::json!(90));
    let r = CachedResponse {
        body: b"{}".to_vec(),
        etag: Some("W/1".into()),
        fetched_at: now(),
        expires_at: now() + Duration::hours(6),
    };
    s.cache_put("https://x/y", &r).unwrap();
    assert_eq!(s.cache_get("https://x/y").unwrap(), Some(r));
    assert_eq!(s.cache_prune(now() + Duration::days(1)).unwrap(), 1);
}

// Simulated crash: a connection dropped without checkpoint loses no committed data.
#[test]
fn wal_survives_unclean_close() {
    let dir = tempfile::tempdir().unwrap();
    let p = Utf8PathBuf::from_path_buf(dir.path().join("c.db")).unwrap();
    let g = draft("Survivor");
    {
        let mut s = Store::open_rw(&p).unwrap();
        s.insert_game(&g).unwrap();
        std::mem::forget(s); // no Drop, no checkpoint - like a power cut after commit
    }
    let s = Store::open_rw(&p).unwrap();
    assert_eq!(s.get_game(g.id).unwrap().map(|x| x.title), Some("Survivor".to_string()));
}

#[test]
fn daily_backup_rolls_and_prunes() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let mut s = Store::open_rw(&root.join("db.sqlite")).unwrap();
    s.insert_game(&draft("A")).unwrap();
    let bdir = root.join("backups");
    std::fs::create_dir_all(&bdir).unwrap();
    std::fs::write(bdir.join("keep-me.txt"), b"x").unwrap();
    let mut day = time::macros::date!(2026 - 01 - 01);
    for _ in 0..9 {
        roll_daily_backup(&s, &bdir, day, 7).unwrap();
        day = day.next_day().unwrap();
    }
    let rep = roll_daily_backup(&s, &bdir, day.previous_day().unwrap(), 7).unwrap();
    assert!(rep.created.is_none());
    let names: Vec<String> =
        std::fs::read_dir(&bdir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names.iter().filter(|n| n.starts_with("moodex-")).count(), 7);
    assert!(names.contains(&"keep-me.txt".to_string()));
    // A backup is a valid database.
    let b = Store::open_rw(&bdir.join("moodex-2026-01-09.db")).unwrap();
    assert_eq!(b.load_games().unwrap().len(), 1);
}

#[test]
fn atomic_write_replaces() {
    let dir = tempfile::tempdir().unwrap();
    let p = Utf8PathBuf::from_path_buf(dir.path().join("f.json")).unwrap();
    write_atomic(&p, b"one").unwrap();
    write_atomic(&p, b"two").unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), b"two");
}
