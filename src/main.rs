#![windows_subsystem = "windows"]

use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
use windows::core::{HSTRING, w};

fn main() {
    if let Err(error) = tray_info::run() {
        // There is no console in the windows subsystem, so a message box is the only visible channel.
        let text = HSTRING::from(format!("{error:#}"));
        unsafe { MessageBoxW(None, &text, w!("tray-info"), MB_OK | MB_ICONERROR) };
    }
}
