// import.rs - One-time v1 import (plan, review, apply) and JSON export/import (spec 15).

use crate::LibraryError;
use camino::{Utf8Path, Utf8PathBuf};
use moodex_core::paths::path_key;
use moodex_core::{Game, GameDraft, GameId, Install, LaunchTarget, Rating, SessionRecord, SessionSource, Status};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, OffsetDateTime, PrimitiveDateTime};

// --- v1 wire format -----------------------------------------------------

#[derive(Debug, Deserialize, Default)]
struct V1File {
    #[serde(default)]
    games: Vec<V1Game>,
    #[serde(default)]
    local_games: Vec<V1Game>,
    // `now_playing` is dead config in v1 and deliberately ignored (spec 2 row 20).
}

#[derive(Debug, Deserialize, Default, Clone)]
struct V1Game {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    background_image: Option<String>,
    #[serde(default)]
    playtime: f64,
    #[serde(default)]
    status: String,
    #[serde(default)]
    exe_path: Option<String>,
    #[serde(default)]
    play_count: i64,
    #[serde(default)]
    rating: f64,
    #[serde(default)]
    added_date: Option<String>,
    #[serde(default)]
    released: Option<String>,
    #[serde(default)]
    last_played: Option<String>,
    #[serde(default)]
    genres: Option<Vec<String>>,
    #[serde(default)]
    notes: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct V1Settings {
    #[serde(default)]
    goat_game_title: String,
}

// --- Plan ---------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportGame {
    pub key: String,
    pub v1_id: i64,
    pub draft: GameDraft,
    pub minutes: f64,
    pub play_count: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_played: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub added_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedMerge {
    pub library_key: String,
    pub local_key: String,
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportWarning {
    pub key: Option<String>,
    pub message: String,
}

// Read-only plan for user review; nothing is written (spec 15).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportPlan {
    pub source_path: Utf8PathBuf,
    pub games: Vec<ImportGame>,
    pub merges: Vec<ProposedMerge>,
    pub warnings: Vec<ImportWarning>,
    pub goat_key: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    pub imported: usize,
    pub merged: usize,
    pub skipped: usize,
    pub sessions: usize,
    pub warnings: Vec<String>,
}

fn map_status(s: &str) -> Option<Status> {
    match s.trim().to_ascii_lowercase().as_str() {
        "playing" => Some(Status::Playing),
        "completed" => Some(Status::Completed),
        "like to play" | "wishlist" | "not played" | "backlog" | "" => Some(Status::Backlog),
        "stopped" | "dropped" => Some(Status::Dropped),
        _ => None,
    }
}

// v1 used a 10-point scale with halves; v2 stores 1..=5 stars.
fn map_rating(r: f64) -> Option<Rating> {
    if !r.is_finite() || r <= 0.0 {
        return None;
    }
    let stars = (r / 2.0).round().clamp(1.0, 5.0) as i64;
    Rating::new(stars).ok()
}

fn parse_v1_date(s: &str) -> Option<Date> {
    let s = s.trim();
    let head = s.get(..10)?;
    Date::parse(head, &format_description!("[year]-[month]-[day]")).ok()
}

fn parse_v1_datetime(s: &str) -> Option<OffsetDateTime> {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("never") || s.eq_ignore_ascii_case("n/a") {
        return None;
    }
    if let Ok(t) = OffsetDateTime::parse(s, &Rfc3339) {
        return Some(t);
    }
    if let Ok(t) = PrimitiveDateTime::parse(s, &format_description!("[year]-[month]-[day] [hour]:[minute]:[second]")) {
        return Some(t.assume_utc());
    }
    parse_v1_date(s).map(|d| d.midnight().assume_utc())
}

fn convert(
    key: String,
    g: &V1Game,
    local: bool,
    now: OffsetDateTime,
    warnings: &mut Vec<ImportWarning>,
) -> Option<ImportGame> {
    let title = g.name.trim().to_string();
    if title.is_empty() {
        warnings.push(ImportWarning { key: Some(key), message: "skipped an entry with no title".into() });
        return None;
    }
    let status = map_status(&g.status).unwrap_or_else(|| {
        warnings.push(ImportWarning {
            key: Some(key.clone()),
            message: format!("{title}: unknown status \"{}\" set to backlog", g.status),
        });
        Status::Backlog
    });
    let install = g.exe_path.as_deref().map(str::trim).filter(|p| !p.is_empty()).map(|p| {
        if !Utf8Path::new(p).exists() {
            warnings.push(ImportWarning {
                key: Some(key.clone()),
                message: format!("{title}: executable not found at {p}; imported and flagged"),
            });
        }
        Install { launch: LaunchTarget::Exe { path: p.into(), args: vec![], working_dir: None }, detect_exe: None }
    });
    let minutes = if g.playtime.is_finite() && g.playtime > 0.0 { g.playtime } else { 0.0 };
    Some(ImportGame {
        key,
        v1_id: g.id,
        draft: GameDraft {
            title,
            // Library entries were added from RAWG, so their v1 id is the RAWG id.
            rawg_id: (!local && g.id > 0).then_some(g.id),
            cover_url: g.background_image.clone().filter(|u| u.starts_with("http")),
            release_date: g.released.as_deref().and_then(parse_v1_date),
            status,
            rating: map_rating(g.rating),
            notes: g.notes.clone().unwrap_or_default(),
            genres: g.genres.clone().unwrap_or_default(),
            install,
        },
        minutes,
        play_count: g.play_count.max(0),
        last_played: g.last_played.as_deref().and_then(parse_v1_datetime),
        added_at: g.added_date.as_deref().and_then(parse_v1_datetime).unwrap_or(now),
    })
}

// Read %LOCALAPPDATA%\MooDeX\games_data.json and build a plan. Never writes (spec 15).
pub fn plan_import(v1_path: &Utf8Path) -> Result<ImportPlan, LibraryError> {
    let text = std::fs::read_to_string(v1_path).map_err(|e| LibraryError::Import(format!("{v1_path}: {e}")))?;
    let settings_path = v1_path.with_file_name("user_settings.json");
    let goat_title = std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|t| serde_json::from_str::<V1Settings>(&t).ok())
        .map(|s| s.goat_game_title)
        .unwrap_or_default();
    plan_from_str(&text, &goat_title, v1_path, OffsetDateTime::now_utc())
}

