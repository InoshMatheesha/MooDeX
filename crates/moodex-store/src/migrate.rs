// migrate.rs - Forward-only numbered migrations embedded in the binary.

use crate::StoreError;
use rusqlite::{Connection, OptionalExtension};

const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../migrations/001_init.sql"))];

pub fn current_version(conn: &Connection) -> Result<i64, StoreError> {
    let exists: Option<String> = conn
        .query_row("SELECT name FROM sqlite_master WHERE type='table' AND name='schema_version'", [], |r| r.get(0))
        .optional()?;
    if exists.is_none() {
        return Ok(0);
    }
    let v: Option<i64> = conn.query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))?;
    Ok(v.unwrap_or(0))
}

// Apply every pending migration in one transaction.
pub fn run(conn: &mut Connection) -> Result<(), StoreError> {
    let from = current_version(conn)?;
    let latest = MIGRATIONS.iter().map(|(v, _)| *v).max().unwrap_or(0);
    if from > latest {
        return Err(StoreError::Corrupt(format!("database schema v{from} is newer than this build (v{latest})")));
    }
    if from == latest {
        return Ok(());
    }
    let tx = conn.transaction()?;
    for (v, sql) in MIGRATIONS.iter().filter(|(v, _)| *v > from) {
        tx.execute_batch(sql)?;
        tx.execute("DELETE FROM schema_version", [])?;
        tx.execute("INSERT INTO schema_version (version) VALUES (?1)", [v])?;
        tracing::info!(version = v, "applied migration");
    }
    tx.commit()?;
    Ok(())
}
