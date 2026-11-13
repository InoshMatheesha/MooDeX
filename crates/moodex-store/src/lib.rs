//! moodex-store - SQLite persistence; only the library actor holds a writable Store (spec 7.2).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))] // tests may unwrap (spec 4.2)

mod atomic;
mod backup;
mod codec;
mod error;
mod migrate;

pub use atomic::write_atomic;
pub use backup::{roll_daily_backup, BackupReport};
pub use codec::{fmt_ts, parse_ts};
pub use error::StoreError;

use camino::{Utf8Path, Utf8PathBuf};
use codec::{game_from_row, GAME_COLUMNS};
use moodex_core::{Game, GameId, Install, OpenSession, PlayStats, SessionRecord, SessionSource};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use time::OffsetDateTime;

// One cached HTTP response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedResponse {
    pub body: Vec<u8>,
    pub etag: Option<String>,
    pub fetched_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

// The database handle. Not Clone, not Sync: there is exactly one owner.
pub struct Store {
    conn: Connection,
    path: Option<Utf8PathBuf>,
}

impl Store {
    // Open the writable database; called once, by the actor (spec 7.2).
    pub fn open_rw(path: &Utf8Path) -> Result<Self, StoreError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path.as_std_path())?;
        let mut store = Store { conn, path: Some(path.to_owned()) };
        store.configure()?;
        store.migrate()?;
        Ok(store)
    }

    // In-memory database for tests.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        let mut store = Store { conn, path: None };
        store.configure()?;
        store.migrate()?;
        Ok(store)
    }

    fn configure(&mut self) -> Result<(), StoreError> {
        // WAL + NORMAL sync: crash-safe without fsync on every statement (spec 6.1).
        self.conn.pragma_update(None, "journal_mode", "WAL")?;
        self.conn.pragma_update(None, "synchronous", "NORMAL")?;
        self.conn.pragma_update(None, "foreign_keys", "ON")?;
        self.conn.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(())
    }

    fn migrate(&mut self) -> Result<(), StoreError> {
        migrate::run(&mut self.conn)
    }

    pub fn path(&self) -> Option<&Utf8Path> {
        self.path.as_deref()
    }

    pub fn schema_version(&self) -> Result<i64, StoreError> {
        migrate::current_version(&self.conn)
    }

    // --- Games -----------------------------------------------------------

    // Every game with its genres.
    pub fn load_games(&self) -> Result<Vec<Game>, StoreError> {
        let mut genres: HashMap<String, Vec<String>> = HashMap::new();
        {
            let mut st = self.conn.prepare("SELECT game_id, genre FROM genres ORDER BY rowid")?;
            let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            for row in rows {
                let (id, g) = row?;
                genres.entry(id).or_default().push(g);
            }
        }
        let sql = format!("SELECT {GAME_COLUMNS} FROM games");
        let mut st = self.conn.prepare(&sql)?;
        let mut rows = st.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            match game_from_row(row) {
                Ok(mut g) => {
                    g.genres = genres.remove(&g.id.to_string()).unwrap_or_default();
                    out.push(g);
                }
                // One bad row must not hide the rest of the library.
                Err(e) => tracing::error!(error = %e, "skipping unreadable game row"),
            }
        }
        Ok(out)
    }

    pub fn get_game(&self, id: GameId) -> Result<Option<Game>, StoreError> {
        let sql = format!("SELECT {GAME_COLUMNS} FROM games WHERE id = ?1");
        let game = self.conn.query_row(&sql, [id.to_string()], |r| Ok(game_from_row(r))).optional()?;
        match game {
            None => Ok(None),
            Some(res) => {
                let mut g = res?;
                g.genres = self.genres_of(id)?;
                Ok(Some(g))
            }
        }
    }

    fn genres_of(&self, id: GameId) -> Result<Vec<String>, StoreError> {
        let mut st = self.conn.prepare("SELECT genre FROM genres WHERE game_id = ?1 ORDER BY rowid")?;
        let rows = st.query_map([id.to_string()], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn insert_game(&mut self, g: &Game) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        codec::insert_game(&tx, g)?;
        tx.commit()?;
        Ok(())
    }

    pub fn update_game(&mut self, g: &Game) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        let n = codec::update_game(&tx, g)?;
        if n == 0 {
            return Err(StoreError::NotFound(g.id));
        }
        tx.commit()?;
        Ok(())
    }

    pub fn delete_game(&mut self, id: GameId) -> Result<(), StoreError> {
        let n = self.conn.execute("DELETE FROM games WHERE id = ?1", [id.to_string()])?;
        if n == 0 {
            return Err(StoreError::NotFound(id));
        }
        Ok(())
    }

    // Exactly one G.O.A.T. at a time; returns every id whose flag changed.
    pub fn set_goat(&mut self, id: Option<GameId>, now: OffsetDateTime) -> Result<Vec<GameId>, StoreError> {
        let tx = self.conn.transaction()?;
        let mut changed = Vec::new();
        {
            let mut st = tx.prepare("SELECT id FROM games WHERE is_goat = 1")?;
            let rows = st.query_map([], |r| r.get::<_, String>(0))?;
            for r in rows {
                if let Ok(gid) = GameId::from_str(&r?) {
                    if Some(gid) != id {
                        changed.push(gid);
                    }
                }
            }
        }
        let ts = fmt_ts(now);
        tx.execute("UPDATE games SET is_goat = 0, updated_at = ?1 WHERE is_goat = 1", [&ts])?;
        if let Some(gid) = id {
            let n = tx
                .execute("UPDATE games SET is_goat = 1, updated_at = ?1 WHERE id = ?2", params![ts, gid.to_string()])?;
            if n == 0 {
                return Err(StoreError::NotFound(gid));
            }
            changed.push(gid);
        }
        tx.commit()?;
        Ok(changed)
    }

    // User-confirmed merge: move sessions and genres to `keep`, then delete `absorb`.
    pub fn merge_games(&mut self, keep: &Game, absorb: GameId) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        let absorb_s = absorb.to_string();
        let keep_s = keep.id.to_string();
        // At most one open session per game: close any open session of the absorbed game first.
        tx.execute(
            "UPDATE play_sessions SET ended_at = last_heartbeat WHERE game_id = ?1 AND ended_at IS NULL",
            [&absorb_s],
        )?;
        tx.execute("UPDATE play_sessions SET game_id = ?1 WHERE game_id = ?2", params![keep_s, absorb_s])?;
        // Free unique columns before the kept row claims them.
        tx.execute("DELETE FROM games WHERE id = ?1", [&absorb_s])?;
        let n = codec::update_game(&tx, keep)?;
        if n == 0 {
            return Err(StoreError::NotFound(keep.id));
        }
        tx.commit()?;
        Ok(())
    }

    // Set or clear the install; detect path must already be canonical.
    pub fn set_install(
        &mut self,
        id: GameId,
        install: Option<&Install>,
        now: OffsetDateTime,
    ) -> Result<(), StoreError> {
        let (launch, detect, name) = codec::install_columns(install)?;
        let n = self.conn.execute(
            "UPDATE games SET launch_target = ?1, detect_exe_path = ?2, detect_exe_name = ?3, updated_at = ?4 WHERE id = ?5",
            params![launch, detect, name, fmt_ts(now), id.to_string()],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound(id));
        }
        Ok(())
    }

    pub fn set_cover_hash(&mut self, id: GameId, hash: Option<&str>) -> Result<(), StoreError> {
        self.conn.execute("UPDATE games SET cover_hash = ?1 WHERE id = ?2", params![hash, id.to_string()])?;
        Ok(())
    }

    // --- Playtime --------------------------------------------------------

    // Derived stats for every game, from the game_playtime view.
    pub fn play_stats_all(&self) -> Result<HashMap<GameId, PlayStats>, StoreError> {
        let mut st = self.conn.prepare("SELECT game_id, minutes, play_count, last_played FROM game_playtime")?;
        let rows = st.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, i64>(2)?, r.get::<_, Option<String>>(3)?))
        })?;
        let mut out = HashMap::new();
        for row in rows {
            let (id, minutes, count, last) = row?;
            let Ok(gid) = GameId::from_str(&id) else { continue };
            out.insert(gid, codec::play_stats(minutes, count, last));
        }
        Ok(out)
    }

    pub fn play_stats(&self, id: GameId) -> Result<PlayStats, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT minutes, play_count, last_played FROM game_playtime WHERE game_id = ?1",
                [id.to_string()],
                |r| Ok((r.get::<_, f64>(0)?, r.get::<_, i64>(1)?, r.get::<_, Option<String>>(2)?)),
            )
            .optional()?;
        Ok(row.map(|(m, c, l)| codec::play_stats(m, c, l)).unwrap_or_default())
    }

    // Games whose playtime includes an imported estimate.
    pub fn games_with_estimates(&self) -> Result<HashSet<GameId>, StoreError> {
        let mut st = self.conn.prepare("SELECT DISTINCT game_id FROM play_sessions WHERE source = 'manual'")?;
        let rows = st.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = HashSet::new();
        for r in rows {
            if let Ok(id) = GameId::from_str(&r?) {
                out.insert(id);
            }
        }
        Ok(out)
    }

    // --- Sessions --------------------------------------------------------

    pub fn open_session(&mut self, game: GameId, at: OffsetDateTime, source: SessionSource) -> Result<i64, StoreError> {
        let ts = fmt_ts(at);
        self.conn.execute(
            "INSERT INTO play_sessions (game_id, started_at, ended_at, last_heartbeat, source) VALUES (?1, ?2, NULL, ?2, ?3)",
            params![game.to_string(), ts, source.as_str()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn heartbeat(&mut self, session_id: i64, at: OffsetDateTime) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE play_sessions SET last_heartbeat = ?1 WHERE id = ?2 AND ended_at IS NULL",
            params![fmt_ts(at), session_id],
        )?;
        Ok(())
    }

    pub fn close_session(&mut self, session_id: i64, at: OffsetDateTime) -> Result<(), StoreError> {
        let ts = fmt_ts(at);
        // Never end before the start, whatever the clock says.
        self.conn.execute(
            "UPDATE play_sessions SET ended_at = MAX(?1, started_at), last_heartbeat = MAX(?1, started_at) WHERE id = ?2",
            params![ts, session_id],
        )?;
        Ok(())
    }

    pub fn delete_session(&mut self, session_id: i64) -> Result<(), StoreError> {
        self.conn.execute("DELETE FROM play_sessions WHERE id = ?1", [session_id])?;
        Ok(())
    }

    pub fn open_sessions(&self) -> Result<Vec<OpenSession>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT id, game_id, started_at, last_heartbeat, source FROM play_sessions WHERE ended_at IS NULL",
        )?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, gid, s, hb, src) = row?;
            let parsed = (|| -> Result<OpenSession, StoreError> {
                Ok(OpenSession {
                    id,
                    game_id: GameId::from_str(&gid).map_err(|e| StoreError::Corrupt(e.to_string()))?,
                    started_at: parse_ts(&s)?,
                    last_heartbeat: parse_ts(&hb)?,
                    source: SessionSource::from_str(&src).map_err(|e| StoreError::Corrupt(e.to_string()))?,
                })
            })();
            match parsed {
                Ok(o) => out.push(o),
                Err(e) => tracing::error!(session = id, error = %e, "unreadable open session"),
            }
        }
        Ok(out)
    }

    // Recent sessions for one game, newest first.
    pub fn sessions_for(&self, game: GameId, limit: u32) -> Result<Vec<SessionRecord>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT id, started_at, ended_at, last_heartbeat, source FROM play_sessions
             WHERE game_id = ?1 ORDER BY started_at DESC LIMIT ?2",
        )?;
        let rows = st.query_map(params![game.to_string(), limit], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, s, e, hb, src) = row?;
            let started = parse_ts(&s)?;
            let ended = e.as_deref().map(parse_ts).transpose()?;
            let end_for_len = match ended {
                Some(t) => t,
                None => parse_ts(&hb)?,
            };
            out.push(SessionRecord {
                id,
                started_at: started,
                ended_at: ended,
                minutes: moodex_core::session::span_minutes(started, end_for_len),
                source: SessionSource::from_str(&src).map_err(|e| StoreError::Corrupt(e.to_string()))?,
            });
        }
        Ok(out)
    }

    // --- Bulk import -----------------------------------------------------

    // Insert games and closed sessions in ONE transaction: all or nothing.
    pub fn import_batch(
        &mut self,
        games: &[Game],
        sessions: &[(GameId, OffsetDateTime, OffsetDateTime, SessionSource)],
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        for g in games {
            codec::insert_game(&tx, g)?;
        }
        for (gid, s, e, src) in sessions {
            let end = if e < s { *s } else { *e };
            tx.execute(
                "INSERT INTO play_sessions (game_id, started_at, ended_at, last_heartbeat, source) VALUES (?1, ?2, ?3, ?3, ?4)",
                params![gid.to_string(), fmt_ts(*s), fmt_ts(end), src.as_str()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    // --- Settings --------------------------------------------------------

    pub fn settings(&self) -> Result<HashMap<String, serde_json::Value>, StoreError> {
        let mut st = self.conn.prepare("SELECT key, value FROM settings")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = HashMap::new();
        for row in rows {
            let (k, v) = row?;
            match serde_json::from_str(&v) {
                Ok(val) => {
                    out.insert(k, val);
                }
                Err(e) => tracing::warn!(key = %k, error = %e, "ignoring malformed setting"),
            }
        }
        Ok(out)
    }

    pub fn set_setting(&mut self, key: &str, value: &serde_json::Value) -> Result<(), StoreError> {
        let text = serde_json::to_string(value).map_err(|e| StoreError::Corrupt(e.to_string()))?;
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, text],
        )?;
        Ok(())
    }

    // --- HTTP cache ------------------------------------------------------

    pub fn cache_get(&self, url: &str) -> Result<Option<CachedResponse>, StoreError> {
        let row = self
            .conn
            .query_row("SELECT body, etag, fetched_at, expires_at FROM http_cache WHERE url = ?1", [url], |r| {
                Ok((
                    r.get::<_, Vec<u8>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .optional()?;
        match row {
            None => Ok(None),
            Some((body, etag, f, e)) => {
                Ok(Some(CachedResponse { body, etag, fetched_at: parse_ts(&f)?, expires_at: parse_ts(&e)? }))
            }
        }
    }

    pub fn cache_put(&mut self, url: &str, resp: &CachedResponse) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO http_cache (url, body, etag, fetched_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(url) DO UPDATE SET body = excluded.body, etag = excluded.etag,
             fetched_at = excluded.fetched_at, expires_at = excluded.expires_at",
            params![url, resp.body, resp.etag, fmt_ts(resp.fetched_at), fmt_ts(resp.expires_at)],
        )?;
        Ok(())
    }

    // Drop cache entries far past expiry.
    pub fn cache_prune(&mut self, older_than: OffsetDateTime) -> Result<usize, StoreError> {
        Ok(self.conn.execute("DELETE FROM http_cache WHERE expires_at < ?1", [fmt_ts(older_than)])?)
    }

    // --- Backup ----------------------------------------------------------

    // Consistent snapshot to `dest`, written to a temp name then renamed.
    pub fn backup_to(&self, dest: &Utf8Path) -> Result<(), StoreError> {
        backup::vacuum_into(&self.conn, dest)
    }

    // Flush the WAL into the main file; used before shutdown.
    pub fn checkpoint(&self) -> Result<(), StoreError> {
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
