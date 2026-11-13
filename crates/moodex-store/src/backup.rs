// backup.rs - Consistent snapshots and the daily rolling backup (spec 7.4, 15).

use crate::StoreError;
use camino::{Utf8Path, Utf8PathBuf};
use rusqlite::Connection;
use time::macros::format_description;
use time::Date;

const PREFIX: &str = "moodex-";
const SUFFIX: &str = ".db";

// What the daily roll did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackupReport {
    pub created: Option<Utf8PathBuf>,
    pub pruned: Vec<Utf8PathBuf>,
}

// VACUUM INTO a temp name, then rename over the destination.
pub fn vacuum_into(conn: &Connection, dest: &Utf8Path) -> Result<(), StoreError> {
    let dir = dest.parent().unwrap_or_else(|| Utf8Path::new("."));
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".{}.tmp-{}", dest.file_name().unwrap_or("backup"), uuid::Uuid::now_v7().simple()));
    let res = conn.execute("VACUUM INTO ?1", [tmp.as_str()]);
    if let Err(e) = res {
        if let Err(re) = std::fs::remove_file(&tmp) {
            tracing::debug!(error = %re, "temp backup cleanup");
        }
        return Err(e.into());
    }
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

fn backup_name(day: Date) -> String {
    let fmt = format_description!("[year]-[month]-[day]");
    format!("{PREFIX}{}{SUFFIX}", day.format(&fmt).unwrap_or_default())
}

// Our backups only - never touch other files in the folder.
fn is_ours(name: &str) -> bool {
    let Some(mid) = name.strip_prefix(PREFIX).and_then(|r| r.strip_suffix(SUFFIX)) else { return false };
    let fmt = format_description!("[year]-[month]-[day]");
    Date::parse(mid, &fmt).is_ok()
}

// Create today's backup if missing, keep the newest `retain`.
pub fn roll_daily_backup(
    conn_owner: &crate::Store,
    dir: &Utf8Path,
    today: Date,
    retain: usize,
) -> Result<BackupReport, StoreError> {
    std::fs::create_dir_all(dir)?;
    let mut report = BackupReport::default();
    let target = dir.join(backup_name(today));
    if !target.exists() {
        conn_owner.backup_to(&target)?;
        report.created = Some(target);
    }
    let mut ours: Vec<Utf8PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else { continue };
        if path.file_name().is_some_and(is_ours) {
            ours.push(path);
        }
    }
    // Names embed ISO dates, so lexical order is chronological.
    ours.sort();
    let excess = ours.len().saturating_sub(retain.max(1));
    for old in ours.into_iter().take(excess) {
        std::fs::remove_file(&old)?;
        report.pruned.push(old);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_dated_backups_are_ours() {
        assert!(is_ours("moodex-2026-01-31.db"));
        assert!(!is_ours("moodex.db"));
        assert!(!is_ours("moodex-latest.db"));
        assert!(!is_ours("games_data.json"));
    }
}