// Pure core of the planner, testable without files.
pub fn plan_from_str(
    json: &str,
    goat_title: &str,
    source: &Utf8Path,
    now: OffsetDateTime,
) -> Result<ImportPlan, LibraryError> {
    let file: V1File =
        serde_json::from_str(json).map_err(|e| LibraryError::Import(format!("not a MooDeX v1 library: {e}")))?;
    let mut warnings = Vec::new();
    let mut games = Vec::new();
    let mut seen_rawg: HashSet<i64> = HashSet::new();
    for (i, g) in file.games.iter().enumerate() {
        if let Some(mut ig) = convert(format!("games:{i}"), g, false, now, &mut warnings) {
            if let Some(r) = ig.draft.rawg_id {
                if !seen_rawg.insert(r) {
                    warnings.push(ImportWarning {
                        key: Some(ig.key.clone()),
                        message: format!("{}: duplicate RAWG link dropped", ig.draft.title),
                    });
                    ig.draft.rawg_id = None;
                }
            }
            games.push(ig);
        }
    }
    let lib_count = games.len();
    for (i, g) in file.local_games.iter().enumerate() {
        if let Some(ig) = convert(format!("local:{i}"), g, true, now, &mut warnings) {
            games.push(ig);
        }
    }
    // Propose collapsing v1's mirror pairs; the user confirms each (spec 15 step 2).
    let mut merges = Vec::new();
    let mut used: HashSet<usize> = HashSet::new();
    for local in games.iter().skip(lib_count) {
        let by_id = games[..lib_count]
            .iter()
            .enumerate()
            .find(|(i, lib)| !used.contains(i) && local.v1_id != 0 && lib.v1_id == local.v1_id);
        let by_title = || {
            games[..lib_count].iter().enumerate().find(|(i, lib)| {
                !used.contains(i) && lib.draft.title.trim().eq_ignore_ascii_case(local.draft.title.trim())
            })
        };
        let (found, reason) = match by_id {
            Some(f) => (Some(f), "same v1 id"),
            None => (by_title(), "same title"),
        };
        if let Some((i, lib)) = found {
            used.insert(i);
            merges.push(ProposedMerge {
                library_key: lib.key.clone(),
                local_key: local.key.clone(),
                title: lib.draft.title.clone(),
                reason: reason.into(),
            });
        }
    }
    if games.iter().any(|g| g.draft.rating.is_some()) {
        warnings.push(ImportWarning {
            key: None,
            message: "ratings converted from the 10-point scale to 1-5 stars".into(),
        });
    }
    if games.iter().any(|g| g.minutes > 0.0) {
        warnings.push(ImportWarning {
            key: None,
            message: "v1 playtime is imported as one estimated session per game".into(),
        });
    }
    let goat_key = if goat_title.trim().is_empty() {
        None
    } else {
        games[..lib_count]
            .iter()
            .find(|g| g.draft.title.trim().eq_ignore_ascii_case(goat_title.trim()))
            .map(|g| g.key.clone())
    };
    Ok(ImportPlan { source_path: source.to_owned(), games, merges, warnings, goat_key })
}

