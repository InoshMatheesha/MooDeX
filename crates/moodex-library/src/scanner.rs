// scanner.rs - Auto-detect games in user-chosen folders; watches them (ReadDirectoryChangesW, no polling).

use crate::{LibraryEvent, LibraryHandle};
use camino::{Utf8Path, Utf8PathBuf};
use moodex_core::matching::rank_matches;
use moodex_core::paths::path_key;
use moodex_core::scan::{folder_title, is_junk_dir, pick_main_exe};
use moodex_core::{GameDraft, GameFilter, GameId, Install, LaunchTarget, SortOrder, Status};
use moodex_rawg::RawgClient;
use notify::{EventKind, RecursiveMode, Watcher};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot};

// Wait this long after the last file change before scanning (installers write for a while).
const SETTLE: Duration = Duration::from_secs(20);
const MAX_DEPTH: usize = 5;
const MAX_ENTRIES: usize = 6000;
// Minimum ranking score to trust a RAWG match for cover and metadata.
const MATCH_THRESHOLD: f64 = 0.85;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ScanReport {
    pub scanned: usize,
    pub added: Vec<String>,
    pub linked: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScanEvent {
    Started,
    Finished { report: ScanReport },
}

enum Cmd {
    ScanAll { reply: Option<oneshot::Sender<ScanReport>> },
    Changed(Utf8PathBuf),
}

#[derive(Clone)]
pub struct ScannerHandle {
    tx: mpsc::Sender<Cmd>,
    events: broadcast::Sender<ScanEvent>,
}

impl ScannerHandle {
    // Full scan of every folder now; resolves with what was found.
    pub async fn scan_now(&self) -> Option<ScanReport> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(Cmd::ScanAll { reply: Some(tx) }).await.ok()?;
        rx.await.ok()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ScanEvent> {
        self.events.subscribe()
    }
}

// The game folder (a direct child of a scan root, or of steamapps\common) that contains `path`.
pub fn game_folder_for(path: &Utf8Path, roots: &[String]) -> Option<Utf8PathBuf> {
    let key = path_key(path.as_str());
    for root in roots {
        for folder in game_folders(Utf8Path::new(root)) {
            let fk = path_key(folder.as_str());
            if key == fk || key.starts_with(&format!("{fk}\\")) {
                return Some(folder);
            }
        }
    }
    None
}

fn subdirs(dir: &Utf8Path) -> Vec<Utf8PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Utf8PathBuf> = rd
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
        .filter(|p| p.file_name().is_some_and(|n| !n.starts_with('.') && !is_junk_dir(n)))
        .collect();
    out.sort();
    out
}

fn has_exe(dir: &Utf8Path) -> bool {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .any(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")) && e.path().is_file())
        })
        .unwrap_or(false)
}

// Each game lives in its own folder: children of the root, Steam's steamapps\common, or the root itself.
pub fn game_folders(root: &Utf8Path) -> Vec<Utf8PathBuf> {
    let common = root.join("steamapps").join("common");
    if common.is_dir() {
        return subdirs(&common);
    }
    if root.file_name().is_some_and(|n| n.eq_ignore_ascii_case("common"))
        && root.parent().and_then(Utf8Path::file_name).is_some_and(|n| n.eq_ignore_ascii_case("steamapps"))
    {
        return subdirs(root);
    }
    if has_exe(root) {
        return vec![root.to_owned()];
    }
    let mut out = Vec::new();
    for d in subdirs(root) {
        if d.file_name().is_some_and(|n| n.eq_ignore_ascii_case("steamapps")) {
            out.extend(subdirs(&d.join("common")));
        } else {
            out.push(d);
        }
    }
    out
}

