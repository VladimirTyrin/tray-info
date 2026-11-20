use anyhow::{Context, Result, bail};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR, w};

/// Command-line flag telling the new instance to wait for this one to exit.
pub const WAIT_FOR_PREVIOUS_FLAG: &str = "--wait-for-previous";

/// Starts this executable again with administrator rights (shows the UAC prompt); the new
/// instance takes over once this one exits. Fails if the user declines the prompt.
pub fn relaunch_elevated() -> Result<()> {
    run_self_elevated(WAIT_FOR_PREVIOUS_FLAG)
}

/// Starts another copy of this executable with administrator rights and the given arguments
/// (shows the UAC prompt) without waiting for it. Fails if the user declines the prompt.
pub fn run_self_elevated(arguments: &str) -> Result<()> {
    let exe = std::env::current_exe().context("cannot resolve own executable path")?;
    let exe = HSTRING::from(exe.as_os_str());
    let arguments = HSTRING::from(arguments);
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("runas"),
            &exe,
            &arguments,
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute reports success as a pseudo-HINSTANCE greater than 32.
    if result.0 as usize <= 32 {
        bail!(
            "ShellExecuteW(runas) failed with code {}",
            result.0 as usize
        );
    }
    Ok(())
}
