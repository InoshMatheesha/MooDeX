# MooDeX 2.0 — Rust MVP Build Specification

> **Status:** Build specification / implementation prompt
> **Target:** A from-scratch rewrite of MooDeX in Rust, shipped as a professionally signed Windows executable
> **Supersedes:** MooDeX v1.3 (C# / WPF / .NET 10)
> **Audience:** The engineer or coding agent implementing this. Read §0 and §1 before writing any code.

---

## 0. How to use this document

This is not a tutorial and not a wishlist. It is a contract.

* **§1 Non-negotiables** are binding rules. Every one of them exists because the v1 codebase broke it. If an implementation choice conflicts with §1, §1 wins.
* **§2** maps each defect found in the v1 audit to the rule it produced. Do not "simplify" a rule without reading why it exists.
* **§4–§12** specify subsystems with real function signatures. Signatures are the intended shape, not literal copy-paste; types may be refined, but the *contract* — what is owned, what can fail, what is returned — must hold.
* **§13 Professional EXE procedure** is mandatory for any build that leaves your machine. There is no "just send them the exe" path in this project.
* **§16 Milestones** define the MVP. Anything marked `[POST-MVP]` must not block the first release.

**Prime directive:** v1's problem was never performance. It was that four components independently owned the same state and papered over the resulting races with `Math.Max`. Rust will not save you from repeating that mistake. The architecture in §5 is the actual fix. Implement it faithfully.

---

## 1. Non-negotiables

1. **One owner of state.** Exactly one task owns the library in memory. Nothing else reads or writes the database directly. All mutation goes through message passing.
2. **No polling for state.** No timer anywhere re-reads storage to discover what changed. State changes are *pushed* as events.
3. **Identity is a UUID.** Games are never matched, merged, or deduplicated by title. Ever.
4. **Every write is crash-safe.** A power cut must never corrupt or truncate a user's library.
5. **No secrets in the binary.** No API key, token, or credential is compiled into the executable or committed to the repository.
6. **No substring matching for process detection.** A process is a game only on a canonical full-path match.
7. **No `unwrap()` / `expect()` / `panic!` on any path reachable from user input, disk, or network.** `clippy::unwrap_used` is denied at the workspace level.
8. **Errors are logged, never swallowed.** No empty catch, no discarded `Result`.
9. **No unbounded concurrency.** Every fan-out — image downloads, API calls — passes through a semaphore with an explicit limit.
10. **Every list that can exceed 30 items is virtualized.**
11. **The repository contains source only.** No build output, no IDE state, no installers, no vendored DLLs.
12. **Every shipped binary is Authenticode-signed and timestamped.** Every update is independently minisign-signed.

---

## 2. Audit carry-over: v1 defects to binding rules

The v1 audit findings, each with the rule it generated. This table is the justification layer for §1.

| # | v1 defect | Location in v1 | Rule produced |
|---|---|---|---|
| 1 | API key hardcoded as a `const string`, published to GitHub and extractable from the shipped exe | `Services/RawgApiService.cs:12` | §1.5 — key lives in a server-side proxy; optional user key in Credential Manager (§9) |
| 2 | 100 of 163 tracked files were build artifacts; `.git` reached 136 MB; a 10 MB third-party DLL and a 6.5 MB installer were committed; no root `.gitignore` | repo-wide | §1.11 — repo hygiene gate in CI (§13.1) |
| 3 | Library written with a bare `File.WriteAllText` — power loss mid-write destroys the user's collection | `StorageService.cs:181,252,316` | §1.4 — SQLite + WAL, plus `tempfile` persist for all file writes (§6) |
| 4 | Four components independently load, mutate and save the same JSON, with races hidden behind `Math.Max(playtime)` merges | `ProcessMonitor`, `LibraryViewModel`, `MyGamesViewModel`, `DashboardViewModel` | §1.1 — a single actor owns state (§7) |
| 5 | Identity fell back to case-insensitive title matching; two games with the same name silently merged into one | `StorageService.cs:85,124,268` | §1.3 — UUID identity, user-confirmed merge only (§6.2) |
| 6 | Process detection by substring: `p.Contains(cleanExeName)` made `game.exe` match `GameBarPresenceWriter` and Steam's `GameOverlayUI`, accruing playtime for games that were not being played | `ProcessMonitor.cs:159` | §1.6 — canonical path match only (§8.4) |
| 7 | `Process.GetProcesses()` called every 10 s and never disposed — a few hundred leaked handles per scan, in a tray app that runs for days | `ProcessMonitor.cs:125` | §1.2 — event-driven detection, RAII handles (§8) |
| 8 | The whole library JSON re-read and re-parsed every 10 s by the monitor, every 5 s per open tab, and again on every sort, save and dialog close | `ProcessMonitor.cs:125`, `LibraryViewModel.cs:244`, `MyGamesViewModel.cs:227`, `ApplySortAsync` | §1.2 — no polling |
| 9 | `ItemsControl` + `WrapPanel` inside a `ScrollViewer` — zero virtualization, so all 50 result cards were realized and all 50 images fetched and decoded at once, unbounded | `Views/DiscoverView.xaml:147` | §1.9, §1.10 — virtualized grid, semaphore-bounded pipeline (§10, §12) |
| 10 | `ReplaceRange` raised `NotifyCollectionChangedAction.Reset`, rebuilding the entire visual tree on every refresh — the source of the UI flicker | `Helpers/RangeObservableCollection.cs:19` | §12.3 — keyed, incremental DOM updates |
| 11 | Eight `async void` methods, including `LaunchGame` and the image-loading property callback: an exception in any of them is uncatchable and kills the process | `ImageHelper.cs:28`, `MyGamesViewModel.cs:443`, and six others | §1.7 — all fallible work returns `Result` |
| 12 | Error reporting via `Console.WriteLine` in a `WinExe`, which has no console attached — five error paths wrote to nowhere, and services swallowed exceptions silently | `StorageService`, `RawgApiService`, `ImageHelper` | §1.8 — `tracing` to a rolling file (§11) |
| 13 | No dependency injection; every navigation click constructed a fresh ViewModel, which constructed its own services and re-read the disk | `MainViewModel.cs:46-50` | §1.1 — services constructed once and injected (§5) |
| 14 | A structural split into `games` + `local_games` with auto-created mirror entries kept in sync by `Math.Max`, generating an entire bug class | `StorageService.LoadGames` | §6.1 — one unified table; "PC Games" is a filter |
| 15 | Playtime accumulated as a float with a clamp hack (`elapsed > 5 min` becomes `10 s`), so sleep and resume silently lost hours and the figure could never be audited | `ProcessMonitor.cs:118-120` | §8.2 — immutable session rows; playtime is a SUM |
| 16 | Launcher-mediated games (Steam, Epic, Ubisoft) tracked the launcher stub, which exits immediately — so playtime was simply wrong for most modern games | `ProcessMonitor` design | §8.3 — separate the launch target from the detect target |
| 17 | Installer unsigned (`Status: NotSigned`), no `FileVersion` resource, `CompanyName: Moody69`, `ProductName: MooDex` inconsistent with the branding — SmartScreen blocks every download | `MooDeX_v1.0 Setup.exe` | §1.12, §13 — full signing pipeline |
| 18 | README claimed the ".NET 10 Runtime bundled with the installer"; the installer is 6.5 MB and that runtime is about 55 MB, so users without .NET hit a runtime error | `README.md` | §13.6 — self-contained bundle, documented WebView2 handling |
| 19 | No tests, no CI, no solution file; the csproj said v1.3 while the committed installer was v1.0 | repo-wide | §14, §15 — CI gates and a single source of version truth |
| 20 | Dead config surface: `GamesDataWrapper.NowPlaying` was persisted but never read | `StorageService.cs:330` | §6.1 — the schema carries no unused fields |

**Carried forward as good.** v1's `UpdateDownloader` used the correct temp-file-then-rename pattern with a streamed download and real progress reporting, and `App.xaml.cs` installed a genuine crash logger with three handler hooks. Both behaviours are preserved, in §11 and §9.

---

## 3. Stack decision

### 3.1 Chosen stack

| Layer | Choice | Version floor |
|---|---|---|
| Shell | **Tauri 2** | 2.12 |
| Core language | Rust, 2021 edition | 1.83+, pinned via `rust-toolchain.toml` |
| Async runtime | `tokio`, multi-thread | 1.x |
| Frontend | TypeScript + Vite; no UI framework required | — |
| Persistence | SQLite via `rusqlite`, bundled | 0.32+ |
| HTTP | `reqwest` with rustls | 0.12 |
| Windows APIs | `windows` (official Microsoft bindings) | 0.58+ |
| Process events | `wmi` | 0.18 |
| Credentials | `keyring` | 3.x |
| Images | `image` + `moka` for the in-memory LRU | — |
| Logging | `tracing` + `tracing-appender` | — |
| Errors | `thiserror` in libraries, `anyhow` in the binary | — |

### 3.2 Why Tauri

* **You already own the frontend.** `MooDeX Web/` is a working dark-themed UI: 951 lines of CSS, 522 of HTML. Tauri is the only Rust option that lets you carry that forward instead of rebuilding it from nothing. This single fact decides the stack.
* It is the most production-ready Rust desktop option on Windows, and it uses the system WebView2 rather than bundling a browser engine.
* The ecosystem supplies tray, autostart, single-instance, window-state and a **signed** updater as first-party plugins — all five of which v1 hand-rolled, including a Win32 `RegisterWindowMessage` broadcast for single-instance.

### 3.3 Rejected alternatives

| Option | Why not |
|---|---|
| **Keep WPF/C#** | Technically the lowest-risk path, and genuinely defensible. Rejected by project decision. Recorded for the file: WPF *cannot* be Native AOT compiled, because Native AOT requires trimming and trimming support for WPF is disabled in the .NET SDK — almost no WPF app survives it. So a small, instantly-starting single binary was never reachable on the old stack. That is the one concrete technical gain this rewrite buys. |
| **`iced`** | The least proven of the Rust GUI options; development pace has lagged the others. |
| **`egui`** | Excellent for tools, dashboards and editors. Immediate-mode rendering will not read as a polished consumer product, which is exactly MooDeX's strength. |
| **`slint`** | Capable and genuinely polished, but the licensing carries an attribution or royalty obligation, and choosing it means discarding the existing web UI. |
| **WinUI 3 / C++** | The largest implementation cost, and it abandons the Rust objective. |

### 3.4 Accepted trade-off

Tauri on Windows requires the **WebView2 runtime**. It ships with Windows 11 and is present on the large majority of Windows 10 installs. §13.6 specifies the bootstrapper policy for the remainder. This dependency is the price of reusing the frontend, and it is accepted deliberately rather than discovered later.

---

## 4. Workspace layout

```
moodex/
├── Cargo.toml                  # workspace manifest
├── Cargo.lock                  # COMMITTED
├── rust-toolchain.toml         # pinned toolchain
├── deny.toml                   # cargo-deny: licenses, advisories, bans
├── .gitignore                  # target/, dist/, node_modules/, *.pdb
├── CHANGELOG.md
├── crates/
│   ├── moodex-core/            # domain types + pure logic. NO I/O.
│   ├── moodex-store/           # SQLite, migrations, crash-safe writes
│   ├── moodex-rawg/            # RAWG client, HTTP cache, rate limiting
│   ├── moodex-images/          # two-tier image cache + transcode
│   ├── moodex-winproc/         # Windows: job objects, WMI, launching
│   └── moodex-library/         # the actor: single owner of state
├── app/
│   ├── src-tauri/
│   │   ├── Cargo.toml
│   │   ├── tauri.conf.json
│   │   ├── build.rs
│   │   ├── moodex.manifest     # DPI / UAC / long paths
│   │   ├── icons/
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/       # IPC surface (§11)
│   │       ├── events.rs
│   │       ├── tray.rs
│   │       └── logging.rs
│   └── ui/                     # Vite + TS frontend, seeded from MooDeX Web/
├── proxy/                      # Vercel edge function holding the RAWG key
└── .github/workflows/
    ├── ci.yml                  # fmt, clippy, test, deny, audit
    └── release.yml             # build, sign, bundle, publish
```

### 4.1 Dependency rule

Dependencies point **inward only**:

```
app  ──►  moodex-library  ──►  moodex-store   ──►  moodex-core
                          ──►  moodex-rawg    ──►  moodex-core
                          ──►  moodex-images  ──►  moodex-core
                          ──►  moodex-winproc ──►  moodex-core
```

`moodex-core` depends on nothing beyond `serde`, `uuid`, `time` and `thiserror`. It performs no I/O, which makes every rule inside it unit-testable without a filesystem, a network, or even Windows. This is where v1's playtime arithmetic and status logic belonged and never lived — which is the direct reason it was never tested.

### 4.2 Workspace lint policy

```toml
[workspace.lints.clippy]
unwrap_used        = "deny"
expect_used        = "deny"
panic              = "deny"
todo               = "deny"
dbg_macro          = "deny"
float_cmp          = "deny"
await_holding_lock = "deny"   # the v1 race class, made unrepresentable
```

`unwrap_used` and `expect_used` are permitted under `#[cfg(test)]` only.

---

## 5. Runtime architecture

```
                   ┌──────────────────────────────────────┐
                   │   WebView2 frontend (TypeScript)     │
                   │   virtualized grid · owns no state   │
                   └───────┬──────────────────────▲───────┘
                 invoke()  │                      │  emit()
                 commands  │                      │  events
                   ┌───────▼──────────────────────┴───────┐
                   │        Tauri command layer           │
                   │   thin: validate → send → map error  │
                   └───────────────┬──────────────────────┘
                                   │ mpsc<LibraryCommand>
                   ┌───────────────▼──────────────────────┐
                   │          LibraryActor  (§7)          │
                   │  *** SOLE OWNER OF LIBRARY STATE *** │
                   │   owns: Vec<Game>, open sessions     │
                   │   broadcasts: LibraryEvent           │
                   └───┬────────┬──────────┬───────────┬──┘
                       │        │          │           │
              ┌────────▼──┐ ┌───▼─────┐ ┌──▼───────┐ ┌─▼──────────┐
              │   Store   │ │  Rawg   │ │  Images  │ │  WinProc   │
              │  SQLite   │ │ client  │ │  cache   │ │ job + WMI  │
              │   (§6)    │ │  (§9)   │ │  (§10)   │ │   (§8)     │
              └───────────┘ └─────────┘ └──────────┘ └─┬──────────┘
                                                       │ PlayEvent
                                                       └──► back to actor
```

**The rule this diagram encodes:** state flows in one direction. The frontend never holds authoritative state; it renders what it is told and sends intents. No component other than the actor may open the database for writing. The v1 bug class in §2 row 4 becomes *unrepresentable*, because there is no second writer left to race with.

---

## 6. Data model

### 6.1 Schema

SQLite in WAL mode, with `foreign_keys = ON` and `synchronous = NORMAL`. Migrations are forward-only, numbered, and embedded in the binary.

```sql
-- migrations/001_init.sql

CREATE TABLE schema_version (version INTEGER NOT NULL);

-- ONE table for all games. v1's games/local_games split is deleted.
-- "My PC Games" is simply the view WHERE launch_target IS NOT NULL.
CREATE TABLE games (
    id              TEXT    PRIMARY KEY,          -- UUIDv7: sortable by creation
    rawg_id         INTEGER UNIQUE,               -- NULL for manual entries
    title           TEXT    NOT NULL,
    sort_title      TEXT    NOT NULL,             -- lowercased, leading articles stripped
    cover_url       TEXT,                         -- remote source URL
    cover_hash      TEXT,                         -- content address into the image cache
    release_date    TEXT,                         -- ISO-8601 date
    status          TEXT    NOT NULL DEFAULT 'backlog'
                            CHECK (status IN ('playing','completed','backlog','dropped')),
    rating          INTEGER CHECK (rating IS NULL OR rating BETWEEN 1 AND 5),
    notes           TEXT    NOT NULL DEFAULT '',
    is_goat         INTEGER NOT NULL DEFAULT 0 CHECK (is_goat IN (0,1)),

    -- Launch/detect split (§8.3). NULL launch_target means "not installed".
    launch_target   TEXT,                         -- JSON: Exe | Uri | Shortcut
    detect_exe_path TEXT,                         -- canonicalized absolute path
    detect_exe_name TEXT,                         -- lowercased filename, display only

    added_at        TEXT    NOT NULL,             -- RFC-3339 UTC
    updated_at      TEXT    NOT NULL
);

CREATE UNIQUE INDEX idx_games_detect_exe ON games(detect_exe_path)
       WHERE detect_exe_path IS NOT NULL;
CREATE INDEX idx_games_sort_title ON games(sort_title);
CREATE INDEX idx_games_status     ON games(status);

-- Playtime is DERIVED from immutable rows. Never a mutable float. (§2 row 15)
CREATE TABLE play_sessions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id         TEXT    NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    started_at      TEXT    NOT NULL,
    ended_at        TEXT,                         -- NULL means currently open
    last_heartbeat  TEXT    NOT NULL,             -- crash / sleep recovery anchor
    source          TEXT    NOT NULL              -- how it was detected
                            CHECK (source IN ('job_object','wmi','manual')),
    CHECK (ended_at IS NULL OR ended_at >= started_at)
);

CREATE INDEX idx_sessions_game ON play_sessions(game_id, started_at DESC);
CREATE UNIQUE INDEX idx_sessions_one_open ON play_sessions(game_id)
       WHERE ended_at IS NULL;                    -- at most one open session per game

CREATE TABLE genres (
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    genre   TEXT NOT NULL,
    PRIMARY KEY (game_id, genre)
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL                           -- JSON scalar
);

-- Stale-while-revalidate cache for RAWG responses (§9.4)
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
```

**Why sessions beat a float.** v1 stored `playtime` as a mutable `double` incremented every 10 s, with a clamp — `if elapsed > 5 min then elapsed = 10 s` — that silently discarded hours whenever the machine slept. That number could drift, could be double-counted by two writers, and could never be audited or corrected. A session table is append-mostly, auditable, correctable, crash-recoverable, and it unlocks play-history features v1 could not express at all. Total playtime becomes `SELECT minutes FROM game_playtime`: derived, so it cannot disagree with itself.

### 6.2 Core types

```rust
// crates/moodex-core/src/model.rs

pub struct GameId(pub Uuid);                     // UUIDv7

pub struct Game {
    pub id: GameId,
    pub rawg_id: Option<i64>,
    pub title: String,
    pub cover: CoverRef,
    pub release_date: Option<Date>,
    pub status: Status,
    pub rating: Option<Rating>,                  // 1..=5, newtype-validated
    pub notes: String,
    pub is_goat: bool,
    pub install: Option<Install>,                // None means not installed
    pub genres: Vec<String>,
    pub added_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The launch/detect split. v1 conflated these, which is precisely why
/// Steam, Epic and Ubisoft games recorded wrong playtime. (§2 row 16)
pub struct Install {
    /// What we execute when the user presses Play.
    pub launch: LaunchTarget,
    /// What we watch to decide the game is running. May differ from `launch`.
    pub detect_exe: Option<Utf8PathBuf>,         // canonicalized, absolute
}

pub enum LaunchTarget {
    Exe { path: Utf8PathBuf, args: Vec<String>, working_dir: Option<Utf8PathBuf> },
    Uri { uri: String },                         // steam://rungameid/440
    Shortcut { path: Utf8PathBuf },              // .lnk / .url
}

pub enum Status { Playing, Completed, Backlog, Dropped }

/// Derived, never stored. Built from play_sessions.
pub struct PlayStats {
    pub minutes: f64,
    pub play_count: u32,
    pub last_played: Option<OffsetDateTime>,
}

pub enum CoverRef {
    None,
    Remote { url: String },
    Cached { hash: ContentHash, url: String },
}
```

### 6.3 Pure logic that belongs in `moodex-core`

Each of these is a function over owned data with no I/O, and is therefore unit-testable without Windows, a disk, or a network. In v1 this logic was scattered through ViewModels and storage code, where it could not be tested at all.

```rust
/// "The Witcher 3" -> "witcher 3". Used for ordering, never for identity.
pub fn sort_title(title: &str) -> String;

/// 755.0 -> "12h 35m"; 0.0 -> "0m". Replaces v1's PlaytimeFormatted.
pub fn format_playtime(minutes: f64) -> String;

/// Aggregate for the dashboard. A pure fold over a slice.
pub fn dashboard_stats(games: &[(Game, PlayStats)]) -> DashboardStats;

/// Candidate duplicates for the merge-suggestion UI.
/// Returns SUGGESTIONS ONLY. Never merges anything. (§1.3)
pub fn find_duplicate_candidates(games: &[Game]) -> Vec<DuplicateCandidate>;

/// Fuzzy-rank RAWG results against a filename stem, for cover matching.
/// "Cyberpunk2077.exe" -> "cyberpunk 2077" must rank the right title first.
pub fn rank_matches(query: &str, candidates: &[RawgGame]) -> Vec<Scored<RawgGame>>;

/// Derive a human title from an executable path, falling back to the parent
/// folder when the filename is uninformative ("bin/x64/launcher.exe").
pub fn title_from_exe_path(path: &Utf8Path) -> String;

/// Close an open session orphaned by a crash or sleep, anchoring the end at
/// the last heartbeat rather than inventing a value.
pub fn recover_orphan_session(open: &OpenSession, now: OffsetDateTime) -> SessionClose;
```

---

## 7. `moodex-library` — the actor

The single most important component. It replaces v1's four competing state owners.

### 7.1 Shape

```rust
pub struct LibraryHandle {
    tx: mpsc::Sender<LibraryCommand>,
    events: broadcast::Sender<LibraryEvent>,
}

impl LibraryHandle {
    /// Construct, run recovery (§7.4), and spawn the actor task.
    pub async fn spawn(store: Store, deps: Deps) -> Result<Self, LibraryError>;

    /// Request/response over a oneshot channel. Every call is cancel-safe.
    async fn call<R>(&self, make: impl FnOnce(oneshot::Sender<R>) -> LibraryCommand)
        -> Result<R, LibraryError>;

    pub fn subscribe(&self) -> broadcast::Receiver<LibraryEvent>;
}

enum LibraryCommand {
    ListGames  { filter: GameFilter, sort: SortOrder, reply: oneshot::Sender<Vec<GameView>> },
    GetGame    { id: GameId, reply: oneshot::Sender<Option<GameView>> },
    AddGame    { draft: GameDraft, reply: oneshot::Sender<Result<GameId, LibraryError>> },
    UpdateGame { id: GameId, patch: GamePatch, reply: oneshot::Sender<Result<(), LibraryError>> },
    DeleteGame { id: GameId, reply: oneshot::Sender<Result<(), LibraryError>> },
    SetInstall { id: GameId, install: Option<Install>, reply: oneshot::Sender<Result<(), LibraryError>> },
    SetGoat    { id: GameId, reply: oneshot::Sender<Result<(), LibraryError>> },
    MergeGames { keep: GameId, absorb: GameId, reply: oneshot::Sender<Result<(), LibraryError>> },
    Dashboard  { reply: oneshot::Sender<DashboardStats> },
    PlayEvent  (PlayEvent),                      // from WinProc; no reply
    Shutdown   { reply: oneshot::Sender<()> },
}

pub enum LibraryEvent {
    GamesChanged { ids: Vec<GameId> },           // granular, never "reload everything"
    SessionStarted { game_id: GameId, at: OffsetDateTime },
    SessionEnded   { game_id: GameId, minutes: f64 },
    PlaytimeTick   { game_id: GameId, minutes: f64 },
    CoverReady     { game_id: GameId, hash: ContentHash },
    ScanProgress   { done: usize, total: usize },
}
```

### 7.2 Invariants the actor enforces

* It is the **only** holder of a writable `Store`. `Store::open_rw` is called once, in `spawn`.
* It never holds a lock across an `await`. The `await_holding_lock` lint makes violations a compile error.
* Mutations are: apply to SQLite inside a transaction, then update the in-memory copy, then broadcast. If the transaction fails, memory is untouched and the error propagates. There is no path that leaves memory and disk disagreeing — which is the exact failure v1's `Math.Max` merges were trying and failing to patch over.
* `GamesChanged` carries the affected ids only. The frontend patches those cards. §2 row 10's full-tree rebuild has no equivalent here.

### 7.3 Backpressure

The command channel is bounded at 256. A full channel means the actor is wedged; commands then fail with `LibraryError::Busy` rather than growing memory without limit. `PlaytimeTick` is emitted at most once per 30 s per game and is dropped rather than queued if the channel is full, because it is advisory.

### 7.4 Startup recovery

Runs before the actor accepts its first command:

1. Apply pending migrations in one transaction.
2. Find sessions with `ended_at IS NULL`. For each, close it at `last_heartbeat` via `recover_orphan_session`, and log at `WARN` with the recovered duration. This is how a crash, a forced shutdown, or a sleep is reconciled — v1 simply lost that time.
3. Verify every `detect_exe_path` still exists. Missing paths mark the game `install_missing` for the UI to flag; **nothing is deleted automatically**.
4. Roll the daily backup (§6.4 / §15.4), retaining seven.
5. Emit `GamesChanged` with all ids so the frontend paints its first state.

---

## 8. `moodex-winproc` — play tracking, fully redesigned

This subsystem is where v1 was most wrong, and where the biggest functional win sits. It is the one place a Rust rewrite genuinely buys capability rather than just speed.

### 8.1 What v1 did and why it failed

```csharp
// ProcessMonitor.cs:159 — the bug
isRunning = runningProcesses.Any(p =>
    p == cleanExeName ||
    (cleanExeName.Length > 3 && (p.StartsWith(cleanExeName) || p.Contains(cleanExeName))));
```

Three independent failures:

1. **False positives.** A game whose exe is `game.exe` matches `GameBarPresenceWriter` and Steam's `GameOverlayUI`, both of which run permanently. That game accrued playtime forever. Short names like `launcher.exe` or `start.exe` cross-matched other games.
2. **False negatives.** Steam, Epic and Ubisoft titles are started through a launcher stub that exits within seconds. v1 watched the stub, so it recorded a few seconds for a four-hour session. Most modern PC games fall in this category, which means v1's headline feature was broken for the majority case.
3. **Cost.** `Process.GetProcesses()` every 10 s, enumerating every process on the machine, and never disposing the returned handles.

### 8.2 The session model

A play session is an immutable row, not an increment:

* **Open** on detected start: insert with `started_at = now`, `ended_at = NULL`.
* **Heartbeat** every 30 s while running: update `last_heartbeat` only. This is the sleep-and-crash anchor.
* **Close** on detected exit: set `ended_at = now`.
* **Recover** on startup: any still-open session closes at its `last_heartbeat` (§7.4).
* **Reject noise**: a session shorter than `min_session_seconds` (default 60) is deleted rather than kept, so an accidental launch does not pollute statistics.

Sleep and resume are handled correctly by construction: heartbeats stop while suspended, so the recovered end time is the last moment the game was observably alive. No clamp hack, and no invented value.

### 8.3 Launch target vs detect target

The key structural fix. These are two different things, and v1 had only one field for both.

| Scenario | `launch` | `detect_exe` | How tracking works |
|---|---|---|---|
| Plain DRM-free exe | `Exe { path }` | same path | Layer A: job object. Exact. |
| Steam game | `Uri { "steam://rungameid/440" }` | `.../Team Fortress 2/hl2.exe` | Layer B: job sees the stub exit, WMI catches the real exe |
| Epic / Ubisoft | `Shortcut { .lnk }` | the real game exe | Layer B |
| Emulator + ROM | `Exe { path, args }` | emulator exe | Layer A |
| Not installed | `None` | `None` | Not tracked, and honestly labelled as such |

### 8.4 Three-layer detection

```rust
/// Layer A — authoritative. Used whenever we start the process ourselves.
///
/// Creates an unnamed job object, assigns the child to it *before* the child
/// runs (create suspended, assign, resume), disallows breakaway, and waits on
/// an IO completion port for JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO.
///
/// Job membership is inherited, so grandchildren join automatically: the whole
/// process tree is covered. Exit detection is an OS event, not a poll.
pub async fn launch_tracked(target: &LaunchTarget)
    -> Result<TrackedLaunch, LaunchError>;

pub struct TrackedLaunch {
    pub job: JobObject,                  // RAII: handle closed on drop
    pub root_pid: u32,
    /// Resolves when the LAST process in the tree exits.
    pub completion: oneshot::Receiver<ExitReason>,
}

/// Layer B/C — event-driven detection for processes we did not start.
///
/// Subscribes to WMI `__InstanceCreationEvent WITHIN 2` over `Win32_Process`
/// and the matching deletion event. Chosen over `Win32_ProcessStartTrace`
/// deliberately: the trace class requires administrator rights, and MooDeX
/// runs as `asInvoker` (§13.3). An intrinsic event needs no elevation.
pub fn watch_processes(registry: Arc<DetectRegistry>)
    -> Result<impl Stream<Item = ProcEvent>, WatchError>;

/// The matching rule. §1.6 made executable.
///
/// Compares the canonicalized `ExecutablePath` reported by WMI against the
/// canonicalized `detect_exe_path`. Case-insensitive (Windows), 8.3-expanded,
/// symlink- and junction-resolved. NO substring, prefix, or stem matching.
/// A game with no `detect_exe_path` is NEVER auto-detected — it is simply
/// untracked, which is correct and honest.
pub fn match_process(ev: &ProcEvent, reg: &DetectRegistry) -> Option<GameId>;
```

**Why the layering is necessary.** Job objects cover the tree spawned *after* assignment, but a game started through `steam://` is spawned by the already-running Steam client, which is outside our job. The literature on job objects is explicit that breakaway must be disallowed and that detached children can still escape. So: Layer A is exact when it applies, and Layer B catches the launcher-mediated case that Layer A structurally cannot. Layer C is Layer B's subscription doing double duty for games started entirely outside MooDeX — a feature v1 only pretended to have, via the substring hack.

### 8.5 Launch flow

```rust
pub async fn play(&self, id: GameId) -> Result<(), PlayError>;
```

1. Load the game. No `install` → `PlayError::NotInstalled` (a typed error the UI renders inline, not v1's `MessageBox`).
2. For `Exe`: verify the path exists and is a regular file. Missing → `PlayError::ExecutableMissing { path }`; mark `install_missing` and prompt a re-link. Never fail silently.
3. `launch_tracked(target)` → `TrackedLaunch`.
4. Emit `PlayEvent::Started { game_id, source: JobObject }` to the actor, which opens a session and broadcasts `SessionStarted`.
5. Spawn a watcher task: heartbeat every 30 s, and await `completion`.
6. **Launcher-stub heuristic:** if the tree exits within `stub_threshold` (default 20 s) *and* the game has a `detect_exe`, do **not** close the session. Hand it to Layer B and log at `INFO` that tracking was promoted to WMI. This single rule is what makes Steam playtime correct.
7. On genuine exit: emit `PlayEvent::Ended`, actor closes the session, discards it if under `min_session_seconds`, and broadcasts `SessionEnded`.

### 8.6 Resource discipline

* `JobObject` and every `HANDLE` are newtypes implementing `Drop` with `CloseHandle`. The v1 handle leak (§2 row 7) is structurally impossible.
* `unsafe` is confined to `moodex-winproc`, each block carrying a `// SAFETY:` comment naming the invariant. No other crate contains `unsafe`.
* The WMI subscription is one long-lived COM apartment on a dedicated thread, not a per-scan connection.

### 8.7 `[POST-MVP]` Library importers

The session model and the launch/detect split make these natural follow-ons: read Steam `libraryfolders.vdf` and `appmanifest_*.acf`, Epic's `.item` manifests, Xbox/GamePass packages. Each yields a `GameDraft` with `Uri` launch and a resolved `detect_exe`. Explicitly out of MVP scope.

---

## 9. `moodex-rawg` — API client

### 9.1 Key handling (fixes §2 row 1)

Three tiers, in order:

1. **Default — server proxy.** The client calls `https://<your-proxy>/api/rawg/*`. The proxy, in `proxy/`, holds the key as a Vercel environment variable, injects it, rate-limits per IP, and forwards. **The key never reaches the client binary.** You already run a Vercel project for the web demo, so there is no new infrastructure.
2. **Optional — user's own key.** Settings accept a personal RAWG key, stored via the `keyring` crate in the **Windows Credential Manager** (DPAPI-protected per user). Never in the SQLite file, never in a config file, never in the binary.
3. **Absent** — Discover degrades to a clear "search unavailable" state. The local library stays fully functional. Offline is a supported mode, not a crash.

**Before writing any code: revoke the key `3e485ee2...` that is published in v1's git history.** It must be treated as compromised.

### 9.2 Client surface

```rust
pub struct RawgClient { /* reqwest::Client + semaphore + cache */ }

impl RawgClient {
    pub fn new(cfg: RawgConfig, cache: HttpCache) -> Result<Self, RawgError>;

    pub async fn search(&self, query: &str, page: u8)
        -> Result<Page<RawgGame>, RawgError>;
    pub async fn trending(&self)  -> Result<Page<RawgGame>, RawgError>;
    pub async fn upcoming(&self)  -> Result<Page<RawgGame>, RawgError>;
    pub async fn popular(&self)   -> Result<Page<RawgGame>, RawgError>;
    pub async fn details(&self, rawg_id: i64) -> Result<RawgGameDetail, RawgError>;
}

#[derive(thiserror::Error, Debug)]
pub enum RawgError {
    #[error("no API key configured")]            NoKey,
    #[error("rate limited; retry after {0:?}")]  RateLimited(Duration),
    #[error("offline")]                          Offline,
    #[error("upstream returned {0}")]            Upstream(StatusCode),
    #[error("malformed response: {0}")]          Decode(String),
}
```

Each variant maps to a specific UI message. v1 returned `new List<Game>()` for every failure, so "network down", "bad key" and "no results" were indistinguishable to the user.

### 9.3 Hardening

* `page_size` capped at 20 for Discover, not v1's 50 — paired with virtualization (§12) and infinite scroll.
* Explicit `User-Agent: MooDeX/<version>`, a 15 s timeout, and `Retry-After` honoured on 429.
* Semaphore of 4 concurrent requests.
* Debounced search at 300 ms, with the in-flight request **cancelled** on a new keystroke — Rust's drop-cancellation makes this correct by default, where v1 relied on a `CancellationTokenSource` dance.
* Responses parsed into typed structs with `#[serde(default)]` on optional fields. No hand-rolled `JsonDocument.TryGetProperty` ladders.

### 9.4 Stale-while-revalidate

v1 refetched Discover on every visit. New behaviour: serve from `http_cache` immediately if present, revalidate in the background with `If-None-Match`, and emit an update event only if the content actually changed. Trending/Popular TTL 6 h, Upcoming 24 h, search 15 min. Discover opens instantly and works offline.

---

## 10. `moodex-images` — cover pipeline

Replaces `ImageHelper.cs`, which was an `async void` dependency-property callback with no concurrency limit, no in-memory cache, and MD5 cache keys.

```rust
pub struct ImageCache { /* moka LRU + disk root + semaphore(6) */ }

impl ImageCache {
    pub fn new(root: Utf8PathBuf, budget_bytes: u64) -> Result<Self, ImageError>;

    /// Returns immediately from memory or disk if present; otherwise schedules
    /// a fetch and returns Pending. Never blocks the caller on the network.
    pub async fn get(&self, url: &str, variant: Variant)
        -> Result<CacheHit, ImageError>;

    /// Warm a batch, respecting the semaphore. Used for a visible viewport.
    pub async fn prefetch(&self, urls: &[String], variant: Variant);

    /// LRU eviction down to the byte budget. Called at startup and daily.
    pub async fn gc(&self) -> Result<GcReport, ImageError>;
}

pub enum Variant { Thumb, /* 320w, grid */  Hero /* 1280w, detail */ }
pub enum CacheHit { Ready(ContentHash), Pending }
```

Design points:

* **Content-addressed by SHA-256** of the source URL, not MD5. Not a security boundary, but MD5 trips supply-chain scanners and there is no reason to keep it.
* **Two tiers:** `moka` in-memory LRU of decoded `Thumb` bytes (default 128 MB) over a disk cache with a byte budget (default 500 MB) and LRU eviction. v1 had no memory tier at all, so scrolling back up re-decoded every image from disk.
* **Pre-generated variants.** Decode once, write both `Thumb` and `Hero` as WebP quality 82. v1 decoded a 500×250 JPEG at quality 75 for every use, including the detail view.
* **Bounded concurrency.** A semaphore of 6. v1 launched 50 simultaneous downloads whenever Discover opened.
* **Served over a custom protocol.** Register `moodex-img://<hash>/<variant>`; the webview streams bytes straight from the cache. No base64 in the IPC channel, and the browser's own image cache does its job.
* **Atomic writes.** `tempfile::NamedTempFile` then `persist`. A half-written cover is impossible, where v1 could leave one and then had to detect and delete it on the next read.

---

## 11. Observability

Fixes §2 row 12, where five error paths wrote to a console that a `WinExe` does not have.

```rust
pub fn init_logging(dir: &Utf8Path) -> Result<WorkerGuard, LogError>;
```

* `tracing-subscriber` with a `tracing-appender` daily rolling file at `%LOCALAPPDATA%\MooDeX\logs\moodex.log`, retaining seven days.
* `RUST_LOG` honoured; default `info`, `moodex=debug` in dev builds.
* Spans around every command, every launch, and every HTTP call, carrying `game_id` so one session's story can be read end to end.
* **Panic hook** writing the panic payload, the backtrace and the build version to the log *before* unwinding, preserving v1's one genuinely good idea (its three-handler crash logger) and improving on it.
* **Diagnostics pane in Settings:** show the app version and build hash, open the log folder, open the data folder, and display the last ten `WARN`/`ERROR` lines. Support stops being guesswork.
* **No telemetry in MVP.** Nothing leaves the machine. If crash reporting is ever added it is strictly opt-in, off by default, and disclosed. `[POST-MVP]`

---

## 12. Frontend

### 12.1 Reuse

Seed `app/ui/` from `MooDeX Web/`: keep `style.css`, the `#BB86FC` palette, the card and hover language, and the thin scrollbars. Port the markup to components and delete the demo's mock data. This is the cheapest high-quality asset in the whole project.

### 12.2 State rule

The frontend holds **no authoritative state**. It keeps a render cache keyed by `GameId`, hydrated by commands and patched by events. It never computes playtime, never decides what is running, and never writes to storage. Every number it shows came from the actor.

### 12.3 Rendering requirements

* **Virtualized grid** (`@tanstack/virtual` or an equivalent): render only the visible rows plus a small overscan. Fixes §2 row 9 — v1 realized all 50 cards.
* **Keyed incremental updates.** On `GamesChanged { ids }`, patch only those cards. No list rebuild, ever. Fixes §2 row 10's `Reset`-driven flicker.
* **`IntersectionObserver`** drives `prefetch` so only on-screen covers are fetched, in scroll order.
* **Fixed card dimensions** with `aspect-ratio` and a skeleton placeholder, so nothing reflows as images arrive.
* **`content-visibility: auto`** on off-screen sections.
* Transitions on `transform` and `opacity` only — never on layout properties.
* **Live session timer** ticks client-side from `SessionStarted`, reconciled by `PlaytimeTick`. The sidebar "Playing now" becomes truthful rather than 10 s stale.

### 12.4 Accessibility and input

Full keyboard navigation of the grid, visible focus rings, `prefers-reduced-motion` honoured, every icon button labelled, and a dialog focus trap. v1's `MessageBox` error reporting is replaced by inline, dismissible, non-blocking messages.

---

## 13. Professional EXE procedure

**Mandatory.** v1 shipped an unsigned 6.5 MB installer with no `FileVersion`, `CompanyName: Moody69`, and `ProductName: MooDex` — so every single download triggered "Windows protected your PC". This section replaces that entirely. No build leaves your machine outside this pipeline.

### 13.1 Repository hygiene gate

`.gitignore` covers `target/`, `dist/`, `node_modules/`, `*.pdb`, `*.msi`, `*.exe`, `.vs/`, `.vercel/`, `.env*`. CI fails if any tracked path matches a build-artifact pattern or if any tracked blob exceeds 2 MB. `Cargo.lock` **is** committed; `--locked` is used in CI.

### 13.2 Release profile

```toml
[profile.release]
opt-level     = 3
lto           = "fat"
codegen-units = 1
panic         = "abort"     # panic hook still runs first (§11)
strip         = false       # keep symbols for the separate PDB
debug         = 1           # line tables, for readable crash reports

[profile.release.package."*"]
opt-level = 3
```

The PDB is archived as a release asset, never shipped in the installer. Crash reports stay symbolizable without handing users your symbols.

### 13.3 Executable metadata and manifest

A real version resource — v1 had none:

```json
{
  "productName": "MooDeX",
  "version": "2.0.0",
  "identifier": "io.moodex.app",
  "bundle": {
    "publisher": "<Your legal name or registered company>",
    "copyright": "Copyright (C) 2026 <publisher>",
    "shortDescription": "Game library manager",
    "category": "Utility"
  }
}
```

`app/src-tauri/moodex.manifest`, embedded via `build.rs`:

* `requestedExecutionLevel level="asInvoker"` — no UAC prompt. MooDeX never needs admin, and asking for it is both a red flag and the reason `Win32_ProcessStartTrace` was rejected in §8.4.
* `dpiAwareness: PerMonitorV2` plus `dpiAware: true/pm` — correct on mixed-DPI multi-monitor setups.
* `longPathAware: true` — game install paths get deep.
* `activeCodePage: UTF-8`.

### 13.4 Supply chain

* `cargo deny check licenses advisories bans sources` in CI. Copyleft and unknown licenses fail the build.
* `cargo audit` on every push, and on a weekly schedule to catch advisories filed after merge.
* **SBOM** via `cargo cyclonedx`, attached to every release.
* `rust-toolchain.toml` pins the exact toolchain so builds are reproducible.

### 13.5 Authenticode signing

The step v1 skipped. **Every PE ships signed:** the app exe, every sidecar, and both installers.

* **Certificate:** Azure Trusted Signing / Azure Artifact Signing is Microsoft's recommended production path for direct distribution, at roughly $10/month, versus $300–500/year for a traditional OV certificate. Eligibility is limited to organizations in the US, Canada, EU or UK and to individuals in the US or Canada — **check whether you qualify from Sri Lanka before budgeting on it.** If not, buy an OV certificate from a commercial CA; the rest of this section is unchanged either way.
* **Wiring:** `bundle.windows.signCommand` in `tauri.conf.json` runs for every binary and for the installer. In CI it invokes `dotnet sign` (or `signtool`) against the Trusted Signing endpoint. Credentials live in GitHub Actions secrets and never in the repo.
* **Timestamping is mandatory.** Always pass an RFC-3161 timestamp server. Without it, every signature you ever shipped becomes invalid the day the certificate expires.
* **Verify in CI:** `signtool verify /pa /v` on each artifact. A build that produces an unsigned PE fails.

### 13.6 Bundling

* **NSIS `.exe`** — the primary download, and the updater artifact.
* **WiX `.msi`** — for managed and enterprise deployment.
* **MSIX + Microsoft Store** `[POST-MVP]` — the Store path is worth taking: signing is free because Microsoft re-signs after certification, and Store distribution sidesteps the SmartScreen reputation problem entirely.
* **Install mode: `perUser`.** No admin prompt, and it is the right default for a personal game manager.
* **WebView2:** set `webviewInstallMode` to `downloadBootstrapper` to keep the installer small. Document the dependency honestly in the README — do **not** repeat v1's false "runtime bundled" claim (§2 row 18).
* **Clean uninstall:** remove the program, keep `%LOCALAPPDATA%\MooDeX` unless the user ticks "also delete my library", and always offer an export first.

### 13.7 Update signing — separate from Authenticode

Two independent signatures, answering two different questions. Authenticode says *Windows trusts this publisher*. Minisign says *this update really came from us*. You need both.

```jsonc
{
  "bundle": { "createUpdaterArtifacts": true },
  "plugins": {
    "updater": {
      "pubkey": "<contents of the public key, not a path>",
      "endpoints": ["https://releases.moodex.app/{{target}}/{{arch}}/{{current_version}}"]
    }
  }
}
```

* Generate the ed25519 keypair once: `cargo tauri signer generate -w ~/.tauri/moodex.key`.
* `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are CI secrets. Note that `.env` files are not read for these. **Back the private key up offline — losing it breaks the update path for every installed copy.**
* The `latest.json` manifest requires `version`, `platforms["windows-x86_64"].url`, and `signature` set to the *contents* of the `.sig` file. Tauri validates the whole manifest before comparing versions, so every platform entry must be complete.
* The most common failure is a `pubkey` in the config that does not match the private key used in CI. Verify the full update path on a clean VM before every release.
* Keep v1's good update UX — progress reporting and temp-file-then-rename — which the plugin already implements.

### 13.8 SmartScreen reputation

Signing does **not** clear SmartScreen immediately. Reputation accrues over weeks and hundreds of clean installs, and EV certificates no longer bypass it instantly.

* Use **one** publisher identity and **one** certificate for every release. Re-keying restarts reputation from zero.
* **Never** distribute an unsigned side-build, not even "just for a friend to test". That is precisely what kept v1 permanently untrusted.
* Ship on a predictable cadence; consecutive signed releases under a stable identity are what actually builds trust.
* Set expectations in the README for the early warning window instead of letting users think the app is malware.

### 13.9 Release checklist

1. `cargo fmt --check`, `clippy -D warnings`, `cargo test`, `cargo deny`, `cargo audit` — all green.
2. Version bumped in **one** place (`tauri.conf.json`), propagated by `build.rs`. v1 had the csproj at 1.3 and the installer at 1.0.
3. `CHANGELOG.md` updated; those notes become the updater's `notes`.
4. Signed, annotated git tag `v2.0.0`.
5. CI builds, signs, bundles, and uploads NSIS + MSI + `.sig` + SBOM + PDB.
6. Publish `latest.json`.
7. `signtool verify /pa /v` on every artifact; publish SHA-256 checksums.
8. Install from the published URL on a **clean Windows VM**: verify first run, tray, autostart, launching a game, playtime recording, and an update from the previous version.
9. Only then announce.

---

## 14. Testing

| Level | Scope | Tooling |
|---|---|---|
| Unit | All of `moodex-core`: `sort_title`, `format_playtime`, `dashboard_stats`, `rank_matches`, `recover_orphan_session`, duplicate detection | `cargo test` |
| Property | Session arithmetic: playtime is never negative, never exceeds wall-clock, is invariant under session reordering | `proptest` |
| Store | Migrations forward from empty and from v1-imported data; WAL crash recovery; the one-open-session constraint | `rusqlite` on a temp file |
| Integration | Actor command/event round-trips against an in-memory store; `GamesChanged` carries exactly the affected ids | `tokio::test` |
| Contract | RAWG client against recorded fixtures, including 429, 500, malformed JSON, and empty results | `wiremock` |
| Windows | `launch_tracked` against a synthetic child-spawning test binary; tree exit detection; `match_process` rejects every v1 false positive (`GameBarPresenceWriter`, `GameOverlayUI`) | `cargo test --test winproc`, Windows CI runner |
| Manual | The §13.9 clean-VM checklist | documented script |

**Minimum bar for MVP:** `moodex-core` at 80%+ line coverage (`cargo llvm-cov`), and a regression test for every §2 defect that can be expressed in code. v1 had zero tests; the §8.1 substring bug would have been caught by a single three-line assertion.

---

## 15. Migration from v1

A v1 user's data must survive. This is a one-time import, run on first launch.

```rust
/// Reads %LOCALAPPDATA%\MooDeX\games_data.json (v1 format) and returns a
/// plan for review. Never writes. The user confirms before anything is applied.
pub fn plan_import(v1_path: &Utf8Path) -> Result<ImportPlan, ImportError>;

pub struct ImportPlan {
    pub games: Vec<GameDraft>,
    /// v1's games/local_games split produced mirror entries kept in sync by
    /// Math.Max. Those collapse into one record here — surfaced for explicit
    /// confirmation, never merged silently. (§1.3)
    pub merges: Vec<ProposedMerge>,
    pub warnings: Vec<ImportWarning>,
}
```

1. Read `games`, `local_games` and `user_settings.json`. Ignore the dead `now_playing` field (§2 row 20).
2. Collapse each mirror pair into one record: `Library` entry supplies metadata, `PCGames` entry supplies the install. **This is the moment v1's structural split finally disappears.**
3. Convert each `playtime` float into one synthetic `play_sessions` row (`source = 'manual'`) ending at `LastPlayed`, so no user loses their hours. Flag it as imported so it is visibly an estimate rather than measured data.
4. Present the plan: *N games, M proposed merges, K warnings.* The user confirms. Nothing is destroyed.
5. Copy the v1 JSON to `games_data.v1.backup.json` before writing anything.
6. Thereafter: SQLite WAL, plus a daily rolling backup retaining seven, plus explicit Export/Import JSON in Settings. v1 users had one non-atomically-written file and no backup at all.

---

## 16. MVP scope and milestones

### M0 — Skeleton (foundation)
Workspace, pinned toolchain, lint policy, CI with all five gates, `moodex-core` types and pure functions **with tests**, `moodex-store` with migrations and crash-safe writes. Tauri window boots and shows a hardcoded list.
**Exit:** `cargo test` green; CI blocks a deliberately introduced `unwrap()`.

### M1 — Library (the heart)
`LibraryActor` with the full command and event surface. Store wired. Frontend renders a virtualized grid from real data. Add, edit, delete, status, rating, notes, G.O.A.T. Startup recovery. v1 import.
**Exit:** 500 games render and scroll at 60 fps; no component but the actor touches the DB; killing the process mid-write loses nothing.

### M2 — Play tracking (the big win)
`moodex-winproc`: job-object launch, WMI subscription, three-layer detection, sessions, heartbeats, the launcher-stub promotion. Live timer in the UI.
**Exit:** a Steam game records correct playtime — the case v1 got wrong for the majority of modern games. `GameOverlayUI` running does not accrue a single minute.

### M3 — Discover and covers
Proxy deployed; RAWG client with cache and rate limiting; search with cancel-on-keystroke; Trending / Upcoming / Popular; image pipeline with both variants and the custom protocol; exe-to-RAWG cover matching.
**Exit:** Discover opens from cache with no network; 100 covers load without the UI stalling; no API key present anywhere in the binary (verify with `strings`).

### M4 — Shell polish
Tray, minimize-to-tray, autostart, start-minimized, single instance, window-state persistence, Settings including the key field and Diagnostics pane, Dashboard with real statistics, export/import, backups.
**Exit:** full feature parity with v1 §Features, plus session history.

### M5 — Professional release
Manifest and version resource, release profile, `cargo deny` and SBOM, Authenticode in CI with timestamping, NSIS + MSI, updater keypair and `latest.json`, clean-VM verification.
**Exit:** every item in §13.9 passes. A fresh download from the published URL installs without a SmartScreen block *attributable to missing signature*, and updates itself to the next version.

### Explicitly `[POST-MVP]`
Steam/Epic/Xbox library importers · MSIX and Store listing · achievements · cloud sync · HLTB integration · per-game session history charts · controller navigation · theming · opt-in crash reporting.

---

## 17. Definition of done

A build may be released only when all of the following hold:

1. Every §1 non-negotiable is satisfied, and each has a test or a CI gate proving it.
2. Every §2 defect has either a regression test or a structural argument for why it can no longer be expressed.
3. `cargo clippy --all-targets -- -D warnings` is clean. No `#[allow]` without a comment justifying it.
4. No `unwrap`, `expect` or `panic!` outside `#[cfg(test)]`.
5. `moodex-core` coverage ≥ 80%.
6. `unsafe` appears only in `moodex-winproc`, every block carrying a `// SAFETY:` comment.
7. Every shipped PE is Authenticode-signed **and** timestamped; `signtool verify /pa /v` passes on all of them.
8. The updater round-trips on a clean VM from the previous published version.
9. A v1 user's `games_data.json` imports with no data loss and no silent merge.
10. The repository contains no build output, no installer, and no secret — verified by the §13.1 gate.
11. The README describes the WebView2 requirement accurately and makes no claim contradicted by the artifacts.

---

## Appendix A — Reference configuration

### `rust-toolchain.toml`
```toml
[toolchain]
channel    = "1.83.0"
components = ["rustfmt", "clippy", "llvm-tools-preview"]
targets    = ["x86_64-pc-windows-msvc"]
```

### `ci.yml` gates
```
fmt → clippy -D warnings → test → llvm-cov → deny → audit → repo-hygiene → build
```

### `release.yml` stages
```
verify (all ci gates)
  → build x86_64-pc-windows-msvc --locked
  → sign app exe  (Authenticode + RFC-3161 timestamp)
  → bundle NSIS + MSI
  → sign installers
  → generate updater .sig  (minisign, TAURI_SIGNING_PRIVATE_KEY)
  → cyclonedx SBOM
  → signtool verify /pa /v  (fail on any unsigned PE)
  → publish GitHub Release + latest.json + checksums + PDB
```

### Settings keys
| Key | Default | Notes |
|---|---|---|
| `minimize_to_tray` | `false` | v1 parity |
| `launch_at_startup` | `false` | via `tauri-plugin-autostart`, not hand-rolled registry writes |
| `start_minimized` | `false` | requires `launch_at_startup` |
| `min_session_seconds` | `60` | discard accidental launches (§8.2) |
| `stub_threshold_seconds` | `20` | launcher-stub promotion (§8.5) |
| `image_cache_budget_mb` | `500` | LRU eviction target |
| `rawg_key_source` | `proxy` | `proxy` \| `user` \| `none` |
| `backup_retention_days` | `7` | daily rolling backup |

---

## Appendix B — Sources consulted

* [Known trimming incompatibilities — Microsoft Learn](https://learn.microsoft.com/en-us/dotnet/core/deploying/trimming/incompatibilities) — WPF trimming disabled in the SDK
* [Native AOT deployment overview — Microsoft Learn](https://learn.microsoft.com/en-us/dotnet/core/deploying/native-aot/) — Native AOT requires trimming
* [Code signing options for Windows app developers — Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)
* [Azure Artifact Signing](https://azure.microsoft.com/en-us/products/artifact-signing)
* [Publish your first Windows app — Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/publish-first-app)
* [Automatically Signing a Windows EXE with Azure Trusted Signing — Scott Hanselman](https://www.hanselman.com/blog/automatically-signing-a-windows-exe-with-azure-trusted-signing-dotnet-sign-and-github-actions)
* [Tauri 2 updater plugin](https://v2.tauri.app/plugin/updater/) — `pubkey`/`endpoints` placement, `latest.json` format
* [Ship Your Tauri v2 App Like a Pro: Code Signing](https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-code-signing-for-macos-and-windows-part-12-3o9n)
* [process-wrap — JobObject wrapper](https://docs.rs/process-wrap/latest/process_wrap/) · [active-rpc-process-tree](https://docs.rs/active-rpc-process-tree/latest/active_rpc_process_tree/) — job membership inheritance, breakaway
* [wmi crate](https://docs.rs/wmi/0.18.1/aarch64-unknown-linux-gnu/wmi) · [Receiving asynchronous event notifications — Microsoft Learn](https://learn.microsoft.com/en-ie/Windows/Win32/wmisdk/receiving-asynchronous-event-notifications) — `__InstanceCreationEvent`; `Win32_ProcessStartTrace` needs admin
* [Rust GUI frameworks in 2026 comparison](https://www.pistack.xyz/posts/2026-08-25-rust-gui-frameworks-egui-iced-slint-comparison/) · [The state of Rust GUI libraries — LogRocket](https://blog.logrocket.com/state-rust-gui-libraries/)
