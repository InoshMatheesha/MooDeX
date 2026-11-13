// actor.rs - LibraryActor: SOLE owner of library state; all mutation is message passing (spec 7).

use crate::import::{self, ExportDoc, ImportPlan, ImportReport};
use crate::settings::Settings;
use crate::view::{ts, DashboardView, GameView, ViewCtx};
use crate::{now_ms, LibraryError};
use camino::Utf8PathBuf;
use moodex_core::matching::find_duplicate_candidates;
use moodex_core::paths::DetectRegistry;
use moodex_core::query::filter_and_sort;
use moodex_core::session::{heartbeat_action, is_noise, recover_orphan_session, span_minutes, HeartbeatAction};
use moodex_core::stats::dashboard_stats;
use moodex_core::{
    ContentHash, CoverRef, DuplicateCandidate, Game, GameDraft, GameFilter, GameId, GamePatch, Install, LaunchTarget,
    OpenSession, PlayStats, SessionRecord, SessionSource, SortOrder,
};
use moodex_images::ImageCache;
use moodex_rawg::CacheEntry;
use moodex_store::{roll_daily_backup, CachedResponse, Store};
use moodex_winproc::SharedRegistry;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::{broadcast, mpsc, oneshot};

const CHANNEL_CAP: usize = 256;
const SEND_TIMEOUT: Duration = Duration::from_secs(5);
const HEARTBEAT: Duration = Duration::from_secs(30);

// Collaborators handed to the actor once, at spawn.
pub struct Deps {
    pub images: ImageCache,
    pub registry: SharedRegistry,
    pub backup_dir: Option<Utf8PathBuf>,
}

// Pushed to the frontend and the tracker; granular, never "reload everything" (spec 7.1).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LibraryEvent {
    GamesChanged { ids: Vec<GameId> },
    SessionStarted { game_id: GameId, at: String },
    SessionEnded { game_id: GameId, minutes: f64 },
    PlaytimeTick { game_id: GameId, minutes: f64 },
    CoverReady { game_id: GameId, hash: String },
    ScanProgress { done: usize, total: usize },
    SettingsChanged { settings: Settings },
    RegistryChanged,
}

// Play tracking facts from WinProc / the tracker.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayEvent {
    Started { game_id: GameId, source: SessionSource, at: OffsetDateTime },
    Heartbeat { game_id: GameId, at: OffsetDateTime },
    Ended { game_id: GameId, at: OffsetDateTime },
    Abandoned { game_id: GameId },
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticsInfo {
    pub games: usize,
    pub installed: usize,
    pub open_sessions: usize,
    pub schema_version: i64,
    pub db_path: Option<String>,
    pub detect_registry: usize,
}

type Reply<T> = oneshot::Sender<T>;

enum Cmd {
    ListIds {
        filter: GameFilter,
        sort: SortOrder,
        reply: Reply<Vec<GameId>>,
    },
    ListGames {
        filter: GameFilter,
        sort: SortOrder,
        reply: Reply<Vec<GameView>>,
    },
    GetGames {
        ids: Vec<GameId>,
        reply: Reply<Vec<GameView>>,
    },
    GetGame {
        id: GameId,
        reply: Reply<Option<GameView>>,
    },
    GetRaw {
        id: GameId,
        reply: Reply<Option<Game>>,
    },
    AddGame {
        draft: GameDraft,
        reply: Reply<Result<GameId, LibraryError>>,
    },
    UpdateGame {
        id: GameId,
        patch: GamePatch,
        reply: Reply<Result<(), LibraryError>>,
    },
    DeleteGame {
        id: GameId,
        reply: Reply<Result<(), LibraryError>>,
    },
    SetInstall {
        id: GameId,
        install: Option<Install>,
        missing: bool,
        reply: Reply<Result<(), LibraryError>>,
    },
    MarkMissing {
        id: GameId,
        missing: bool,
    },
    SetGoat {
        id: Option<GameId>,
        reply: Reply<Result<(), LibraryError>>,
    },
    MergeGames {
        keep: GameId,
        absorb: GameId,
        reply: Reply<Result<(), LibraryError>>,
    },
    Dashboard {
        reply: Reply<DashboardView>,
    },
    Duplicates {
        reply: Reply<Vec<DuplicateCandidate>>,
    },
    Sessions {
        id: GameId,
        reply: Reply<Result<Vec<SessionRecord>, LibraryError>>,
    },
    Running {
        reply: Reply<Vec<GameId>>,
    },
    RawgIds {
        reply: Reply<Vec<(i64, GameId)>>,
    },
    Play(PlayEvent),
    GetSettings {
        reply: Reply<Settings>,
    },
    SetSetting {
        key: String,
        value: serde_json::Value,
        reply: Reply<Result<Settings, LibraryError>>,
    },
    CacheGet {
        key: String,
        reply: Reply<Option<CacheEntry>>,
    },
    CachePut {
        key: String,
        entry: CacheEntry,
    },
    ImportV1 {
        plan: Box<ImportPlan>,
        accepted: Vec<(String, String)>,
        reply: Reply<Result<ImportReport, LibraryError>>,
    },
    Export {
        reply: Reply<Result<ExportDoc, LibraryError>>,
    },
    ImportDoc {
        doc: Box<ExportDoc>,
        reply: Reply<Result<ImportReport, LibraryError>>,
    },
    Diagnostics {
        reply: Reply<DiagnosticsInfo>,
    },
    Shutdown {
        reply: Reply<()>,
    },
}

