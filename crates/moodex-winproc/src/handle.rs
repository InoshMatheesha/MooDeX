// handle.rs - RAII kernel handle; closed exactly once on drop (fixes spec 2 row 7).

use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};

#[derive(Debug)]
pub struct OwnedHandle(HANDLE);

impl OwnedHandle {
    // Take ownership; None for null or invalid handles.
    pub fn new(h: HANDLE) -> Option<Self> {
        if h.is_invalid() || h == INVALID_HANDLE_VALUE || h.0.is_null() {
            None
        } else {
            Some(Self(h))
        }
    }

    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: we own this handle (constructed only from a fresh, valid handle) and close it once.
        if let Err(e) = unsafe { CloseHandle(self.0) } {
            tracing::warn!(error = %e, "CloseHandle failed");
        }
    }
}

// SAFETY: kernel handles are process-wide values usable from any thread; the type has no thread affinity.
unsafe impl Send for OwnedHandle {}
// SAFETY: all operations through &OwnedHandle are thread-safe kernel calls on the same handle value.
unsafe impl Sync for OwnedHandle {}
