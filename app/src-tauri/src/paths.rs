// paths.rs - Where MooDeX keeps its data; v2 lives in its own subfolder so nothing else is touched.

use anyhow::{Context, Result};
use camino::Utf8PathBuf;

#[derive(Debug, Clone)]
pub struct AppDirs {
    // %LOCALAPPDATA%\MooDeX - shared with v1 and other MooDeX-branded apps; read-only for us.
    pub legacy_root: Utf8PathBuf,
    // %LOCALAPPDATA%\MooDeX\v2 - everything this app writes.
    pub root: Utf8PathBuf,
    pub db: Utf8PathBuf,
    pub covers: Utf8PathBuf,
    pub backups: Utf8PathBuf,
    pub logs: Utf8PathBuf,
}

impl AppDirs {
    pub fn resolve() -> Result<Self> {
        let local = std::env::var("LOCALAPPDATA").context("LOCALAPPDATA is not set")?;
        let legacy_root = Utf8PathBuf::from(local).join("MooDeX");
        let root = legacy_root.join("v2");
        let dirs = Self {
            db: root.join("moodex.db"),
            covers: root.join("covers"),
            backups: root.join("backups"),
            logs: root.join("logs"),
            legacy_root,
            root,
        };
        for d in [&dirs.root, &dirs.covers, &dirs.backups, &dirs.logs] {
            std::fs::create_dir_all(d).with_context(|| format!("creating {d}"))?;
        }
        Ok(dirs)
    }

    // The v1 library file, if present.
    pub fn v1_file(&self) -> Option<Utf8PathBuf> {
        let p = self.legacy_root.join("games_data.json");
        p.is_file().then_some(p)
    }
}