#[derive(Clone)]
pub struct LibraryHandle {
    tx: mpsc::Sender<Cmd>,
    events: broadcast::Sender<LibraryEvent>,
}

impl LibraryHandle {
    // Run recovery (spec 7.4), then spawn the actor task.
    pub async fn spawn(store: Store, deps: Deps) -> Result<Self, LibraryError> {
        let (events, _) = broadcast::channel(1024);
        let mut actor = Actor::start(store, deps, events.clone())?;
        let (tx, rx) = mpsc::channel(CHANNEL_CAP);
        let all: Vec<GameId> = actor.games.keys().copied().collect();
        tokio::spawn(async move { actor.run(rx).await });
        let h = Self { tx, events };
        h.emit(LibraryEvent::GamesChanged { ids: all });
        Ok(h)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LibraryEvent> {
        self.events.subscribe()
    }

    fn emit(&self, e: LibraryEvent) {
        if self.events.send(e).is_err() {
            tracing::trace!("no event subscribers yet");
        }
    }

    // Request/response over a oneshot; a wedged actor yields Busy, never unbounded memory (spec 7.3).
    async fn call<R>(&self, make: impl FnOnce(Reply<R>) -> Cmd) -> Result<R, LibraryError> {
        let (tx, rx) = oneshot::channel();
        match tokio::time::timeout(SEND_TIMEOUT, self.tx.send(make(tx))).await {
            Err(_) => return Err(LibraryError::Busy),
            Ok(Err(_)) => return Err(LibraryError::Closed),
            Ok(Ok(())) => {}
        }
        rx.await.map_err(|_| LibraryError::Closed)
    }

    async fn post(&self, cmd: Cmd) -> Result<(), LibraryError> {
        match tokio::time::timeout(SEND_TIMEOUT, self.tx.send(cmd)).await {
            Err(_) => Err(LibraryError::Busy),
            Ok(Err(_)) => Err(LibraryError::Closed),
            Ok(Ok(())) => Ok(()),
        }
    }

    pub async fn list_ids(&self, filter: GameFilter, sort: SortOrder) -> Result<Vec<GameId>, LibraryError> {
        self.call(|reply| Cmd::ListIds { filter, sort, reply }).await
    }

    pub async fn list_games(&self, filter: GameFilter, sort: SortOrder) -> Result<Vec<GameView>, LibraryError> {
        self.call(|reply| Cmd::ListGames { filter, sort, reply }).await
    }

    pub async fn get_games(&self, ids: Vec<GameId>) -> Result<Vec<GameView>, LibraryError> {
        self.call(|reply| Cmd::GetGames { ids, reply }).await
    }

    pub async fn get_game(&self, id: GameId) -> Result<Option<GameView>, LibraryError> {
        self.call(|reply| Cmd::GetGame { id, reply }).await
    }

    pub async fn get_raw(&self, id: GameId) -> Result<Option<Game>, LibraryError> {
        self.call(|reply| Cmd::GetRaw { id, reply }).await
    }

    pub async fn add_game(&self, mut draft: GameDraft) -> Result<GameId, LibraryError> {
        let mut missing = false;
        if let Some(i) = draft.install.take() {
            let (i, m) = canonical_install(i).await?;
            draft.install = Some(i);
            missing = m;
        }
        let id = self.call(|reply| Cmd::AddGame { draft, reply }).await??;
        if missing {
            self.post(Cmd::MarkMissing { id, missing }).await?;
        }
        Ok(id)
    }

    pub async fn update_game(&self, id: GameId, patch: GamePatch) -> Result<(), LibraryError> {
        self.call(|reply| Cmd::UpdateGame { id, patch, reply }).await?
    }

    pub async fn delete_game(&self, id: GameId) -> Result<(), LibraryError> {
        self.call(|reply| Cmd::DeleteGame { id, reply }).await?
    }

    // Canonicalise paths off the actor, then hand over (spec 8.4).
    pub async fn set_install(&self, id: GameId, install: Option<Install>) -> Result<(), LibraryError> {
        let (install, missing) = match install {
            None => (None, false),
            Some(i) => {
                let (i, m) = canonical_install(i).await?;
                (Some(i), m)
            }
        };
        self.call(|reply| Cmd::SetInstall { id, install, missing, reply }).await?
    }

    pub async fn mark_missing(&self, id: GameId, missing: bool) -> Result<(), LibraryError> {
        self.post(Cmd::MarkMissing { id, missing }).await
    }

    pub async fn set_goat(&self, id: Option<GameId>) -> Result<(), LibraryError> {
        self.call(|reply| Cmd::SetGoat { id, reply }).await?
    }

    pub async fn merge_games(&self, keep: GameId, absorb: GameId) -> Result<(), LibraryError> {
        self.call(|reply| Cmd::MergeGames { keep, absorb, reply }).await?
    }

    pub async fn dashboard(&self) -> Result<DashboardView, LibraryError> {
        self.call(|reply| Cmd::Dashboard { reply }).await
    }

    pub async fn duplicates(&self) -> Result<Vec<DuplicateCandidate>, LibraryError> {
        self.call(|reply| Cmd::Duplicates { reply }).await
    }

    pub async fn sessions(&self, id: GameId) -> Result<Vec<SessionRecord>, LibraryError> {
        self.call(|reply| Cmd::Sessions { id, reply }).await?
    }

    pub async fn running(&self) -> Result<Vec<GameId>, LibraryError> {
        self.call(|reply| Cmd::Running { reply }).await
    }

    // RAWG ids already in the library, for Discover badges.
    pub async fn rawg_ids(&self) -> Result<Vec<(i64, GameId)>, LibraryError> {
        self.call(|reply| Cmd::RawgIds { reply }).await
    }

    // Play facts are never dropped; they wait for capacity.
    pub async fn play_event(&self, ev: PlayEvent) -> Result<(), LibraryError> {
        self.tx.send(Cmd::Play(ev)).await.map_err(|_| LibraryError::Closed)
    }

    pub async fn settings(&self) -> Result<Settings, LibraryError> {
        self.call(|reply| Cmd::GetSettings { reply }).await
    }

    pub async fn set_setting(&self, key: String, value: serde_json::Value) -> Result<Settings, LibraryError> {
        self.call(|reply| Cmd::SetSetting { key, value, reply }).await?
    }

    pub async fn cache_get(&self, key: String) -> Option<CacheEntry> {
        match self.call(|reply| Cmd::CacheGet { key, reply }).await {
            Ok(v) => v,
            Err(e) => {
                tracing::debug!(error = %e, "cache read skipped");
                None
            }
        }
    }

    // Advisory: dropped rather than queued when the actor is saturated.
    pub fn cache_put(&self, key: String, entry: CacheEntry) {
        if self.tx.try_send(Cmd::CachePut { key, entry }).is_err() {
            tracing::debug!("cache write dropped: actor busy");
        }
    }

    pub async fn import_v1(
        &self,
        plan: ImportPlan,
        accepted: Vec<(String, String)>,
    ) -> Result<ImportReport, LibraryError> {
        // Back up the v1 file before anything is written (spec 15 step 5).
        import::backup_v1_file(&plan.source_path)?;
        self.call(|reply| Cmd::ImportV1 { plan: Box::new(plan), accepted, reply }).await?
    }

    pub async fn export_doc(&self) -> Result<ExportDoc, LibraryError> {
        self.call(|reply| Cmd::Export { reply }).await?
    }

    pub async fn import_doc(&self, doc: ExportDoc) -> Result<ImportReport, LibraryError> {
        self.call(|reply| Cmd::ImportDoc { doc: Box::new(doc), reply }).await?
    }

    pub async fn diagnostics(&self) -> Result<DiagnosticsInfo, LibraryError> {
        self.call(|reply| Cmd::Diagnostics { reply }).await
    }

    pub async fn shutdown(&self) -> Result<(), LibraryError> {
        self.call(|reply| Cmd::Shutdown { reply }).await
    }
}

// Canonical detect path is required; a missing launch exe is allowed but flagged.
async fn canonical_install(mut i: Install) -> Result<(Install, bool), LibraryError> {
    i.launch.validate()?;
    tokio::task::spawn_blocking(move || {
        let mut missing = false;
        if let LaunchTarget::Exe { path, .. } | LaunchTarget::Shortcut { path } = &mut i.launch {
            match moodex_winproc::canonicalize(path) {
                Ok(c) => *path = c,
                Err(_) => missing = true,
            }
        }
        if let Some(d) = i.detect_exe.take() {
            let c = moodex_winproc::canonicalize(&d)
                .map_err(|_| LibraryError::Invalid(format!("detection executable not found: {d}")))?;
            // Equal to the launched exe means "use the default"; keep it implicit.
            let same = matches!(&i.launch, LaunchTarget::Exe { path, .. } if path == &c);
            i.detect_exe = if same { None } else { Some(c) };
        }
        Ok((i, missing))
    })
    .await
    .map_err(|e| LibraryError::Io(e.to_string()))?
}

struct Actor {
    store: Store,
    games: HashMap<GameId, Game>,
    stats: HashMap<GameId, PlayStats>,
    open: HashMap<GameId, OpenSession>,
    missing: HashSet<GameId>,
    estimates: HashSet<GameId>,
    settings: Settings,
    events: broadcast::Sender<LibraryEvent>,
    images: ImageCache,
    registry: SharedRegistry,
    backup_dir: Option<Utf8PathBuf>,
}

impl Actor {
    // Startup recovery runs before the first command (spec 7.4).
    fn start(mut store: Store, deps: Deps, events: broadcast::Sender<LibraryEvent>) -> Result<Self, LibraryError> {
        // 1. Migrations already applied by Store::open_rw in one transaction.
        // 2. Close orphaned sessions at their last heartbeat.
        let now = now_ms();
        let settings = Settings::from_stored(&store.settings()?);
        for o in store.open_sessions()? {
            let close = recover_orphan_session(&o, now);
            if is_noise(close.minutes, settings.min_session_seconds) {
                store.delete_session(o.id)?;
                tracing::warn!(game_id = %o.game_id, "discarded orphaned sub-minimum session");
            } else {
                store.close_session(o.id, close.ended_at)?;
                tracing::warn!(game_id = %o.game_id, minutes = close.minutes, "recovered orphaned session at last heartbeat");
            }
        }
        let games: HashMap<GameId, Game> = store.load_games()?.into_iter().map(|g| (g.id, g)).collect();
        let stats = store.play_stats_all()?;
        let estimates = store.games_with_estimates()?;
        // 3. Flag installs whose files vanished; nothing is deleted (spec 7.4).
        let missing: HashSet<GameId> = games.values().filter(|g| install_missing(g)).map(|g| g.id).collect();
        let mut actor = Actor {
            store,
            games,
            stats,
            open: HashMap::new(),
            missing,
            estimates,
            settings,
            events,
            images: deps.images,
            registry: deps.registry,
            backup_dir: deps.backup_dir,
        };
        // 4. Daily rolling backup; image cache GC runs at startup and daily (spec 10).
        actor.roll_backup();
        let images = actor.images.clone();
        tokio::spawn(async move {
            match images.gc().await {
                Ok(r) => tracing::info!(removed = r.removed, bytes = r.bytes_after, "startup image cache gc"),
                Err(e) => tracing::warn!(error = %e, "startup image cache gc failed"),
            }
        });
        actor.rebuild_registry();
        if !actor.missing.is_empty() {
            tracing::warn!(count = actor.missing.len(), "installs point at missing files");
        }
        Ok(actor)
    }

