// text.rs - Title normalisation helpers; used for ordering and suggestions, never identity.

use camino::Utf8Path;

const ARTICLES: [&str; 3] = ["the ", "a ", "an "];

// "The Witcher 3" -> "witcher 3".
pub fn sort_title(title: &str) -> String {
    let lower = title.trim().to_lowercase();
    for a in ARTICLES {
        if let Some(rest) = lower.strip_prefix(a) {
            let rest = rest.trim_start();
            if !rest.is_empty() {
                return rest.to_string();
            }
        }
    }
    lower
}

// Insert spaces at camelCase and letter/digit boundaries: "Cyberpunk2077" -> "Cyberpunk 2077".
pub fn split_words(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let sep = c == '_' || c == '-' || c == '.' || c.is_whitespace();
        if sep {
            if !out.ends_with(' ') && !out.is_empty() {
                out.push(' ');
            }
            continue;
        }
        if let Some(&p) = i.checked_sub(1).and_then(|j| chars.get(j)) {
            let boundary = (p.is_ascii_lowercase() && c.is_ascii_uppercase())
                || (p.is_ascii_alphabetic() && c.is_ascii_digit())
                || (p.is_ascii_digit() && c.is_ascii_alphabetic())
                || (p.is_ascii_uppercase()
                    && c.is_ascii_uppercase()
                    && chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase()));
            if boundary && !out.ends_with(' ') {
                out.push(' ');
            }
        }
        out.push(c);
    }
    out.trim().to_string()
}

// Lowercase word tokens with punctuation and articles removed, for fuzzy matching.
pub fn match_tokens(s: &str) -> Vec<String> {
    let spaced = split_words(s);
    let cleaned: String =
        spaced.chars().map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { ' ' }).collect();
    cleaned.split_whitespace().filter(|w| !matches!(*w, "the" | "a" | "an" | "of")).map(roman_to_arabic).collect()
}

// Fold common roman numerals so "Witcher III" matches "Witcher 3".
fn roman_to_arabic(w: &str) -> String {
    let v = match w {
        "ii" => "2",
        "iii" => "3",
        "iv" => "4",
        "vi" => "6",
        "vii" => "7",
        "viii" => "8",
        "ix" => "9",
        _ => w,
    };
    v.to_string()
}

// Canonical key used for duplicate suggestions.
pub fn match_key(s: &str) -> String {
    match_tokens(s).join(" ")
}

// Trim, drop empties, dedupe preserving order.
pub fn clean_genres(genres: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(genres.len());
    for g in genres {
        let t = g.trim().to_string();
        if !t.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(&t)) {
            out.push(t);
        }
    }
    out
}

// Folder or file names that say nothing about the game.
fn is_uninformative(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    let n = n.trim();
    matches!(
        n,
        "" | "bin"
            | "bin64"
            | "bin32"
            | "binaries"
            | "win64"
            | "win32"
            | "x64"
            | "x86"
            | "x86_64"
            | "game"
            | "games"
            | "retail"
            | "release"
            | "launcher"
            | "start"
            | "play"
            | "main"
            | "client"
            | "app"
            | "program"
            | "setup"
            | "run"
            | "shipping"
            | "common"
            | "steamapps"
            | "program files"
            | "program files (x86)"
            | "engine"
            | "content"
    ) || n.len() <= 2
}

// Strip engine suffixes such as "-Win64-Shipping".
pub(crate) fn strip_engine_suffix(stem: &str) -> &str {
    let lower = stem.to_ascii_lowercase();
    for suf in ["-win64-shipping", "-win32-shipping", "-wingdk-shipping", "_x64", "_dx12", "_dx11", "-shipping"] {
        if lower.ends_with(suf) && lower.len() > suf.len() + 1 {
            return &stem[..stem.len() - suf.len()];
        }
    }
    stem
}

// Nearest parent folder whose name says something about the game.
fn informative_ancestor(path: &Utf8Path) -> Option<&str> {
    let mut cur = path.parent();
    while let Some(dir) = cur {
        if let Some(name) = dir.file_name() {
            if !is_uninformative(name) {
                return Some(name);
            }
        }
        cur = dir.parent();
    }
    None
}