// Copy the v1 file aside before writing anything (spec 15 step 5).
pub(crate) fn backup_v1_file(src: &Utf8Path) -> Result<(), LibraryError> {
    let dest = src.with_file_name("games_data.v1.backup.json");
    if dest.exists() {
        return Ok(());
    }
    let bytes = std::fs::read(src)?;
    moodex_store::write_atomic(&dest, &bytes).map_err(|e| LibraryError::Io(e.to_string()))
}

pub(crate) struct Built {
    pub games: Vec<Game>,
    pub sessions: Vec<(GameId, OffsetDateTime, OffsetDateTime, SessionSource)>,
    pub goat: Option<GameId>,
    pub report: ImportReport,
}

// Turn a confirmed plan into rows: accepted pairs collapse, all else stays separate.
pub(crate) fn build_v1(
    plan: &ImportPlan,
    accepted: &[(String, String)],
    existing_rawg: &HashSet<i64>,
    existing_detect: &HashSet<String>,
    now: OffsetDateTime,
) -> Result<Built, LibraryError> {
    let by_key: HashMap<&str, &ImportGame> = plan.games.iter().map(|g| (g.key.as_str(), g)).collect();
    let valid: HashSet<(&str, &str)> =
        plan.merges.iter().map(|m| (m.library_key.as_str(), m.local_key.as_str())).collect();
    let mut absorbed: HashSet<&str> = HashSet::new();
    let mut partner: HashMap<&str, &ImportGame> = HashMap::new();
    for (lib, loc) in accepted {
        if !valid.contains(&(lib.as_str(), loc.as_str())) {
            return Err(LibraryError::Invalid(format!("merge {lib} + {loc} was not proposed")));
        }
        if let Some(l) = by_key.get(loc.as_str()) {
            absorbed.insert(loc.as_str());
            partner.insert(lib.as_str(), l);
        }
    }
    let mut out = Built { games: Vec::new(), sessions: Vec::new(), goat: None, report: ImportReport::default() };
    let mut rawg_taken = existing_rawg.clone();
    let mut detect_taken = existing_detect.clone();
    for ig in &plan.games {
        if absorbed.contains(ig.key.as_str()) {
            continue;
        }
        let mut draft = ig.draft.clone();
        let mut minutes = ig.minutes;
        let mut last = ig.last_played;
        if let Some(local) = partner.get(ig.key.as_str()) {
            // Mirrors were synced by Math.Max in v1, so take the max, never the sum.
            minutes = minutes.max(local.minutes);
            last = last.max(local.last_played);
            if draft.install.is_none() {
                draft.install = local.draft.install.clone();
            }
            if draft.cover_url.is_none() {
                draft.cover_url = local.draft.cover_url.clone();
            }
            draft.genres.extend(local.draft.genres.iter().cloned());
            if !local.draft.notes.trim().is_empty() && local.draft.notes != draft.notes {
                draft.notes = if draft.notes.trim().is_empty() {
                    local.draft.notes.clone()
                } else {
                    format!("{}\n\n{}", draft.notes, local.draft.notes)
                };
            }
            out.report.merged += 1;
        }
        if let Some(r) = draft.rawg_id {
            if !rawg_taken.insert(r) {
                out.report.skipped += 1;
                out.report
                    .warnings
                    .push(format!("{}: already in your library (same RAWG entry); skipped", draft.title));
                continue;
            }
        }
        if let Some(i) = &draft.install {
            if let Some(p) = i.effective_detect() {
                if !detect_taken.insert(path_key(p.as_str())) {
                    out.report.warnings.push(format!(
                        "{}: executable already used by another game; install not imported",
                        draft.title
                    ));
                    draft.install = None;
                }
            }
        }
        let mut g = draft.into_game(now)?;
        g.added_at = ig.added_at.min(now);
        if plan.goat_key.as_deref() == Some(ig.key.as_str()) {
            out.goat = Some(g.id);
        }
        if minutes > 0.0 {
            // One synthetic, clearly-labelled session so nobody loses their hours (spec 15 step 3).
            let end = last.unwrap_or(g.added_at).min(now);
            let start = end - time::Duration::seconds_f64(minutes * 60.0);
            out.sessions.push((g.id, start, end, SessionSource::Manual));
        }
        out.games.push(g);
    }
    out.report.imported = out.games.len();
    out.report.sessions = out.sessions.len();
    out.report.warnings.extend(plan.warnings.iter().map(|w| w.message.clone()));
    Ok(out)
}

