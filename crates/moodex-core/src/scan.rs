// scan.rs - Pure rules for auto-detecting games in a folder: skip junk, pick the main exe.

use crate::text::{match_key, similarity, split_words, strip_engine_suffix};
use camino::{Utf8Path, Utf8PathBuf};

// Folders that never contain the game itself.
const JUNK_DIRS: [&str; 22] = [
    "_commonredist",
    "commonredist",
    "redist",
    "redists",
    "redistributables",
    "_redist",
    "directx",
    "dotnet",
    "vcredist",
    "support",
    "__installer",
    "installer",
    "installers",
    "easyanticheat",
    "battleye",
    "prereq",
    "prerequisites",
    "thirdparty",
    "crashreportclient",
    "_data",
    "mono",
    "manual",
];

// Executable name fragments that are tools, installers or helpers, never the game.
const JUNK_NAMES: [&str; 30] = [
    "unins",
    "uninstall",
    "setup",
    "install",
    "redist",
    "vcredist",
    "vc_redist",
    "dxsetup",
    "dxwebsetup",
    "directx",
    "crash",
    "report",
    "prereq",
    "easyanticheat",
    "beservice",
    "battleye",
    "dotnet",
    "python",
    "helper",
    "updater",
    "update",
    "notification",
    "cefprocess",
    "webhelper",
    "quicksfv",
    "oalinst",
    "physx",
    "touchup",
    "cleanup",
    "unitycrashhandler",
];

// Is this directory name one we never descend into?
pub fn is_junk_dir(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    JUNK_DIRS.contains(&n.as_str()) || n.starts_with("_commonredist") || n.ends_with("redist")
}

// Is this exe a helper rather than a game?
pub fn is_junk_exe(path: &Utf8Path) -> bool {
    let stem = path.file_stem().unwrap_or("").to_ascii_lowercase();
    if stem.is_empty() {
        return true;
    }
    JUNK_NAMES.iter().any(|j| stem.contains(j))
}

// Human title for a game folder: "Onimusha.Way.of.the.Sword" -> "Onimusha Way of the Sword".
pub fn folder_title(name: &str) -> String {
    let t = split_words(name);
    if t.is_empty() {
        name.to_string()
    } else {
        t
    }
}

// Score an exe candidate against its game folder; higher is more likely the game.
fn score(folder: &str, rel: &Utf8Path, size: u64) -> f64 {
    let raw = rel.file_stem().unwrap_or("");
    let lower = raw.to_ascii_lowercase();
    let stem = strip_engine_suffix(raw);
    let mut s = 3.0 * similarity(&match_key(stem), &match_key(folder));
    let fk = match_key(folder);
    let sk = match_key(stem);
    if let (Some(a), Some(b)) = (fk.split(' ').next(), sk.split(' ').next()) {
        if !a.is_empty() && a == b {
            s += 1.0;
        }
    }
    // Bigger binaries are usually the game; capped so size never dominates the name.
    let mb = size as f64 / 1_048_576.0;
    s += (mb.max(0.1).ln() / 200f64.ln()).clamp(-0.5, 1.0);
    // Unreal's real game binary.
    if lower.ends_with("-win64-shipping") || lower.ends_with("-wingdk-shipping") {
        s += 0.6;
    }
    if lower.contains("launcher") {
        s -= 0.8;
    }
    // Prefer shallow paths.
    s -= 0.25 * rel.components().count().saturating_sub(1) as f64;
    s
}

// Pick the main executable among candidates (paths relative to the game folder, with sizes).
pub fn pick_main_exe(folder_name: &str, candidates: &[(Utf8PathBuf, u64)]) -> Option<Utf8PathBuf> {
    candidates
        .iter()
        .filter(|(p, size)| !is_junk_exe(p) && *size >= 100 * 1024)
        .map(|(p, size)| (score(folder_name, p, *size), p))
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, p)| p.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(p: &str, mb: u64) -> (Utf8PathBuf, u64) {
        (Utf8PathBuf::from(p), mb * 1_048_576)
    }

    #[test]
    fn junk_is_filtered() {
        assert!(is_junk_exe(Utf8Path::new("unins000.exe")));
        assert!(is_junk_exe(Utf8Path::new("UnityCrashHandler64.exe")));
        assert!(is_junk_exe(Utf8Path::new("vc_redist.x64.exe")));
        assert!(!is_junk_exe(Utf8Path::new("Hades.exe")));
        assert!(is_junk_dir("_CommonRedist"));
        assert!(is_junk_dir("EasyAntiCheat"));
        assert!(!is_junk_dir("bin64"));
    }

    #[test]
    fn picks_named_game_exe() {
        let picks = vec![
            c("unins000.exe", 3),
            c("OnimushaWotS.exe", 180),
            c("UnityCrashHandler64.exe", 2),
            c("tools/ModTool.exe", 40),
        ];
        assert_eq!(
            pick_main_exe("Onimusha.Way.of.the.Sword", &picks).as_deref(),
            Some(Utf8Path::new("OnimushaWotS.exe"))
        );
    }

    #[test]
    fn prefers_unreal_shipping_binary_over_stub() {
        let picks = vec![c("Stray.exe", 1), c("Hk_project/Binaries/Win64/Stray-Win64-Shipping.exe", 90)];
        assert_eq!(
            pick_main_exe("Stray", &picks).as_deref(),
            Some(Utf8Path::new("Hk_project/Binaries/Win64/Stray-Win64-Shipping.exe"))
        );
    }

    #[test]
    fn prefers_game_over_launcher() {
        let picks = vec![c("Launcher.exe", 30), c("bin64/CrimsonDesert.exe", 300)];
        assert_eq!(pick_main_exe("Crimson Desert", &picks).as_deref(), Some(Utf8Path::new("bin64/CrimsonDesert.exe")));
    }

    #[test]
    fn nothing_usable_is_none() {
        assert_eq!(pick_main_exe("X", &[c("setup.exe", 5), (Utf8PathBuf::from("tiny.exe"), 2048)]), None);
    }

    #[test]
    fn titles_from_folders() {
        assert_eq!(folder_title("Onimusha.Way.of.the.Sword"), "Onimusha Way of the Sword");
        assert_eq!(folder_title("Hollow Knight"), "Hollow Knight");
    }
}
