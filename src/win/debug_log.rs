use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;
use windows::core::HSTRING;

/// Writes a diagnostic line visible in DebugView / an attached debugger.
/// The app has no console and no log file, and this costs nothing when nobody listens.
pub fn debug_log(message: &str) {
    let line = HSTRING::from(format!("tray-info: {message}\n"));
    unsafe { OutputDebugStringW(&line) };
}