// --- Export / import of v2 data ------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportSession {
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ended_at: Option<OffsetDateTime>,
    pub source: SessionSource,
}

impl From<SessionRecord> for ExportSession {
    fn from(s: SessionRecord) -> Self {
        Self { started_at: s.started_at, ended_at: s.ended_at, source: s.source }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportGame {
    pub game: Game,
    pub sessions: Vec<ExportSession>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportDoc {
    pub format: String,
    pub version: u32,
    #[serde(with = "time::serde::rfc3339")]
    pub exported_at: OffsetDateTime,
    pub app_version: String,
    pub games: Vec<ExportGame>,
}

impl ExportDoc {
    pub fn new(games: Vec<ExportGame>, now: OffsetDateTime) -> Self {
        Self {
            format: "moodex-library".into(),
            version: 1,
            exported_at: now,
            app_version: env!("CARGO_PKG_VERSION").into(),
            games,
        }
    }
}

// Additive restore: existing games are left alone, nothing is merged silently.
pub(crate) fn build_from_doc(
    doc: ExportDoc,
    existing: &HashSet<GameId>,
    rawg: &HashSet<i64>,
    detect: &HashSet<String>,
) -> Result<Built, LibraryError> {
    if doc.format != "moodex-library" || doc.version != 1 {
        return Err(LibraryError::Import("not a MooDeX library export".into()));
    }
    let mut out = Built { games: Vec::new(), sessions: Vec::new(), goat: None, report: ImportReport::default() };
    let mut rawg = rawg.clone();
    let mut detect = detect.clone();
    for eg in doc.games {
        let mut g = eg.game;
        if existing.contains(&g.id) {
            out.report.skipped += 1;
            continue;
        }
        if g.title.trim().is_empty() {
            out.report.skipped += 1;
            continue;
        }
        if let Some(r) = g.rawg_id {
            if !rawg.insert(r) {
                out.report.warnings.push(format!("{}: RAWG link already used; link dropped", g.title));
                g.rawg_id = None;
            }
        }
        if let Some(i) = &g.install {
            if i.launch.validate().is_err() {
                g.install = None;
            } else if let Some(p) = i.effective_detect() {
                if !detect.insert(path_key(p.as_str())) {
                    out.report.warnings.push(format!("{}: executable already used; install dropped", g.title));
                    g.install = None;
                }
            }
        }
        g.is_goat = false;
        for s in eg.sessions {
            let end = s.ended_at.unwrap_or(s.started_at);
            out.sessions.push((g.id, s.started_at, end, s.source));
        }
        out.games.push(g);
    }
    out.report.imported = out.games.len();
    out.report.sessions = out.sessions.len();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    const V1: &str = r#"{
      "games": [
        {"id": 3328, "name": "The Witcher 3", "background_image": "https://media.rawg.io/w3.jpg", "playtime": 755.0,
         "status": "Playing", "rating": 9.5, "added_date": "2025-01-02 10:00:00", "released": "2015-05-18",
         "last_played": "2026-08-05", "genres": ["RPG"], "notes": "", "play_count": 14},
        {"id": 22, "name": "Doom", "playtime": 0, "status": "Like to Play", "rating": 0, "last_played": "Never"},
        {"id": 23, "name": "Doom", "playtime": 30, "status": "Stopped", "rating": 3},
        {"id": 3328, "name": "Dupe link", "status": "Completed"},
        {"id": 5, "name": "Hades", "status": "Completed", "playtime": 100, "added_date": "N/A"}
      ],
      "local_games": [
        {"id": 3328, "name": "The Witcher 3", "exe_path": "C:\\NotThere\\witcher3.exe", "playtime": 760.0, "status": "Playing",
         "last_played": "2026-08-06"},
        {"id": 9001, "name": "hades", "exe_path": "", "playtime": 90, "status": "Playing"},
        {"id": 9002, "name": "Indie Thing", "exe_path": "", "status": "Weird"}
      ],
      "now_playing": {"ignored": true}
    }"#;

    fn plan() -> ImportPlan {
        plan_from_str(V1, "The Witcher 3", Utf8Path::new("C:/x/games_data.json"), datetime!(2026-11-01 00:00 UTC))
            .unwrap()
    }

    #[test]
    fn plan_maps_fields() {
        let p = plan();
        assert_eq!(p.games.len(), 8);
        let w3 = &p.games[0];
        assert_eq!(w3.draft.rawg_id, Some(3328));
        assert_eq!(w3.draft.status, Status::Playing);
        assert_eq!(w3.draft.rating.map(|r| r.get()), Some(5));
        assert_eq!(w3.draft.release_date.map(|d| d.year()), Some(2015));
        assert_eq!(p.games[1].draft.status, Status::Backlog);
        assert_eq!(p.games[2].draft.status, Status::Dropped);
        assert_eq!(p.games[2].draft.rating.map(|r| r.get()), Some(2));
        assert_eq!(p.games[3].draft.rawg_id, None, "duplicate RAWG id dropped");
        assert_eq!(p.games[7].draft.status, Status::Backlog);
        assert!(p.warnings.iter().any(|w| w.message.contains("unknown status")));
        assert!(p.warnings.iter().any(|w| w.message.contains("not found")));
        assert_eq!(p.goat_key.as_deref(), Some("games:0"));
    }

    #[test]
    fn mirror_pairs_are_proposed_not_merged() {
        let p = plan();
        assert_eq!(p.merges.len(), 2);
        assert_eq!(p.merges[0].library_key, "games:0");
        assert_eq!(p.merges[0].reason, "same v1 id");
        assert_eq!(p.merges[1].local_key, "local:1");
        assert_eq!(p.merges[1].reason, "same title");
        // Two "Doom" entries stay two games.
        assert_eq!(p.games.iter().filter(|g| g.draft.title == "Doom").count(), 2);
    }

    #[test]
    fn build_respects_user_choices() {
        let p = plan();
        let now = datetime!(2026-11-01 00:00 UTC);
        let accept = vec![("games:0".to_string(), "local:0".to_string())];
        let b = build_v1(&p, &accept, &HashSet::new(), &HashSet::new(), now).unwrap();
        // 8 entries, one absorbed.
        assert_eq!(b.games.len(), 7);
        assert_eq!(b.report.merged, 1);
        let w3 = b.games.iter().find(|g| g.title == "The Witcher 3").unwrap();
        assert!(w3.install.is_some());
        let (_, s, e, src) = b.sessions.iter().find(|x| x.0 == w3.id).unwrap();
        assert_eq!(*src, SessionSource::Manual);
        // Max of the mirrors, never the sum.
        assert!(((*e - *s).as_seconds_f64() / 60.0 - 760.0).abs() < 0.01);
        assert_eq!(b.goat, Some(w3.id));
        // Unaccepted Hades pair stays as two games.
        assert_eq!(b.games.iter().filter(|g| g.title.eq_ignore_ascii_case("hades")).count(), 2);
    }

    #[test]
    fn unproposed_merge_is_rejected() {
        let p = plan();
        let bad = vec![("games:1".to_string(), "local:2".to_string())];
        assert!(build_v1(&p, &bad, &HashSet::new(), &HashSet::new(), datetime!(2026-11-01 00:00 UTC)).is_err());
    }

    #[test]
    fn reimport_skips_existing_rawg_entries() {
        let p = plan();
        let existing: HashSet<i64> = [3328].into_iter().collect();
        let b = build_v1(&p, &[], &existing, &HashSet::new(), datetime!(2026-11-01 00:00 UTC)).unwrap();
        assert!(b.report.skipped >= 1);
        assert!(!b.games.iter().any(|g| g.rawg_id == Some(3328)));
    }

    #[test]
    fn garbage_is_an_import_error() {
        assert!(plan_from_str("[1,2", "", Utf8Path::new("x"), datetime!(2026-11-01 00:00 UTC)).is_err());
    }
}
