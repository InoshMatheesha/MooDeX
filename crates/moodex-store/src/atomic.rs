// atomic.rs - Crash-safe file writes: temp file in the same folder, fsync, then rename (spec 1.4).

use crate::StoreError;
use camino::Utf8Path;
use std::io::Write;

pub fn write_atomic(path: &Utf8Path, bytes: &[u8]) -> Result<(), StoreError> {
    let dir = path.parent().filter(|p| !p.as_str().is_empty()).unwrap_or_else(|| Utf8Path::new("."));
    std::fs::create_dir_all(dir)?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path.as_std_path()).map_err(|e| StoreError::Io(e.error))?;
    Ok(())
}