    async fn run(&mut self, mut rx: mpsc::Receiver<Cmd>) {
        let mut covers = self.images.subscribe_ready();
        let mut daily = tokio::time::interval(Duration::from_secs(24 * 3600));
        daily.tick().await;
        loop {
            tokio::select! {
                cmd = rx.recv() => match cmd {
                    Some(Cmd::Shutdown { reply }) => {
                        self.shutdown();
                        if reply.send(()).is_err() {
                            tracing::debug!("shutdown caller gone");
                        }
                        break;
                    }
                    Some(c) => self.handle(c),
                    None => {
                        self.shutdown();
                        break;
                    }
                },
                ready = covers.recv() => match ready {
                    Ok((url, hash)) => self.cover_cached(&url, &hash),
                    Err(broadcast::error::RecvError::Lagged(n)) => tracing::debug!(n, "cover events lagged"),
                    Err(broadcast::error::RecvError::Closed) => covers = self.images.subscribe_ready(),
                },
                _ = daily.tick() => self.daily_maintenance(),
            }
        }
        tracing::info!("library actor stopped");
    }

    fn emit(&self, e: LibraryEvent) {
        if self.events.send(e).is_err() {
            tracing::trace!("event dropped: no subscribers");
        }
    }

    fn changed(&self, ids: Vec<GameId>) {
        if !ids.is_empty() {
            self.emit(LibraryEvent::GamesChanged { ids });
        }
    }

