-- 001_init.sql - Initial schema (spec 6.1).

CREATE TABLE schema_version (version INTEGER NOT NULL);

-- ONE table for all games; "My PC Games" is the view WHERE launch_target IS NOT NULL.
CREATE TABLE games (
    id              TEXT    PRIMARY KEY,
    rawg_id         INTEGER UNIQUE,
    title           TEXT    NOT NULL,
    sort_title      TEXT    NOT NULL,
    cover_url       TEXT,
    cover_hash      TEXT,
    release_date    TEXT,
    status          TEXT    NOT NULL DEFAULT 'backlog'
                            CHECK (status IN ('playing','completed','backlog','dropped')),
    rating          INTEGER CHECK (rating IS NULL OR rating BETWEEN 1 AND 5),
    notes           TEXT    NOT NULL DEFAULT '',
    is_goat         INTEGER NOT NULL DEFAULT 0 CHECK (is_goat IN (0,1)),
    launch_target   TEXT,
    detect_exe_path TEXT,
    detect_exe_name TEXT,
    added_at        TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL
);

CREATE UNIQUE INDEX idx_games_detect_exe ON games(detect_exe_path)
       WHERE detect_exe_path IS NOT NULL;
CREATE INDEX idx_games_sort_title ON games(sort_title);
CREATE INDEX idx_games_status     ON games(status);

-- Playtime is DERIVED from immutable rows, never a mutable float.
CREATE TABLE play_sessions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id         TEXT    NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    started_at      TEXT    NOT NULL,
    ended_at        TEXT,
    last_heartbeat  TEXT    NOT NULL,
    source          TEXT    NOT NULL
                            CHECK (source IN ('job_object','wmi','manual')),
    CHECK (ended_at IS NULL OR ended_at >= started_at)
);

CREATE INDEX idx_sessions_game ON play_sessions(game_id, started_at DESC);
CREATE UNIQUE INDEX idx_sessions_one_open ON play_sessions(game_id)
       WHERE ended_at IS NULL;

CREATE TABLE genres (
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    genre   TEXT NOT NULL,
    PRIMARY KEY (game_id, genre)
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Stale-while-revalidate cache for RAWG responses.
CREATE TABLE http_cache (
    url         TEXT PRIMARY KEY,
    body        BLOB NOT NULL,
    etag        TEXT,
    fetched_at  TEXT NOT NULL,
    expires_at  TEXT NOT NULL
);

-- Derived playtime. Queried, never stored.
CREATE VIEW game_playtime AS
SELECT g.id AS game_id,
       COALESCE(SUM(
           (julianday(COALESCE(s.ended_at, s.last_heartbeat))
          - julianday(s.started_at)) * 1440.0
       ), 0.0) AS minutes,
       COUNT(s.id) AS play_count,
       MAX(COALESCE(s.ended_at, s.last_heartbeat)) AS last_played
FROM games g LEFT JOIN play_sessions s ON s.game_id = g.id
GROUP BY g.id;
