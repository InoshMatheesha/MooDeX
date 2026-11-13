// tracker.rs - Turns launches and WMI events into PlayEvents; sole owner of "what is running" (spec 8.4-8.5).

use crate::{now_ms, LibraryEvent, LibraryHandle, PlayEvent};
use moodex_core::{GameId, SessionSource};
use moodex_winproc::{
    is_process_alive, match_process, scan_running, ExitReason, LaunchMethod, ProcEvent, ProcEventKind, SharedRegistry,
    TrackedLaunch,
};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};

const HEARTBEAT: Duration = Duration::from_secs(30);
// How long a promoted launch may take to show the real game before it is abandoned.
const PROMOTION_GRACE: Duration = Duration::from_secs(180);

enum Input {
    Launching { game_id: GameId, ack: tokio::sync::oneshot::Sender<()> },
    LaunchFailed { game_id: GameId },
    Launched { game_id: GameId, launch: TrackedLaunch, has_detect: bool },
    JobDone { game_id: GameId, reason: ExitReason, elapsed: Duration },
    Stub(Duration),
}

#[derive(Debug, Default)]
struct Tracked {
    job_alive: bool,
    pids: HashSet<u32>,
    has_detect: bool,
    session_open: bool,
    // A Play is in flight: WMI must not open a competing session.
    launching: bool,
    promoted_until: Option<Instant>,
}

#[derive(Clone)]
pub struct TrackerHandle {
    tx: mpsc::Sender<Input>,
}

impl TrackerHandle {
    // Announce a Play before the process exists, so WMI cannot race the job.
    pub async fn launching(&self, game_id: GameId) {
        let (ack, done) = tokio::sync::oneshot::channel();
        if self.tx.send(Input::Launching { game_id, ack }).await.is_err() {
            tracing::debug!("tracker gone");
            return;
        }
        // Wait until the tracker has recorded it, so no later scan can race the launch.
        if done.await.is_err() {
            tracing::debug!("tracker dropped launching ack");
        }
    }

    pub async fn launch_failed(&self, game_id: GameId) {
        if self.tx.send(Input::LaunchFailed { game_id }).await.is_err() {
            tracing::debug!("tracker gone");
        }
    }

    // Hand a fresh launch to the tracker; it opens the session and watches the tree.
    pub async fn launched(&self, game_id: GameId, launch: TrackedLaunch, has_detect: bool) {
        if self.tx.send(Input::Launched { game_id, launch, has_detect }).await.is_err() {
            tracing::error!(%game_id, "tracker is gone; launch will not be tracked");
        }
    }

    // Launcher-stub threshold follows the setting.
    pub async fn set_stub_threshold(&self, d: Duration) {
        if self.tx.send(Input::Stub(d)).await.is_err() {
            tracing::debug!("tracker gone");
        }
    }
}

pub struct PlayTracker {
    library: LibraryHandle,
    registry: SharedRegistry,
    games: HashMap<GameId, Tracked>,
    pid_owner: HashMap<u32, GameId>,
    stub_threshold: Duration,
    tx: mpsc::Sender<Input>,
}

impl PlayTracker {
    // Spawn the tracker task; `proc_events` is the WMI stream (None if WMI is unavailable).
    pub fn spawn(
        library: LibraryHandle,
        registry: SharedRegistry,
        proc_events: Option<mpsc::Receiver<ProcEvent>>,
        stub_threshold: Duration,
    ) -> TrackerHandle {
        let (tx, rx) = mpsc::channel(64);
        let lib_events = library.subscribe();
        let t = PlayTracker {
            library,
            registry,
            games: HashMap::new(),
            pid_owner: HashMap::new(),
            stub_threshold,
            tx: tx.clone(),
        };
        tokio::spawn(t.run(rx, proc_events, lib_events));
        TrackerHandle { tx }
    }

