use anyhow::Result;
use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, HANDLE, WAIT_ABANDONED_0, WAIT_OBJECT_0,
};
use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};
use windows::core::w;

use super::{InstanceWait, debug_log};

/// A relaunched instance gives the previous one this long to shut down.
const PREVIOUS_INSTANCE_EXIT_MS: u32 = 10_000;

/// Ownership of the per-session named mutex that keeps one tray-info per user session.
pub struct SingleInstance(HANDLE);

impl SingleInstance {
    /// Returns `None` when another instance already owns the session.
    pub fn acquire(wait: InstanceWait) -> Result<Option<Self>> {
        let handle =
            match unsafe { CreateMutexW(None, false, w!("Local\\tray-info.single-instance")) } {
                Ok(handle) => handle,
                // An elevated instance creates the mutex with an admin-only DACL.
                Err(error) if error.code() == ERROR_ACCESS_DENIED.to_hresult() => return Ok(None),
                Err(error) => return Err(anyhow::Error::new(error).context("CreateMutexW failed")),
            };
        let timeout = match wait {
            InstanceWait::NoWait => 0,
            InstanceWait::ForPrevious => PREVIOUS_INSTANCE_EXIT_MS,
        };
        // WAIT_ABANDONED means the previous owner exited without releasing: ownership is ours.
        let result = unsafe { WaitForSingleObject(handle, timeout) };
        if result == WAIT_OBJECT_0 || result == WAIT_ABANDONED_0 {
            return Ok(Some(Self(handle)));
        }
        close(handle);
        Ok(None)
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        if let Err(error) = unsafe { ReleaseMutex(self.0) } {
            debug_log(&format!("ReleaseMutex failed: {error}"));
        }
        close(self.0);
    }
}

fn close(handle: HANDLE) {
    if let Err(error) = unsafe { CloseHandle(handle) } {
        debug_log(&format!("CloseHandle failed: {error}"));
    }
}
