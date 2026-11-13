// keys.rs - Optional personal RAWG key in Windows Credential Manager; never in SQLite or config (spec 9.1).

use moodex_library::Settings;
use moodex_rawg::KeySource;

const SERVICE: &str = "MooDeX";
const ACCOUNT: &str = "rawg-api-key";

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())
}

pub fn get_user_key() -> Option<String> {
    match entry().and_then(|e| e.get_password().map_err(|e| e.to_string())) {
        Ok(k) if !k.trim().is_empty() => Some(k),
        Ok(_) => None,
        Err(e) => {
            tracing::debug!(error = %e, "no personal RAWG key stored");
            None
        }
    }
}

pub fn set_user_key(key: Option<&str>) -> Result<(), String> {
    let e = entry()?;
    match key.map(str::trim).filter(|k| !k.is_empty()) {
        Some(k) => e.set_password(k).map_err(|e| e.to_string()),
        None => match e.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.to_string()),
        },
    }
}

// Pick the request route from settings (proxy, user key, or none).
pub fn key_source(s: &Settings) -> KeySource {
    match s.rawg_key_source.as_str() {
        "proxy" if !s.rawg_proxy_url.trim().is_empty() => KeySource::Proxy { base_url: s.rawg_proxy_url.clone() },
        "user" => match get_user_key() {
            Some(key) => KeySource::User { key },
            None => KeySource::None,
        },
        _ => KeySource::None,
    }
}