    async fn run(
        mut self,
        mut rx: mpsc::Receiver<Input>,
        mut procs: Option<mpsc::Receiver<ProcEvent>>,
        mut lib: broadcast::Receiver<LibraryEvent>,
    ) {
        // Layer C at boot: games already running before MooDeX started.
        self.rescan().await;
        let mut tick = tokio::time::interval(HEARTBEAT);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        tick.tick().await;
        loop {
            tokio::select! {
                // Own inputs first: a queued "launching" must be seen before any WMI event.
                biased;
                input = rx.recv() => match input {
                    Some(i) => self.on_input(i).await,
                    None => break,
                },
                ev = async { match procs.as_mut() { Some(r) => r.recv().await, None => std::future::pending().await } } => match ev {
                    Some(e) => self.on_proc(e).await,
                    None => {
                        tracing::warn!("process event stream closed; WMI tracking disabled");
                        procs = None;
                    }
                },
                le = lib.recv() => match le {
                    Ok(LibraryEvent::RegistryChanged) => self.rescan().await,
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => self.rescan().await,
                    Err(broadcast::error::RecvError::Closed) => break,
                },
                _ = tick.tick() => self.heartbeat().await,
            }
        }
        tracing::info!("play tracker stopped");
    }

    async fn send(&self, ev: PlayEvent) {
        if let Err(e) = self.library.play_event(ev).await {
            tracing::error!(error = %e, "could not deliver play event");
        }
    }

    async fn start_session(&mut self, game_id: GameId, source: SessionSource) {
        let t = self.games.entry(game_id).or_default();
        if t.session_open {
            return;
        }
        t.session_open = true;
        self.send(PlayEvent::Started { game_id, source, at: now_ms() }).await;
    }

    async fn end_session(&mut self, game_id: GameId) {
        if let Some(t) = self.games.remove(&game_id) {
            for pid in t.pids {
                self.pid_owner.remove(&pid);
            }
            if t.session_open {
                self.send(PlayEvent::Ended { game_id, at: now_ms() }).await;
            }
        }
    }

    async fn on_input(&mut self, i: Input) {
        match i {
            Input::Stub(d) => self.stub_threshold = d,
            Input::Launching { game_id, ack } => {
                self.games.entry(game_id).or_default().launching = true;
                if ack.send(()).is_err() {
                    tracing::debug!("launcher stopped waiting");
                }
            }
            Input::LaunchFailed { game_id } => {
                let drop_entry = match self.games.get_mut(&game_id) {
                    Some(t) => {
                        t.launching = false;
                        !t.session_open && t.pids.is_empty()
                    }
                    None => false,
                };
                if drop_entry {
                    self.games.remove(&game_id);
                } else if self.games.get(&game_id).is_some_and(|t| !t.session_open && !t.pids.is_empty()) {
                    self.start_session(game_id, SessionSource::Wmi).await;
                }
            }
            Input::Launched { game_id, launch, has_detect } => {
                let tracked_by_job = launch.method != LaunchMethod::Untracked;
                {
                    let t = self.games.entry(game_id).or_default();
                    t.has_detect = t.has_detect || has_detect;
                    t.job_alive = tracked_by_job;
                    t.launching = false;
                }
                if tracked_by_job {
                    self.start_session(game_id, SessionSource::JobObject).await;
                } else if self.games.get(&game_id).is_some_and(|t| !t.pids.is_empty()) {
                    // The shell launch gave no handle but WMI already sees the game.
                    self.start_session(game_id, SessionSource::Wmi).await;
                }
                let tx = self.tx.clone();
                let started = launch.started;
                let completion = launch.completion;
                // Keep the job alive for the life of the watch.
                let job = launch.job;
                tokio::spawn(async move {
                    let reason = completion.await.unwrap_or(ExitReason::WaitFailed("waiter dropped".into()));
                    drop(job);
                    let elapsed = started.elapsed();
                    if tx.send(Input::JobDone { game_id, reason, elapsed }).await.is_err() {
                        tracing::debug!("tracker gone before job completion");
                    }
                });
            }
            Input::JobDone { game_id, reason, elapsed } => {
                let threshold = self.stub_threshold;
                let Some(t) = self.games.get_mut(&game_id) else { return };
                t.job_alive = false;
                if !t.pids.is_empty() {
                    // The real game is already visible via WMI; it now owns the session.
                    tracing::info!(%game_id, ?reason, "launch tree exited; game still running (WMI)");
                    return;
                }
                if elapsed < threshold && t.has_detect {
                    // Launcher stub (Steam/Epic): promote tracking to WMI (spec 8.5 step 6).
                    t.promoted_until = Some(Instant::now() + PROMOTION_GRACE);
                    tracing::info!(%game_id, ?elapsed, "launcher stub exited; tracking promoted to WMI");
                    return;
                }
                tracing::info!(%game_id, ?reason, ?elapsed, "game exited");
                self.end_session(game_id).await;
            }
        }
    }