    fn reply<T>(tx: Reply<T>, v: T) {
        if tx.send(v).is_err() {
            tracing::debug!("caller went away before reply");
        }
    }

    fn handle(&mut self, cmd: Cmd) {
        let span = tracing::debug_span!("library_cmd");
        let _e = span.enter();
        match cmd {
            Cmd::ListIds { filter, sort, reply } => Self::reply(reply, self.list_ids(&filter, sort)),
            Cmd::ListGames { filter, sort, reply } => {
                let ids = self.list_ids(&filter, sort);
                Self::reply(reply, self.views(&ids));
            }
            Cmd::GetGames { ids, reply } => Self::reply(reply, self.views(&ids)),
            Cmd::GetGame { id, reply } => Self::reply(reply, self.view(id)),
            Cmd::GetRaw { id, reply } => Self::reply(reply, self.games.get(&id).cloned()),
            Cmd::AddGame { draft, reply } => {
                let r = self.add_game(draft);
                Self::reply(reply, r)
            }
            Cmd::UpdateGame { id, patch, reply } => {
                let r = self.update_game(id, &patch);
                Self::reply(reply, r)
            }
            Cmd::DeleteGame { id, reply } => {
                let r = self.delete_game(id);
                Self::reply(reply, r)
            }
            Cmd::SetInstall { id, install, missing, reply } => {
                let r = self.set_install(id, install, missing);
                Self::reply(reply, r)
            }
            Cmd::MarkMissing { id, missing } => {
                let was = self.missing.contains(&id);
                if missing {
                    self.missing.insert(id);
                } else {
                    self.missing.remove(&id);
                }
                if was != missing {
                    self.changed(vec![id]);
                }
            }
            Cmd::SetGoat { id, reply } => {
                let r = self.set_goat(id);
                Self::reply(reply, r)
            }
            Cmd::MergeGames { keep, absorb, reply } => {
                let r = self.merge(keep, absorb);
                Self::reply(reply, r)
            }
            Cmd::Dashboard { reply } => Self::reply(reply, self.dashboard()),
            Cmd::Duplicates { reply } => {
                let all: Vec<Game> = self.games.values().cloned().collect();
                Self::reply(reply, find_duplicate_candidates(&all))
            }
            Cmd::Sessions { id, reply } => Self::reply(reply, self.store.sessions_for(id, 200).map_err(Into::into)),
            Cmd::Running { reply } => Self::reply(reply, self.open.keys().copied().collect()),
            Cmd::RawgIds { reply } => {
                Self::reply(reply, self.games.values().filter_map(|g| g.rawg_id.map(|r| (r, g.id))).collect())
            }
            Cmd::Play(ev) => self.play_event(ev),
            Cmd::GetSettings { reply } => Self::reply(reply, self.settings.clone()),
            Cmd::SetSetting { key, value, reply } => {
                let r = self.set_setting(&key, &value);
                Self::reply(reply, r)
            }
            Cmd::CacheGet { key, reply } => {
                let v = match self.store.cache_get(&key) {
                    Ok(v) => v.map(|c| CacheEntry {
                        body: c.body,
                        etag: c.etag,
                        fetched_at: c.fetched_at,
                        expires_at: c.expires_at,
                    }),
                    Err(e) => {
                        tracing::warn!(error = %e, "cache read failed");
                        None
                    }
                };
                Self::reply(reply, v)
            }
            Cmd::CachePut { key, entry } => {
                let c = CachedResponse {
                    body: entry.body,
                    etag: entry.etag,
                    fetched_at: entry.fetched_at,
                    expires_at: entry.expires_at,
                };
                if let Err(e) = self.store.cache_put(&key, &c) {
                    tracing::warn!(error = %e, "cache write failed");
                }
            }
            Cmd::ImportV1 { plan, accepted, reply } => {
                let r = self.import_v1(&plan, &accepted);
                Self::reply(reply, r)
            }
            Cmd::Export { reply } => {
                let r = self.export();
                Self::reply(reply, r)
            }
            Cmd::ImportDoc { doc, reply } => {
                let r = self.import_doc(*doc);
                Self::reply(reply, r)
            }
            Cmd::Diagnostics { reply } => {
                let info = DiagnosticsInfo {
                    games: self.games.len(),
                    installed: self.games.values().filter(|g| g.install.is_some()).count(),
                    open_sessions: self.open.len(),
                    schema_version: self.store.schema_version().unwrap_or(0),
                    db_path: self.store.path().map(|p| p.to_string()),
                    detect_registry: self.registry.read().map(|r| r.len()).unwrap_or(0),
                };
                Self::reply(reply, info)
            }
            Cmd::Shutdown { reply } => Self::reply(reply, ()),
        }
    }

