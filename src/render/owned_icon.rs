use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};

use crate::win::debug_log;

/// An `HICON` created by this process and destroyed when dropped.
pub struct OwnedIcon(HICON);

impl OwnedIcon {
    pub fn new(handle: HICON) -> Self {
        Self(handle)
    }

    pub fn handle(&self) -> HICON {
        self.0
    }
}

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        if let Err(error) = unsafe { DestroyIcon(self.0) } {
            debug_log(&format!("DestroyIcon failed: {error}"));
        }
    }
}