// All lowercase words, stop words kept (for initials).
fn raw_words(s: &str) -> Vec<String> {
    split_words(s)
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

// The folder spells out the stem: same first word, and the stem's remaining letters are
// prefixes of, or initials of, the folder's remaining words ("OnimushaWotS" -> "Onimusha Way of the Sword").
fn expands(stem: &str, folder: &str) -> bool {
    let s = raw_words(stem);
    let f = raw_words(folder);
    let (Some(s0), Some(f0)) = (s.first(), f.first()) else { return false };
    if s0 != f0 || f.len() <= s.len() {
        return false;
    }
    let rest: String = s.iter().skip(1).map(String::as_str).collect();
    if rest.is_empty() {
        return false;
    }
    let inits: String = f.iter().skip(1).filter_map(|w| w.chars().next()).collect();
    let prefixes = s.iter().skip(1).all(|t| f.iter().skip(1).any(|w| w.starts_with(t.as_str())));
    inits.starts_with(&rest) || prefixes
}

// Derive a human title from an exe path, falling back to parent folders.
pub fn title_from_exe_path(path: &Utf8Path) -> String {
    let stem = path.file_stem().unwrap_or("");
    let stem = strip_engine_suffix(stem);
    if !is_uninformative(stem) {
        // "OnimushaWotS.exe" in "Onimusha.Way.of.the.Sword": the folder spells the abbreviation out.
        if let Some(folder) = informative_ancestor(path) {
            if expands(stem, folder) {
                return split_words(folder);
            }
        }
        return split_words(stem);
    }
    let mut cur = path.parent();
    while let Some(dir) = cur {
        if let Some(name) = dir.file_name() {
            if !is_uninformative(name) {
                return split_words(name);
            }
        }
        cur = dir.parent();
    }
    if stem.is_empty() {
        "Untitled game".to_string()
    } else {
        split_words(stem)
    }
}

// Levenshtein similarity in [0, 1].
pub fn similarity(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let max = a.len().max(b.len());
    if max == 0 {
        return 1.0;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    let dist = prev[b.len()];
    1.0 - dist as f64 / max as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_title_strips_articles() {
        assert_eq!(sort_title("The Witcher 3"), "witcher 3");
        assert_eq!(sort_title("A Plague Tale"), "plague tale");
        assert_eq!(sort_title("An Ode"), "ode");
        assert_eq!(sort_title("  Halo  "), "halo");
        assert_eq!(sort_title("The"), "the");
        assert_eq!(sort_title("Theme Hospital"), "theme hospital");
    }

    #[test]
    fn split_words_handles_camel_and_digits() {
        assert_eq!(split_words("Cyberpunk2077"), "Cyberpunk 2077");
        assert_eq!(split_words("RedDeadRedemption2"), "Red Dead Redemption 2");
        assert_eq!(split_words("witcher3"), "witcher 3");
        assert_eq!(split_words("HLSL_Tool"), "HLSL Tool");
        assert_eq!(split_words("my_game-name"), "my game name");
    }

    #[test]
    fn match_tokens_folds_numerals_and_punctuation() {
        assert_eq!(match_key("The Witcher III: Wild Hunt"), "witcher 3 wild hunt");
        assert_eq!(match_key("witcher 3 - wild hunt"), "witcher 3 wild hunt");
    }

    #[test]
    fn title_from_exe_uses_stem_when_informative() {
        assert_eq!(title_from_exe_path(Utf8Path::new("C:/Games/x/Cyberpunk2077.exe")), "Cyberpunk 2077");
        assert_eq!(title_from_exe_path(Utf8Path::new("C:/Games/Stray/Stray-Win64-Shipping.exe")), "Stray");
    }

    #[test]
    fn title_from_exe_falls_back_to_folder() {
        assert_eq!(title_from_exe_path(Utf8Path::new("C:/Games/Hollow Knight/bin/x64/launcher.exe")), "Hollow Knight");
        assert_eq!(title_from_exe_path(Utf8Path::new("D:/Celeste/game.exe")), "Celeste");
    }

    #[test]
    fn folder_expands_abbreviated_stem() {
        let p = Utf8Path::new("D:/Games/Onimusha.Way.of.the.Sword/OnimushaWotS.exe");
        assert_eq!(title_from_exe_path(p), "Onimusha Way of the Sword");
        let w = Utf8Path::new("D:/Games/The Witcher 3 Wild Hunt/bin/x64/witcher3.exe");
        assert_eq!(title_from_exe_path(w), "witcher 3");
        // A different game's folder never wins.
        assert_eq!(title_from_exe_path(Utf8Path::new("D:/Games/Collection/Hades.exe")), "Hades");
        assert_eq!(title_from_exe_path(Utf8Path::new("D:/Games/Hades II Early/Hades.exe")), "Hades");
    }

    #[test]
    fn similarity_bounds() {
        assert!((similarity("abc", "abc") - 1.0).abs() < 1e-9);
        assert!(similarity("abc", "xyz") < 0.01);
        assert!((similarity("", "") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn clean_genres_dedupes() {
        let g = clean_genres(vec!["Action".into(), " action ".into(), "".into(), "RPG".into()]);
        assert_eq!(g, vec!["Action".to_string(), "RPG".to_string()]);
    }
}