    // --- Reads -----------------------------------------------------------

    fn rows(&self) -> Vec<(Game, PlayStats)> {
        self.games.values().map(|g| (g.clone(), self.stats.get(&g.id).copied().unwrap_or_default())).collect()
    }

    fn list_ids(&self, f: &GameFilter, sort: SortOrder) -> Vec<GameId> {
        filter_and_sort(&self.rows(), f, sort)
    }

    fn view(&self, id: GameId) -> Option<GameView> {
        let g = self.games.get(&id)?;
        let ctx = ViewCtx { images: &self.images };
        Some(ctx.build(
            g,
            &self.stats.get(&id).copied().unwrap_or_default(),
            self.missing.contains(&id),
            self.open.get(&id).map(|o| o.started_at),
            self.estimates.contains(&id),
        ))
    }

    fn views(&self, ids: &[GameId]) -> Vec<GameView> {
        ids.iter().filter_map(|id| self.view(*id)).collect()
    }

    fn dashboard(&self) -> DashboardView {
        let stats = dashboard_stats(&self.rows());
        DashboardView {
            most_played: self.views(&stats.most_played),
            recently_played: self.views(&stats.recently_played),
            playing_now: self.views(&self.open.keys().copied().collect::<Vec<_>>()),
            goat: stats.goat.and_then(|id| self.view(id)),
            stats,
        }
    }

    // --- Mutations: SQLite first, then memory, then broadcast (spec 7.2) ---

    fn add_game(&mut self, draft: GameDraft) -> Result<GameId, LibraryError> {
        let g = draft.into_game(now_ms())?;
        self.store.insert_game(&g)?;
        let id = g.id;
        let has_install = g.install.is_some();
        self.games.insert(id, g);
        self.stats.insert(id, PlayStats::default());
        if has_install {
            self.rebuild_registry();
        }
        self.changed(vec![id]);
        Ok(id)
    }