    async fn on_proc(&mut self, ev: ProcEvent) {
        match ev.kind {
            ProcEventKind::Started => {
                let reg = match self.registry.read() {
                    Ok(r) => r.clone(),
                    Err(p) => p.into_inner().clone(),
                };
                let Some(game_id) = match_process(&ev, &reg) else { return };
                self.attach(ev.pid, game_id).await;
            }
            ProcEventKind::Exited => {
                tracing::debug!(pid = ev.pid, name = %ev.name, "process exit event");
                let Some(game_id) = self.pid_owner.remove(&ev.pid) else { return };
                let done = match self.games.get_mut(&game_id) {
                    Some(t) => {
                        t.pids.remove(&ev.pid);
                        t.pids.is_empty() && !t.job_alive && t.promoted_until.is_none()
                    }
                    None => false,
                };
                if done {
                    tracing::info!(%game_id, pid = ev.pid, "detected game exited");
                    self.end_session(game_id).await;
                }
            }
        }
    }

    async fn attach(&mut self, pid: u32, game_id: GameId) {
        self.pid_owner.insert(pid, game_id);
        let t = self.games.entry(game_id).or_default();
        t.pids.insert(pid);
        t.promoted_until = None;
        let open = t.session_open || t.launching;
        if !open {
            tracing::info!(%game_id, pid, "game detected via WMI");
            self.start_session(game_id, SessionSource::Wmi).await;
        }
    }

    async fn rescan(&mut self) {
        let reg = match self.registry.read() {
            Ok(r) => r.clone(),
            Err(p) => p.into_inner().clone(),
        };
        let found = tokio::task::spawn_blocking(move || scan_running(&reg)).await.unwrap_or_default();
        for (pid, game_id) in found {
            if self.pid_owner.get(&pid) != Some(&game_id) {
                self.attach(pid, game_id).await;
            }
        }
    }

    // Every 30 s: confirm liveness, heartbeat live sessions, close dead ones.
    async fn heartbeat(&mut self) {
        let now = Instant::now();
        let ids: Vec<GameId> = self.games.keys().copied().collect();
        for game_id in ids {
            let Some(t) = self.games.get_mut(&game_id) else { continue };
            // A missed WMI deletion must not keep a session alive forever.
            let dead: Vec<u32> = t.pids.iter().copied().filter(|p| !is_process_alive(*p)).collect();
            for p in dead {
                t.pids.remove(&p);
                self.pid_owner.remove(&p);
            }
            let Some(t) = self.games.get(&game_id) else { continue };
            let alive = t.job_alive || !t.pids.is_empty();
            if alive {
                if t.session_open {
                    self.send(PlayEvent::Heartbeat { game_id, at: now_ms() }).await;
                }
                continue;
            }
            match t.promoted_until {
                Some(until) if now < until => {
                    if t.session_open {
                        self.send(PlayEvent::Heartbeat { game_id, at: now_ms() }).await;
                    }
                }
                Some(_) => {
                    tracing::info!(%game_id, "promoted launch never showed the game; abandoning");
                    self.games.remove(&game_id);
                    self.send(PlayEvent::Abandoned { game_id }).await;
                }
                None => self.end_session(game_id).await,
            }
        }
    }
}