// Every .exe under a game folder (relative path, size), bounded in depth and count.
fn collect_exes(folder: &Utf8Path) -> Vec<(Utf8PathBuf, u64)> {
    let mut out = Vec::new();
    let mut stack = vec![(folder.to_owned(), 0usize)];
    let mut seen = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.filter_map(Result::ok) {
            seen += 1;
            if seen > MAX_ENTRIES {
                return out;
            }
            let Ok(p) = Utf8PathBuf::from_path_buf(e.path()) else { continue };
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                if depth < MAX_DEPTH && p.file_name().is_some_and(|n| !is_junk_dir(n)) {
                    stack.push((p, depth + 1));
                }
            } else if ft.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")) {
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                if let Ok(rel) = p.strip_prefix(folder) {
                    out.push((rel.to_owned(), size));
                }
            }
        }
    }
    out
}

pub struct Scanner {
    library: LibraryHandle,
    rawg: RawgClient,
    events: broadcast::Sender<ScanEvent>,
    folders: Vec<String>,
    ignored: Vec<String>,
    watcher: Option<notify::RecommendedWatcher>,
    tx: mpsc::Sender<Cmd>,
}

impl Scanner {
    pub fn spawn(library: LibraryHandle, rawg: RawgClient) -> ScannerHandle {
        let (tx, rx) = mpsc::channel(512);
        let (events, _) = broadcast::channel(32);
        let s = Scanner {
            library,
            rawg,
            events: events.clone(),
            folders: Vec::new(),
            ignored: Vec::new(),
            watcher: None,
            tx: tx.clone(),
        };
        tokio::spawn(s.run(rx));
        ScannerHandle { tx, events }
    }