    fn update_game(&mut self, id: GameId, patch: &GamePatch) -> Result<(), LibraryError> {
        let mut g = self.games.get(&id).cloned().ok_or(LibraryError::NotFound(id))?;
        patch.apply(&mut g, now_ms())?;
        self.store.update_game(&g)?;
        self.games.insert(id, g);
        self.changed(vec![id]);
        Ok(())
    }

    fn delete_game(&mut self, id: GameId) -> Result<(), LibraryError> {
        self.store.delete_game(id)?;
        let had_install = self.games.remove(&id).is_some_and(|g| g.install.is_some());
        self.stats.remove(&id);
        self.open.remove(&id);
        self.missing.remove(&id);
        self.estimates.remove(&id);
        if had_install {
            self.rebuild_registry();
        }
        self.changed(vec![id]);
        Ok(())
    }

    fn set_install(&mut self, id: GameId, install: Option<Install>, missing: bool) -> Result<(), LibraryError> {
        if !self.games.contains_key(&id) {
            return Err(LibraryError::NotFound(id));
        }
        let now = now_ms();
        self.store.set_install(id, install.as_ref(), now)?;
        if let Some(g) = self.games.get_mut(&id) {
            g.install = install;
            g.updated_at = now;
        }
        if missing {
            self.missing.insert(id);
        } else {
            self.missing.remove(&id);
        }
        self.rebuild_registry();
        self.changed(vec![id]);
        Ok(())
    }

    fn set_goat(&mut self, id: Option<GameId>) -> Result<(), LibraryError> {
        if let Some(i) = id {
            if !self.games.contains_key(&i) {
                return Err(LibraryError::NotFound(i));
            }
        }
        let now = now_ms();
        let changed = self.store.set_goat(id, now)?;
        for g in self.games.values_mut() {
            let want = Some(g.id) == id;
            if g.is_goat != want {
                g.is_goat = want;
                g.updated_at = now;
            }
        }
        self.changed(changed);
        Ok(())
    }

    // User-confirmed only (spec 1.3). Keep's fields win; gaps are filled from absorb.
    fn merge(&mut self, keep: GameId, absorb: GameId) -> Result<(), LibraryError> {
        if keep == absorb {
            return Err(LibraryError::Invalid("cannot merge a game with itself".into()));
        }
        if self.open.contains_key(&keep) || self.open.contains_key(&absorb) {
            return Err(LibraryError::Conflict("stop the game before merging".into()));
        }
        let a = self.games.get(&absorb).cloned().ok_or(LibraryError::NotFound(absorb))?;
        let mut k = self.games.get(&keep).cloned().ok_or(LibraryError::NotFound(keep))?;
        if k.rawg_id.is_none() {
            k.rawg_id = a.rawg_id;
        }
        if k.cover == CoverRef::None {
            k.cover = a.cover.clone();
        }
        if k.release_date.is_none() {
            k.release_date = a.release_date;
        }
        if k.rating.is_none() {
            k.rating = a.rating;
        }
        if k.install.is_none() {
            k.install = a.install.clone();
        }
        if !a.notes.trim().is_empty() && !k.notes.contains(a.notes.trim()) {
            k.notes = if k.notes.trim().is_empty() { a.notes.clone() } else { format!("{}\n\n{}", k.notes, a.notes) };
        }
        k.is_goat = k.is_goat || a.is_goat;
        let mut genres = k.genres.clone();
        genres.extend(a.genres.iter().cloned());
        k.genres = moodex_core::text::clean_genres(genres);
        k.added_at = k.added_at.min(a.added_at);
        k.updated_at = now_ms();
        self.store.merge_games(&k, absorb)?;
        if self.missing.remove(&absorb) && k.install == a.install {
            self.missing.insert(keep);
        }
        if self.estimates.remove(&absorb) {
            self.estimates.insert(keep);
        }
        self.games.remove(&absorb);
        self.stats.remove(&absorb);
        self.games.insert(keep, k);
        self.refresh_stats(keep);
        self.rebuild_registry();
        self.changed(vec![keep, absorb]);
        Ok(())
    }

    fn set_setting(&mut self, key: &str, value: &serde_json::Value) -> Result<Settings, LibraryError> {
        let mut next = self.settings.clone();
        let normalised = next.apply(key, value)?;
        self.store.set_setting(key, &normalised)?;
        // Dependent keys changed too (start_minimized follows launch_at_startup).
        if next.start_minimized != self.settings.start_minimized {
            self.store.set_setting("start_minimized", &serde_json::json!(next.start_minimized))?;
        }
        self.settings = next;
        self.emit(LibraryEvent::SettingsChanged { settings: self.settings.clone() });
        Ok(self.settings.clone())
    }

    fn refresh_stats(&mut self, id: GameId) -> f64 {
        match self.store.play_stats(id) {
            Ok(p) => {
                self.stats.insert(id, p);
                p.minutes
            }
            Err(e) => {
                tracing::error!(%id, error = %e, "could not read play stats");
                self.stats.get(&id).map(|p| p.minutes).unwrap_or(0.0)
            }
        }
    }

