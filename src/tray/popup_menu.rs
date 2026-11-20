use anyhow::{Context, Result};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, HMENU, MENU_ITEM_FLAGS, MF_CHECKED, MF_SEPARATOR,
    MF_STRING, PostMessageW, SetForegroundWindow, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    TrackPopupMenu, WM_NULL,
};
use windows::core::{HSTRING, PCWSTR};

use super::{MenuCommand, MenuEntry};
use crate::win::debug_log;

/// Shows the context menu at the given screen point and returns the chosen command.
/// Blocks in a modal loop that keeps dispatching messages (timers keep ticking).
pub fn show_popup_menu(
    hwnd: HWND,
    x: i32,
    y: i32,
    entries: &[MenuEntry],
) -> Result<Option<MenuCommand>> {
    let menu = unsafe { CreatePopupMenu() }.context("CreatePopupMenu failed")?;
    let result = fill(menu, entries).map(|()| track(hwnd, menu, x, y));
    if let Err(error) = unsafe { DestroyMenu(menu) } {
        debug_log(&format!("DestroyMenu failed: {error}"));
    }
    result
}

fn fill(menu: HMENU, entries: &[MenuEntry]) -> Result<()> {
    for entry in entries {
        match entry {
            MenuEntry::Separator => {
                unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()) }
                    .context("AppendMenuW(separator) failed")?;
            }
            MenuEntry::Command(command) => {
                append(menu, MF_STRING, command.id() as usize, command.label())?;
            }
            MenuEntry::Toggle { command, checked } => {
                let flags = if *checked {
                    MF_STRING | MF_CHECKED
                } else {
                    MF_STRING
                };
                append(menu, flags, command.id() as usize, command.label())?;
            }
        }
    }
    Ok(())
}

fn append(menu: HMENU, flags: MENU_ITEM_FLAGS, id: usize, text: &str) -> Result<()> {
    let text = HSTRING::from(text);
    unsafe { AppendMenuW(menu, flags, id, &text) }.context("AppendMenuW failed")
}

fn track(hwnd: HWND, menu: HMENU, x: i32, y: i32) -> Option<MenuCommand> {
    // Without being the foreground window the menu would not close when clicking elsewhere.
    if !unsafe { SetForegroundWindow(hwnd) }.as_bool() {
        debug_log("SetForegroundWindow failed before showing the menu");
    }
    let selected = unsafe {
        TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
            x,
            y,
            None,
            hwnd,
            None,
        )
    };
    // Documented workaround (KB135788): a dummy message makes the next menu open reliably.
    if let Err(error) = unsafe { PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) } {
        debug_log(&format!("PostMessageW(WM_NULL) failed: {error}"));
    }
    MenuCommand::from_id(selected.0 as u32)
}
