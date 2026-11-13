// canon.rs - Canonical absolute paths: long names, symlinks and junctions resolved (spec 8.4).

use crate::handle::OwnedHandle;
use camino::{Utf8Path, Utf8PathBuf};
use windows::core::PWSTR;
use windows::Win32::Foundation::WAIT_TIMEOUT;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, WaitForSingleObject, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
};

// Strip the verbatim prefix std::fs::canonicalize adds.
fn strip_verbatim(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{rest}")
    } else if let Some(rest) = s.strip_prefix("\\\\?\\") {
        rest.to_string()
    } else {
        s.to_string()
    }
}

// Resolve to the final on-disk path (GetFinalPathNameByHandle normalises 8.3 names too).
pub fn canonicalize(path: &Utf8Path) -> std::io::Result<Utf8PathBuf> {
    let real = std::fs::canonicalize(path.as_std_path())?;
    let s = real.to_str().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "path is not UTF-8"))?;
    Ok(Utf8PathBuf::from(strip_verbatim(s)))
}

// Full image path of a running process, if we may query it.
pub fn process_image_path(pid: u32) -> Option<String> {
    // SAFETY: OpenProcess has no memory-safety preconditions; failure is returned as Err.
    let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let h = OwnedHandle::new(h)?;
    let mut buf = vec![0u16; 32768];
    let mut len = buf.len() as u32;
    // SAFETY: buf has `len` u16 slots and outlives the call; len is updated to the written length.
    let ok = unsafe { QueryFullProcessImageNameW(h.raw(), PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len) };
    if ok.is_err() {
        return None;
    }
    buf.truncate(len as usize);
    String::from_utf16(&buf).ok()
}

// True while the process exists; a zero-timeout wait, not an enumeration.
pub fn is_process_alive(pid: u32) -> bool {
    // SAFETY: OpenProcess has no memory-safety preconditions; failure means we cannot see it.
    let Ok(h) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, false, pid) }) else {
        return false;
    };
    let Some(h) = OwnedHandle::new(h) else { return false };
    // SAFETY: the handle is live; a zero timeout only polls the signalled state.
    let state = unsafe { WaitForSingleObject(h.raw(), 0) };
    state == WAIT_TIMEOUT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liveness() {
        assert!(is_process_alive(std::process::id()));
        assert!(!is_process_alive(0xFFFF_FFF0));
    }

    #[test]
    fn canonicalizes_system_paths() {
        let p = canonicalize(Utf8Path::new("C:/Windows/System32/../System32/cmd.exe")).unwrap();
        assert!(!p.as_str().starts_with("\\\\?\\"));
        assert_eq!(p.as_str().to_lowercase(), "c:\\windows\\system32\\cmd.exe");
    }

    #[test]
    fn expands_short_names_when_present() {
        // 8.3 names may be disabled on a volume; only assert when the short form resolves.
        if let Ok(p) = canonicalize(Utf8Path::new("C:\\PROGRA~1")) {
            assert!(!p.as_str().contains('~'), "{p}");
        }
    }

    #[test]
    fn own_image_path_is_readable() {
        let me = process_image_path(std::process::id()).unwrap();
        assert!(me.to_lowercase().ends_with(".exe"));
    }
}