    // Registry = canonical detect path -> game; only the actor writes it.
    fn rebuild_registry(&mut self) {
        let entries = self
            .games
            .values()
            .filter_map(|g| g.install.as_ref().and_then(|i| i.effective_detect()).map(|p| (p.to_string(), g.id)));
        let reg = DetectRegistry::from_entries(entries);
        match self.registry.write() {
            Ok(mut w) => *w = reg,
            Err(p) => *p.into_inner() = reg,
        }
        self.emit(LibraryEvent::RegistryChanged);
    }

    // --- Sessions (spec 8.2) ---------------------------------------------

    fn play_event(&mut self, ev: PlayEvent) {
        let span = tracing::info_span!("play_event", game_id = tracing::field::Empty);
        let _e = span.enter();
        match ev {
            PlayEvent::Started { game_id, source, at } => {
                span.record("game_id", tracing::field::display(game_id));
                if self.open.contains_key(&game_id) || !self.games.contains_key(&game_id) {
                    return;
                }
                match self.store.open_session(game_id, at, source) {
                    Ok(id) => {
                        self.open
                            .insert(game_id, OpenSession { id, game_id, started_at: at, last_heartbeat: at, source });
                        tracing::info!(%game_id, ?source, "session started");
                        self.emit(LibraryEvent::SessionStarted { game_id, at: ts(at) });
                        self.changed(vec![game_id]);
                    }
                    Err(e) => tracing::error!(%game_id, error = %e, "could not open session"),
                }
            }
            PlayEvent::Heartbeat { game_id, at } => {
                let Some(o) = self.open.get(&game_id).cloned() else { return };
                let action = heartbeat_action(
                    o.last_heartbeat,
                    at,
                    time::Duration::try_from(HEARTBEAT).unwrap_or(time::Duration::seconds(30)),
                );
                match action {
                    HeartbeatAction::Continue => {
                        if let Err(e) = self.store.heartbeat(o.id, at) {
                            tracing::error!(%game_id, error = %e, "heartbeat write failed");
                            return;
                        }
                        if let Some(s) = self.open.get_mut(&game_id) {
                            s.last_heartbeat = at;
                        }
                    }
                    HeartbeatAction::Split => {
                        // The machine slept: end at the last heartbeat, start fresh now.
                        tracing::info!(%game_id, "heartbeat gap detected (sleep); splitting session");
                        self.finish(game_id, o.last_heartbeat, false);
                        self.play_event(PlayEvent::Started { game_id, source: o.source, at });
                        return;
                    }
                }
                let minutes = self.refresh_stats(game_id);
                self.emit(LibraryEvent::PlaytimeTick { game_id, minutes });
            }
            PlayEvent::Ended { game_id, at } => self.finish(game_id, at, true),
            PlayEvent::Abandoned { game_id } => {
                if let Some(o) = self.open.remove(&game_id) {
                    if let Err(e) = self.store.delete_session(o.id) {
                        tracing::error!(%game_id, error = %e, "could not discard abandoned session");
                    }
                    tracing::info!(%game_id, "launch never produced the game; session discarded");
                    self.refresh_stats(game_id);
                    self.emit(LibraryEvent::SessionEnded { game_id, minutes: 0.0 });
                    self.changed(vec![game_id]);
                }
            }
        }
    }

    fn finish(&mut self, game_id: GameId, at: OffsetDateTime, announce: bool) {
        let Some(o) = self.open.remove(&game_id) else { return };
        let minutes = span_minutes(o.started_at, at);
        let res = if is_noise(minutes, self.settings.min_session_seconds) {
            tracing::info!(%game_id, minutes, "session below minimum; discarded");
            self.store.delete_session(o.id)
        } else {
            self.store.close_session(o.id, at)
        };
        if let Err(e) = res {
            tracing::error!(%game_id, error = %e, "could not close session");
        }
        self.refresh_stats(game_id);
        if announce {
            tracing::info!(%game_id, minutes, "session ended");
            self.emit(LibraryEvent::SessionEnded { game_id, minutes });
        }
        self.changed(vec![game_id]);
    }

    // --- Covers ----------------------------------------------------------

    fn cover_cached(&mut self, url: &str, hash: &ContentHash) {
        let ids: Vec<GameId> = self
            .games
            .values()
            .filter(|g| g.cover.url() == Some(url) && !matches!(g.cover, CoverRef::Cached { .. }))
            .map(|g| g.id)
            .collect();
        for id in &ids {
            if let Err(e) = self.store.set_cover_hash(*id, Some(&hash.0)) {
                tracing::warn!(%id, error = %e, "could not record cover hash");
                continue;
            }
            if let Some(g) = self.games.get_mut(id) {
                g.cover = CoverRef::Cached { hash: hash.clone(), url: url.to_string() };
            }
            self.emit(LibraryEvent::CoverReady { game_id: *id, hash: hash.0.clone() });
        }
        self.changed(ids);
    }