    async fn run(mut self, mut rx: mpsc::Receiver<Cmd>) {
        let mut lib = self.library.subscribe();
        if let Ok(s) = self.library.settings().await {
            self.configure(s.scan_folders, s.scan_ignored);
            if !self.folders.is_empty() {
                self.scan_all().await;
            }
        }
        let mut pending: HashSet<Utf8PathBuf> = HashSet::new();
        let settle = tokio::time::sleep(Duration::from_secs(3600 * 24 * 365));
        tokio::pin!(settle);
        loop {
            tokio::select! {
                cmd = rx.recv() => match cmd {
                    None => break,
                    Some(Cmd::ScanAll { reply }) => {
                        let r = self.scan_all().await;
                        if let Some(tx) = reply {
                            if tx.send(r).is_err() {
                                tracing::debug!("scan caller went away");
                            }
                        }
                    }
                    Some(Cmd::Changed(p)) => {
                        if let Some(f) = game_folder_for(&p, &self.folders) {
                            pending.insert(f);
                            settle.as_mut().reset(tokio::time::Instant::now() + SETTLE);
                        }
                    }
                },
                ev = lib.recv() => match ev {
                    Ok(LibraryEvent::SettingsChanged { settings }) => {
                        let changed = settings.scan_folders != self.folders;
                        self.ignored = settings.scan_ignored.clone();
                        if changed {
                            self.configure(settings.scan_folders, settings.scan_ignored);
                            if !self.folders.is_empty() {
                                self.scan_all().await;
                            }
                        }
                    }
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                },
                () = &mut settle, if !pending.is_empty() => {
                    let batch: Vec<Utf8PathBuf> = pending.drain().collect();
                    tracing::info!(folders = batch.len(), "new files settled; scanning changed game folders");
                    self.scan(batch).await;
                    settle.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(3600 * 24 * 365));
                }
            }
        }
    }

    // (Re)create the directory watcher for the configured folders.
    fn configure(&mut self, folders: Vec<String>, ignored: Vec<String>) {
        self.folders = folders;
        self.ignored = ignored;
        self.watcher = None;
        if self.folders.is_empty() {
            return;
        }
        let tx = self.tx.clone();
        let handler = move |res: notify::Result<notify::Event>| {
            let Ok(ev) = res else { return };
            if !matches!(ev.kind, EventKind::Create(_) | EventKind::Modify(notify::event::ModifyKind::Name(_))) {
                return;
            }
            for p in ev.paths {
                // Only new executables matter; ignore saves, shaders and logs.
                if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")) {
                    continue;
                }
                if let Ok(p) = Utf8PathBuf::from_path_buf(p) {
                    if tx.try_send(Cmd::Changed(p)).is_err() {
                        tracing::debug!("scan queue full; the next full scan will catch it");
                    }
                }
            }
        };
        match notify::recommended_watcher(handler) {
            Ok(mut w) => {
                for f in &self.folders {
                    if let Err(e) = w.watch(Utf8Path::new(f).as_std_path(), RecursiveMode::Recursive) {
                        tracing::warn!(folder = %f, error = %e, "cannot watch game folder");
                    }
                }
                tracing::info!(folders = self.folders.len(), "watching game folders for new installs");
                self.watcher = Some(w);
            }
            Err(e) => tracing::warn!(error = %e, "folder watcher unavailable; use Scan now"),
        }
    }

    async fn scan_all(&mut self) -> ScanReport {
        let roots: Vec<Utf8PathBuf> = self.folders.iter().map(Utf8PathBuf::from).collect();
        let folders =
            tokio::task::spawn_blocking(move || roots.iter().flat_map(|r| game_folders(r)).collect::<Vec<_>>())
                .await
                .unwrap_or_default();
        self.scan(folders).await
    }

    async fn scan(&mut self, folders: Vec<Utf8PathBuf>) -> ScanReport {
        if self.events.send(ScanEvent::Started).is_err() {
            tracing::trace!("no scan listeners");
        }
        let mut report = ScanReport { scanned: folders.len(), ..ScanReport::default() };
        let known = self.known_installs().await;
        let owned: HashMap<i64, GameId> = self.library.rawg_ids().await.unwrap_or_default().into_iter().collect();
        let ignored: HashSet<String> = self.ignored.iter().map(|p| path_key(p)).collect();
        for folder in folders {
            let fk = path_key(folder.as_str());
            if ignored.contains(&fk) || known.iter().any(|k| k == &fk || k.starts_with(&format!("{fk}\\"))) {
                continue;
            }
            match self.scan_folder(&folder, &owned).await {
                Ok(Some((title, linked))) => {
                    if linked {
                        report.linked.push(title);
                    } else {
                        report.added.push(title);
                    }
                }
                Ok(None) => {}
                Err(e) => report.errors.push(format!("{folder}: {e}")),
            }
        }
        tracing::info!(
            scanned = report.scanned,
            added = report.added.len(),
            linked = report.linked.len(),
            "game scan finished"
        );
        if self.events.send(ScanEvent::Finished { report: report.clone() }).is_err() {
            tracing::trace!("no scan listeners");
        }
        report
    }

    // Install paths already in the library (launch and detect), as keys.
    async fn known_installs(&self) -> Vec<String> {
        let f = GameFilter { installed_only: true, ..GameFilter::default() };
        let views = self.library.list_games(f, SortOrder::Title).await.unwrap_or_default();
        let mut out = Vec::new();
        for v in views {
            let Some(i) = v.install else { continue };
            if let LaunchTarget::Exe { path, .. } | LaunchTarget::Shortcut { path } = &i.launch {
                out.push(path_key(path.as_str()));
            }
            if let Some(d) = &i.detect_exe {
                out.push(path_key(d.as_str()));
            }
        }
        out
    }

    // Returns (title, linked_to_existing) when something was added.
    async fn scan_folder(
        &self,
        folder: &Utf8Path,
        owned: &HashMap<i64, GameId>,
    ) -> Result<Option<(String, bool)>, String> {
        let name = folder.file_name().unwrap_or("").to_string();
        let f = folder.to_owned();
        let n = name.clone();
        let exe = tokio::task::spawn_blocking(move || pick_main_exe(&n, &collect_exes(&f)))
            .await
            .map_err(|e| e.to_string())?;
        let Some(rel) = exe else { return Ok(None) };
        let exe = folder.join(rel);
        let title = folder_title(&name);
        let install = Install {
            launch: LaunchTarget::Exe { path: exe.clone(), args: vec![], working_dir: None },
            detect_exe: None,
        };

        // A confident RAWG match gives the cover; if that game is already in the library, link it instead.
        let matched = match self.rawg.search(&title, 1).await {
            Ok(page) => rank_matches(&title, &page.results)
                .into_iter()
                .next()
                .filter(|s| s.score >= MATCH_THRESHOLD)
                .map(|s| s.item),
            Err(e) => {
                tracing::debug!(%title, error = %e, "no RAWG match for scanned game");
                None
            }
        };
        if let Some(m) = &matched {
            if let Some(existing) = owned.get(&m.id) {
                let has_install =
                    self.library.get_raw(*existing).await.ok().flatten().is_some_and(|g| g.install.is_some());
                if has_install {
                    return Ok(None);
                }
                self.library.set_install(*existing, Some(install)).await.map_err(|e| e.to_string())?;
                tracing::info!(%title, %exe, "linked scanned install to existing library game");
                return Ok(Some((m.name.clone(), true)));
            }
        }
        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        let draft = match matched {
            Some(m) => GameDraft {
                title: m.name.clone(),
                rawg_id: Some(m.id),
                cover_url: m.background_image.clone(),
                release_date: m.released.as_deref().and_then(|d| time::Date::parse(d, &fmt).ok()),
                genres: m.genres.clone(),
                status: Status::Backlog,
                install: Some(install),
                ..GameDraft::default()
            },
            None => GameDraft {
                title: title.clone(),
                status: Status::Backlog,
                install: Some(install),
                ..GameDraft::default()
            },
        };
        let shown = draft.title.clone();
        match self.library.add_game(draft).await {
            Ok(_) => {
                tracing::info!(title = %shown, %exe, "auto-added game from scan folder");
                Ok(Some((shown, false)))
            }
            // Already present by exe or RAWG link: nothing to do.
            Err(crate::LibraryError::Conflict(_)) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: &Utf8Path, bytes: usize) {
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).unwrap();
        }
        std::fs::write(p, vec![0u8; bytes]).unwrap();
    }

    #[test]
    fn finds_game_folders_and_main_exes() {
        let d = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(d.path().to_path_buf()).unwrap();
        touch(&root.join("Hades").join("Hades.exe"), 300_000);
        touch(&root.join("Hades").join("unins000.exe"), 300_000);
        touch(&root.join("Crimson Desert").join("bin64").join("CrimsonDesert.exe"), 400_000);
        touch(&root.join("_CommonRedist").join("vc_redist.x64.exe"), 300_000);
        let folders = game_folders(&root);
        assert_eq!(folders.len(), 2, "{folders:?}");
        let cd = root.join("Crimson Desert");
        let exes = collect_exes(&cd);
        assert_eq!(pick_main_exe("Crimson Desert", &exes), Some(Utf8Path::new("bin64").join("CrimsonDesert.exe")));
        let hit = game_folder_for(&cd.join("bin64").join("CrimsonDesert.exe"), &[root.to_string()]);
        assert_eq!(hit.as_deref(), Some(cd.as_path()));
    }

    #[test]
    fn steam_library_uses_steamapps_common() {
        let d = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(d.path().to_path_buf()).unwrap();
        touch(&root.join("steamapps").join("common").join("Portal 2").join("portal2.exe"), 300_000);
        touch(&root.join("steamapps").join("common").join("Celeste").join("Celeste.exe"), 300_000);
        let names: Vec<String> = game_folders(&root).iter().filter_map(|p| p.file_name().map(str::to_string)).collect();
        assert_eq!(names, vec!["Celeste".to_string(), "Portal 2".to_string()]);
    }
}
