// paths.rs - Exact path identity for process detection (spec 1.6); no substring matching.

use crate::model::GameId;
use std::collections::HashMap;

// Case-insensitive, prefix-free, separator-normalised key for an absolute path.
pub fn path_key(raw: &str) -> String {
    let mut s = raw.trim().trim_matches('"').replace('/', "\\");
    if let Some(rest) = s.strip_prefix("\\\\?\\UNC\\") {
        s = format!("\\\\{rest}");
    } else if let Some(rest) = s.strip_prefix("\\\\?\\") {
        s = rest.to_string();
    } else if let Some(rest) = s.strip_prefix("\\??\\") {
        s = rest.to_string();
    }
    let unc = s.starts_with("\\\\");
    let mut out = String::with_capacity(s.len());
    let mut last_sep = false;
    for c in s.chars() {
        if c == '\\' {
            if last_sep {
                continue;
            }
            last_sep = true;
        } else {
            last_sep = false;
        }
        out.push(c);
    }
    if unc {
        out.insert(0, '\\');
    }
    while out.ends_with('\\') && out.len() > 3 {
        out.pop();
    }
    out.to_lowercase()
}

// Map from canonical detect path to the game that owns it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DetectRegistry {
    by_key: HashMap<String, GameId>,
}

impl DetectRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // Build from already-canonicalised detect paths.
    pub fn from_entries<I, S>(entries: I) -> Self
    where
        I: IntoIterator<Item = (S, GameId)>,
        S: AsRef<str>,
    {
        let by_key = entries.into_iter().map(|(p, id)| (path_key(p.as_ref()), id)).collect();
        Self { by_key }
    }

    // Full-path match only. A game without a detect path is never auto-detected.
    pub fn match_path(&self, canonical_exe_path: &str) -> Option<GameId> {
        if canonical_exe_path.trim().is_empty() {
            return None;
        }
        self.by_key.get(&path_key(canonical_exe_path)).copied()
    }

    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    // Exact (case-insensitive) file-name membership; a cheap pre-filter, never a match by itself.
    pub fn contains_file_name(&self, name: &str) -> bool {
        let n = name.to_lowercase();
        !n.is_empty() && self.by_key.keys().any(|k| k.rsplit('\\').next() == Some(n.as_str()))
    }

    // Lowercased file names in the registry - used to pre-filter WMI events cheaply.
    pub fn file_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.by_key.keys().filter_map(|k| k.rsplit('\\').next().map(str::to_string)).collect();
        v.sort();
        v.dedup();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_normalise_case_prefix_and_separators() {
        assert_eq!(path_key("C:/Games/Foo/GAME.EXE"), "c:\\games\\foo\\game.exe");
        assert_eq!(path_key("\\\\?\\C:\\Games\\Foo\\game.exe"), "c:\\games\\foo\\game.exe");
        assert_eq!(path_key("C:\\\\Games\\Foo\\\\game.exe"), "c:\\games\\foo\\game.exe");
        assert_eq!(path_key("\\\\?\\UNC\\server\\share\\g.exe"), "\\\\server\\share\\g.exe");
        assert_eq!(path_key("\"C:\\a b\\c.exe\""), "c:\\a b\\c.exe");
    }

    // Regression for v1 ProcessMonitor.cs:159 (spec 2 row 6).
    #[test]
    fn v1_substring_false_positives_are_rejected() {
        let id = GameId::new();
        let reg = DetectRegistry::from_entries([("C:\\Games\\Foo\\game.exe", id)]);
        assert_eq!(
            reg.match_path("C:\\Program Files\\WindowsApps\\Microsoft.XboxGamingOverlay\\GameBarPresenceWriter.exe"),
            None
        );
        assert_eq!(reg.match_path("C:\\Program Files (x86)\\Steam\\GameOverlayUI.exe"), None);
        assert_eq!(reg.match_path("C:\\Other\\game.exe"), None);
        assert_eq!(reg.match_path("C:\\Games\\Foo\\game.exe.bak"), None);
        assert_eq!(reg.match_path("C:\\Games\\Foo\\mygame.exe"), None);
        assert_eq!(reg.match_path(""), None);
        assert_eq!(reg.match_path("c:/games/foo/GAME.exe"), Some(id));
    }

    #[test]
    fn short_names_do_not_cross_match() {
        let a = GameId::new();
        let b = GameId::new();
        let reg = DetectRegistry::from_entries([("D:\\A\\launcher.exe", a), ("D:\\B\\start.exe", b)]);
        assert_eq!(reg.match_path("D:\\C\\launcher.exe"), None);
        assert_eq!(reg.match_path("D:\\A\\launcher.exe"), Some(a));
        assert_eq!(reg.match_path("D:\\B\\start.exe"), Some(b));
        assert_eq!(reg.file_names(), vec!["launcher.exe".to_string(), "start.exe".to_string()]);
    }
}