    // --- Import / export -------------------------------------------------

    fn import_v1(&mut self, plan: &ImportPlan, accepted: &[(String, String)]) -> Result<ImportReport, LibraryError> {
        let existing_rawg: HashSet<i64> = self.games.values().filter_map(|g| g.rawg_id).collect();
        let existing_detect: HashSet<String> = self
            .games
            .values()
            .filter_map(|g| {
                g.install.as_ref().and_then(|i| i.effective_detect()).map(|p| moodex_core::paths::path_key(p.as_str()))
            })
            .collect();
        let built = import::build_v1(plan, accepted, &existing_rawg, &existing_detect, now_ms())?;
        self.store.import_batch(&built.games, &built.sessions)?;
        if let Some(goat) = built.goat {
            if !self.games.values().any(|g| g.is_goat) {
                self.store.set_goat(Some(goat), now_ms())?;
            }
        }
        self.store.set_setting("v1_import_done", &serde_json::json!(true))?;
        self.settings.v1_import_done = true;
        self.reload()?;
        self.changed(built.games.iter().map(|g| g.id).collect());
        Ok(built.report)
    }

    fn export(&self) -> Result<ExportDoc, LibraryError> {
        let mut games: Vec<&Game> = self.games.values().collect();
        games.sort_by_key(|g| g.id);
        let mut out = Vec::with_capacity(games.len());
        for g in games {
            let sessions = self.store.sessions_for(g.id, u32::MAX)?;
            out.push(import::ExportGame {
                game: g.clone(),
                sessions: sessions.into_iter().map(import::ExportSession::from).collect(),
            });
        }
        Ok(ExportDoc::new(out, now_ms()))
    }

    fn import_doc(&mut self, doc: ExportDoc) -> Result<ImportReport, LibraryError> {
        let existing: HashSet<GameId> = self.games.keys().copied().collect();
        let rawg: HashSet<i64> = self.games.values().filter_map(|g| g.rawg_id).collect();
        let detect: HashSet<String> = self
            .games
            .values()
            .filter_map(|g| {
                g.install.as_ref().and_then(|i| i.effective_detect()).map(|p| moodex_core::paths::path_key(p.as_str()))
            })
            .collect();
        let built = import::build_from_doc(doc, &existing, &rawg, &detect)?;
        self.store.import_batch(&built.games, &built.sessions)?;
        self.reload()?;
        self.changed(built.games.iter().map(|g| g.id).collect());
        Ok(built.report)
    }

    // Full reload after bulk writes (memory follows disk).
    fn reload(&mut self) -> Result<(), LibraryError> {
        self.games = self.store.load_games()?.into_iter().map(|g| (g.id, g)).collect();
        self.stats = self.store.play_stats_all()?;
        self.estimates = self.store.games_with_estimates()?;
        self.missing = self.games.values().filter(|g| install_missing(g)).map(|g| g.id).collect();
        self.rebuild_registry();
        Ok(())
    }

    // --- Maintenance -----------------------------------------------------

    fn roll_backup(&self) {
        let Some(dir) = &self.backup_dir else { return };
        let today = OffsetDateTime::now_utc().date();
        match roll_daily_backup(&self.store, dir, today, self.settings.backup_retention_days as usize) {
            Ok(r) => {
                if let Some(c) = r.created {
                    tracing::info!(path = %c, pruned = r.pruned.len(), "daily backup written");
                }
            }
            Err(e) => tracing::error!(error = %e, "daily backup failed"),
        }
    }

    fn daily_maintenance(&mut self) {
        self.roll_backup();
        match self.store.cache_prune(now_ms() - time::Duration::days(30)) {
            Ok(n) if n > 0 => tracing::info!(n, "pruned stale HTTP cache rows"),
            Ok(_) => {}
            Err(e) => tracing::warn!(error = %e, "cache prune failed"),
        }
        let images = self.images.clone();
        tokio::spawn(async move {
            match images.gc().await {
                Ok(r) => tracing::info!(removed = r.removed, bytes = r.bytes_after, "image cache gc"),
                Err(e) => tracing::warn!(error = %e, "image cache gc failed"),
            }
        });
    }

    // Close live sessions cleanly so nothing relies on crash recovery.
    fn shutdown(&mut self) {
        let now = now_ms();
        let ids: Vec<GameId> = self.open.keys().copied().collect();
        for id in ids {
            self.finish(id, now, true);
        }
        if let Err(e) = self.store.checkpoint() {
            tracing::warn!(error = %e, "WAL checkpoint on shutdown failed");
        }
    }
}

// Does the install point at files that no longer exist?
fn install_missing(g: &Game) -> bool {
    let Some(i) = &g.install else { return false };
    let launch_gone = match &i.launch {
        LaunchTarget::Exe { path, .. } | LaunchTarget::Shortcut { path } => !path.exists(),
        LaunchTarget::Uri { .. } => false,
    };
    let detect_gone = i.detect_exe.as_ref().is_some_and(|p| !p.exists());
    launch_gone || detect_gone
}
