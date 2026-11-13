// settings.rs - Typed settings with the defaults from spec Appendix A; validated on write.

use crate::LibraryError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub minimize_to_tray: bool,
    pub launch_at_startup: bool,
    pub start_minimized: bool,
    pub min_session_seconds: u64,
    pub stub_threshold_seconds: u64,
    pub image_cache_budget_mb: u64,
    pub rawg_key_source: String,
    pub rawg_proxy_url: String,
    pub backup_retention_days: u32,
    pub v1_import_done: bool,
    // Folders scanned and watched for newly installed games.
    pub scan_folders: Vec<String>,
    // Game folders the user removed; never auto-added again.
    pub scan_ignored: Vec<String>,
    // Look for a new release on GitHub at startup.
    pub auto_update_check: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            minimize_to_tray: false,
            launch_at_startup: false,
            start_minimized: false,
            min_session_seconds: 60,
            stub_threshold_seconds: 20,
            image_cache_budget_mb: 500,
            rawg_key_source: "proxy".into(),
            rawg_proxy_url: String::new(),
            backup_retention_days: 7,
            v1_import_done: false,
            scan_folders: Vec::new(),
            scan_ignored: Vec::new(),
            auto_update_check: true,
        }
    }
}

fn as_bool(key: &str, v: &Value) -> Result<bool, LibraryError> {
    v.as_bool().ok_or_else(|| LibraryError::Invalid(format!("{key} must be true or false")))
}

// A list of absolute folder paths, trimmed and de-duplicated (case-insensitive).
fn as_paths(key: &str, v: &Value, max: usize) -> Result<Vec<String>, LibraryError> {
    let arr = v.as_array().ok_or_else(|| LibraryError::Invalid(format!("{key} must be a list of folders")))?;
    let mut out: Vec<String> = Vec::new();
    for item in arr {
        let p = item.as_str().ok_or_else(|| LibraryError::Invalid(format!("{key} must contain text paths")))?;
        let p = p.trim().trim_end_matches(['\\', '/']).to_string();
        if p.is_empty() {
            continue;
        }
        if !camino::Utf8Path::new(&p).is_absolute() {
            return Err(LibraryError::Invalid(format!("not an absolute folder: {p}")));
        }
        if !out.iter().any(|o| o.eq_ignore_ascii_case(&p)) {
            out.push(p);
        }
    }
    if out.len() > max {
        return Err(LibraryError::Invalid(format!("{key} may hold at most {max} folders")));
    }
    Ok(out)
}

fn as_range(key: &str, v: &Value, lo: u64, hi: u64) -> Result<u64, LibraryError> {
    match v.as_u64() {
        Some(n) if (lo..=hi).contains(&n) => Ok(n),
        _ => Err(LibraryError::Invalid(format!("{key} must be a whole number between {lo} and {hi}"))),
    }
}

impl Settings {
    // Overlay stored values; unknown or invalid entries are logged and ignored.
    pub fn from_stored(stored: &HashMap<String, Value>) -> Self {
        let mut s = Settings::default();
        for (k, v) in stored {
            if let Err(e) = s.apply(k, v) {
                tracing::warn!(key = %k, error = %e, "ignoring stored setting");
            }
        }
        s
    }

    // Validate and apply one key; returns the normalised value to persist.
    pub fn apply(&mut self, key: &str, v: &Value) -> Result<Value, LibraryError> {
        match key {
            "minimize_to_tray" => self.minimize_to_tray = as_bool(key, v)?,
            "launch_at_startup" => {
                self.launch_at_startup = as_bool(key, v)?;
                if !self.launch_at_startup {
                    self.start_minimized = false;
                }
            }
            "start_minimized" => {
                let b = as_bool(key, v)?;
                // start_minimized requires launch_at_startup (Appendix A).
                if b && !self.launch_at_startup {
                    return Err(LibraryError::Invalid("enable launch at startup first".into()));
                }
                self.start_minimized = b;
            }
            "min_session_seconds" => self.min_session_seconds = as_range(key, v, 0, 3600)?,
            "stub_threshold_seconds" => self.stub_threshold_seconds = as_range(key, v, 1, 600)?,
            "image_cache_budget_mb" => self.image_cache_budget_mb = as_range(key, v, 50, 20_000)?,
            "backup_retention_days" => self.backup_retention_days = as_range(key, v, 1, 90)? as u32,
            "v1_import_done" => self.v1_import_done = as_bool(key, v)?,
            "auto_update_check" => self.auto_update_check = as_bool(key, v)?,
            "scan_folders" => self.scan_folders = as_paths(key, v, 32)?,
            "scan_ignored" => self.scan_ignored = as_paths(key, v, 5000)?,
            "rawg_key_source" => {
                let s = v.as_str().unwrap_or_default();
                if !matches!(s, "proxy" | "user" | "none") {
                    return Err(LibraryError::Invalid("rawg_key_source must be proxy, user or none".into()));
                }
                self.rawg_key_source = s.to_string();
            }
            "rawg_proxy_url" => {
                let s = v.as_str().unwrap_or_default().trim().to_string();
                if !s.is_empty()
                    && !s.starts_with("https://")
                    && !s.starts_with("http://127.0.0.1")
                    && !s.starts_with("http://localhost")
                {
                    return Err(LibraryError::Invalid("the proxy URL must start with https://".into()));
                }
                self.rawg_proxy_url = s;
            }
            other => return Err(LibraryError::Invalid(format!("unknown setting {other}"))),
        }
        Ok(self.value_of(key))
    }

    fn value_of(&self, key: &str) -> Value {
        serde_json::to_value(self).ok().and_then(|o| o.get(key).cloned()).unwrap_or(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_match_spec() {
        let s = Settings::default();
        assert_eq!(s.min_session_seconds, 60);
        assert_eq!(s.stub_threshold_seconds, 20);
        assert_eq!(s.image_cache_budget_mb, 500);
        assert_eq!(s.rawg_key_source, "proxy");
        assert_eq!(s.backup_retention_days, 7);
    }

    #[test]
    fn validation() {
        let mut s = Settings::default();
        assert!(s.apply("start_minimized", &json!(true)).is_err());
        s.apply("launch_at_startup", &json!(true)).unwrap();
        s.apply("start_minimized", &json!(true)).unwrap();
        s.apply("launch_at_startup", &json!(false)).unwrap();
        assert!(!s.start_minimized);
        assert!(s.apply("min_session_seconds", &json!(-1)).is_err());
        assert!(s.apply("rawg_key_source", &json!("evil")).is_err());
        assert!(s.apply("rawg_proxy_url", &json!("http://example.com")).is_err());
        assert_eq!(
            s.apply("rawg_proxy_url", &json!(" https://p.example/api/rawg ")).unwrap(),
            json!("https://p.example/api/rawg")
        );
        assert!(s.apply("nope", &json!(1)).is_err());
        let f = s.apply("scan_folders", &json!(["D:/Games/", " d:/games ", "E:/SteamLibrary"])).unwrap();
        assert_eq!(f, json!(["D:/Games", "E:/SteamLibrary"]));
        assert!(s.apply("scan_folders", &json!(["relative/path"])).is_err());
        assert!(s.apply("scan_folders", &json!("D:/Games")).is_err());
    }

    #[test]
    fn bad_stored_values_fall_back() {
        let mut m = HashMap::new();
        m.insert("min_session_seconds".to_string(), json!("x"));
        m.insert("minimize_to_tray".to_string(), json!(true));
        let s = Settings::from_stored(&m);
        assert_eq!(s.min_session_seconds, 60);
        assert!(s.minimize_to_tray);
    }
}
