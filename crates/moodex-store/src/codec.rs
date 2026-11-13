// codec.rs - Row <-> domain mapping; timestamps in one fixed, sortable UTC format.

use crate::StoreError;
use camino::Utf8PathBuf;
use moodex_core::text::sort_title;
use moodex_core::{CoverRef, Game, GameId, Install, LaunchTarget, PlayStats, Rating, Status};
use rusqlite::{params, Row, Transaction};
use std::str::FromStr;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, OffsetDateTime, UtcOffset};

pub const GAME_COLUMNS: &str =
    "id, rawg_id, title, cover_url, cover_hash, release_date, status, rating, notes, is_goat, \
     launch_target, detect_exe_path, added_at, updated_at";

// Fixed-width UTC timestamp so string order equals time order.
pub fn fmt_ts(t: OffsetDateTime) -> String {
    let fmt = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");
    t.to_offset(UtcOffset::UTC).format(&fmt).unwrap_or_else(|_| "1970-01-01T00:00:00.000Z".to_string())
}

pub fn parse_ts(s: &str) -> Result<OffsetDateTime, StoreError> {
    OffsetDateTime::parse(s, &Rfc3339).map_err(|e| StoreError::Corrupt(format!("bad timestamp {s:?}: {e}")))
}

fn fmt_date(d: Date) -> String {
    let fmt = format_description!("[year]-[month]-[day]");
    d.format(&fmt).unwrap_or_default()
}

fn parse_date(s: &str) -> Option<Date> {
    let fmt = format_description!("[year]-[month]-[day]");
    Date::parse(s, &fmt).ok()
}

// (launch JSON, canonical detect path, display name).
pub type InstallColumns = (Option<String>, Option<String>, Option<String>);

// Columns for the install: launch JSON, canonical detect path, display name.
pub fn install_columns(install: Option<&Install>) -> Result<InstallColumns, StoreError> {
    match install {
        None => Ok((None, None, None)),
        Some(i) => {
            let launch = serde_json::to_string(&i.launch).map_err(|e| StoreError::Corrupt(e.to_string()))?;
            let detect = i.effective_detect().map(|p| p.to_string());
            let name = i.effective_detect().and_then(|p| p.file_name()).map(str::to_lowercase);
            Ok((Some(launch), detect, name))
        }
    }
}

pub fn game_from_row(r: &Row<'_>) -> Result<Game, StoreError> {
    let id: String = r.get(0)?;
    let id = GameId::from_str(&id).map_err(|e| StoreError::Corrupt(e.to_string()))?;
    let status: String = r.get(6)?;
    let rating: Option<i64> = r.get(7)?;
    let launch: Option<String> = r.get(10)?;
    let detect: Option<String> = r.get(11)?;
    let install = match launch {
        None => None,
        Some(json) => {
            let launch: LaunchTarget =
                serde_json::from_str(&json).map_err(|e| StoreError::Corrupt(format!("launch_target: {e}")))?;
            // A detect path equal to the launched exe is the implicit default - keep it implicit.
            let same_as_exe =
                matches!(&launch, LaunchTarget::Exe { path, .. } if Some(path.as_str()) == detect.as_deref());
            let detect_exe = if same_as_exe { None } else { detect.map(Utf8PathBuf::from) };
            Some(Install { launch, detect_exe })
        }
    };
    let release: Option<String> = r.get(5)?;
    Ok(Game {
        id,
        rawg_id: r.get(1)?,
        title: r.get(2)?,
        cover: CoverRef::from_parts(r.get(3)?, r.get(4)?),
        release_date: release.as_deref().and_then(parse_date),
        status: Status::from_str(&status).map_err(|e| StoreError::Corrupt(e.to_string()))?,
        rating: rating.and_then(|v| Rating::new(v).ok()),
        notes: r.get(8)?,
        is_goat: r.get::<_, i64>(9)? != 0,
        install,
        genres: Vec::new(),
        added_at: parse_ts(&r.get::<_, String>(12)?)?,
        updated_at: parse_ts(&r.get::<_, String>(13)?)?,
    })
}

fn cover_parts(c: &CoverRef) -> (Option<&str>, Option<&str>) {
    match c {
        CoverRef::None => (None, None),
        CoverRef::Remote { url } => (Some(url), None),
        CoverRef::Cached { hash, url } => (Some(url), Some(hash.0.as_str())),
    }
}

fn write_genres(tx: &Transaction<'_>, g: &Game) -> Result<(), StoreError> {
    tx.execute("DELETE FROM genres WHERE game_id = ?1", [g.id.to_string()])?;
    let mut st = tx.prepare("INSERT OR IGNORE INTO genres (game_id, genre) VALUES (?1, ?2)")?;
    for genre in &g.genres {
        st.execute(params![g.id.to_string(), genre])?;
    }
    Ok(())
}

pub fn insert_game(tx: &Transaction<'_>, g: &Game) -> Result<(), StoreError> {
    let (launch, detect, name) = install_columns(g.install.as_ref())?;
    let (cover_url, cover_hash) = cover_parts(&g.cover);
    tx.execute(
        "INSERT INTO games (id, rawg_id, title, sort_title, cover_url, cover_hash, release_date, status, rating, notes,
             is_goat, launch_target, detect_exe_path, detect_exe_name, added_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            g.id.to_string(),
            g.rawg_id,
            g.title,
            sort_title(&g.title),
            cover_url,
            cover_hash,
            g.release_date.map(fmt_date),
            g.status.as_str(),
            g.rating.map(i64::from),
            g.notes,
            i64::from(g.is_goat),
            launch,
            detect,
            name,
            fmt_ts(g.added_at),
            fmt_ts(g.updated_at),
        ],
    )?;
    write_genres(tx, g)
}

// Returns the number of rows updated (0 means not found).
pub fn update_game(tx: &Transaction<'_>, g: &Game) -> Result<usize, StoreError> {
    let (launch, detect, name) = install_columns(g.install.as_ref())?;
    let (cover_url, cover_hash) = cover_parts(&g.cover);
    let n = tx.execute(
        "UPDATE games SET rawg_id = ?2, title = ?3, sort_title = ?4, cover_url = ?5, cover_hash = ?6, release_date = ?7,
             status = ?8, rating = ?9, notes = ?10, is_goat = ?11, launch_target = ?12, detect_exe_path = ?13,
             detect_exe_name = ?14, updated_at = ?15
         WHERE id = ?1",
        params![
            g.id.to_string(),
            g.rawg_id,
            g.title,
            sort_title(&g.title),
            cover_url,
            cover_hash,
            g.release_date.map(fmt_date),
            g.status.as_str(),
            g.rating.map(i64::from),
            g.notes,
            i64::from(g.is_goat),
            launch,
            detect,
            name,
            fmt_ts(g.updated_at),
        ],
    )?;
    if n > 0 {
        write_genres(tx, g)?;
    }
    Ok(n)
}

pub fn play_stats(minutes: f64, count: i64, last: Option<String>) -> PlayStats {
    PlayStats {
        minutes: if minutes.is_finite() && minutes > 0.0 { minutes } else { 0.0 },
        play_count: u32::try_from(count.max(0)).unwrap_or(u32::MAX),
        last_played: last.as_deref().and_then(|s| parse_ts(s).ok()),
    }
}
