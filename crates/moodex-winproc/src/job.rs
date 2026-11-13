// job.rs - Unnamed job object + IO completion port; exit of the whole tree is an OS event (spec 8.4 layer A).

use crate::handle::OwnedHandle;
use std::sync::Arc;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectAssociateCompletionPortInformation, SetInformationJobObject,
    JOBOBJECT_ASSOCIATE_COMPLETION_PORT,
};
use windows::Win32::System::IO::{CreateIoCompletionPort, GetQueuedCompletionStatus, OVERLAPPED};

// winnt.h JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO.
const MSG_ACTIVE_PROCESS_ZERO: u32 = 4;
// winnt.h JOB_OBJECT_MSG_NEW_PROCESS.
const MSG_NEW_PROCESS: u32 = 6;

#[derive(Debug)]
pub struct JobObject {
    job: OwnedHandle,
    port: Arc<OwnedHandle>,
}

impl JobObject {
    // Create the job and associate its completion port before any process joins.
    pub fn create() -> windows::core::Result<Self> {
        // SAFETY: null attributes and null name create a private, unnamed job.
        let job = unsafe { CreateJobObjectW(None, PCWSTR::null())? };
        let job = OwnedHandle::new(job).ok_or_else(windows::core::Error::from_thread)?;
        // SAFETY: INVALID_HANDLE_VALUE with no existing port creates a fresh completion port.
        let port = unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, None, 0, 1)? };
        let port = OwnedHandle::new(port).ok_or_else(windows::core::Error::from_thread)?;
        let info = JOBOBJECT_ASSOCIATE_COMPLETION_PORT { CompletionKey: job.raw().0, CompletionPort: port.raw() };
        // SAFETY: `info` is a valid, correctly sized struct that outlives the call; both handles are live.
        // Breakaway is NOT enabled, so children cannot leave the job (spec 8.4).
        unsafe {
            SetInformationJobObject(
                job.raw(),
                JobObjectAssociateCompletionPortInformation,
                std::ptr::from_ref(&info).cast(),
                std::mem::size_of::<JOBOBJECT_ASSOCIATE_COMPLETION_PORT>() as u32,
            )?;
        }
        Ok(Self { job, port: Arc::new(port) })
    }

    // Add a process (and therefore all its future descendants) to the job.
    pub fn assign(&self, process: HANDLE) -> windows::core::Result<()> {
        // SAFETY: both handles are valid for the duration of the call; the caller owns `process`.
        unsafe { AssignProcessToJobObject(self.job.raw(), process) }
    }

    // Block until the last process in the tree exits. Run on a dedicated thread.
    pub fn wait_until_empty(&self) -> Result<u32, String> {
        wait_port(&self.port)
    }

    // A clonable waiter that keeps the port alive independently of the job handle.
    pub fn waiter(&self) -> impl FnOnce() -> Result<u32, String> + Send + 'static {
        let port = Arc::clone(&self.port);
        move || wait_port(&port)
    }
}

// Drain completion packets until ACTIVE_PROCESS_ZERO; returns how many processes joined.
fn wait_port(port: &OwnedHandle) -> Result<u32, String> {
    let mut spawned = 0u32;
    loop {
        let mut code = 0u32;
        let mut key = 0usize;
        let mut ov: *mut OVERLAPPED = std::ptr::null_mut();
        // SAFETY: all out-pointers reference live locals; the port handle is kept alive by the Arc.
        let r = unsafe { GetQueuedCompletionStatus(port.raw(), &mut code, &mut key, &mut ov, u32::MAX) };
        if let Err(e) = r {
            return Err(e.to_string());
        }
        match code {
            MSG_NEW_PROCESS => spawned = spawned.saturating_add(1),
            MSG_ACTIVE_PROCESS_ZERO => return Ok(spawned),
            _ => {}
        }
    }
}
