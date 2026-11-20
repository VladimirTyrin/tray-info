use anyhow::Result;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NOTIFY_ICON_MESSAGE, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::WM_APP;

use super::TrayIconId;
use crate::render::{IconSpec, OwnedIcon, paint_icon};
use crate::win::debug_log;

/// Window message the shell sends for mouse and keyboard events on our icons.
pub const TRAY_CALLBACK_MESSAGE: u32 = WM_APP + 1;

/// One icon in the notification area. Removed from the tray when dropped.
pub struct TrayIcon {
    hwnd: HWND,
    id: TrayIconId,
    spec: IconSpec,
    tooltip: String,
    size: i32,
    icon: OwnedIcon,
}

impl TrayIcon {
    pub fn add(
        hwnd: HWND,
        id: TrayIconId,
        spec: IconSpec,
        tooltip: String,
        size: i32,
    ) -> Result<Self> {
        let icon = paint_icon(&spec, size)?;
        let tray_icon = Self {
            hwnd,
            id,
            spec,
            tooltip,
            size,
            icon,
        };
        // Failing here is not fatal: when Explorer is not up yet (early autostart) the icon is
        // registered again on the TaskbarCreated broadcast.
        tray_icon.register();
        Ok(tray_icon)
    }

    /// Adds the icon to the shell; also used after Explorer restarts and forgets all icons.
    pub fn register(&self) {
        let mut data = self.data();
        if !self.notify(NIM_ADD, &data) {
            return;
        }
        // Version 4 gives one consistent callback format (event in LOWORD, anchor point in wParam).
        data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        self.notify(NIM_SETVERSION, &data);
    }

    /// Repaints only when something visible changed, keeping steady-state updates free.
    pub fn update(&mut self, spec: IconSpec, tooltip: String, size: i32) -> Result<()> {
        if self.spec == spec && self.tooltip == tooltip && self.size == size {
            return Ok(());
        }
        if self.spec != spec || self.size != size {
            self.icon = paint_icon(&spec, size)?;
            self.spec = spec;
            self.size = size;
        }
        self.tooltip = tooltip;
        let data = self.data();
        self.notify(NIM_MODIFY, &data);
        Ok(())
    }

    fn data(&self) -> NOTIFYICONDATAW {
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: self.id.value(),
            uFlags: NIF_ICON | NIF_TIP | NIF_MESSAGE | NIF_SHOWTIP,
            uCallbackMessage: TRAY_CALLBACK_MESSAGE,
            hIcon: self.icon.handle(),
            ..Default::default()
        };
        // Leave room for the terminating NUL; longer tooltips are cut.
        let capacity = data.szTip.len() - 1;
        for (slot, unit) in data
            .szTip
            .iter_mut()
            .zip(self.tooltip.encode_utf16().take(capacity))
        {
            *slot = unit;
        }
        data
    }

    fn notify(&self, message: NOTIFY_ICON_MESSAGE, data: &NOTIFYICONDATAW) -> bool {
        let succeeded = unsafe { Shell_NotifyIconW(message, data) }.as_bool();
        if !succeeded {
            debug_log(&format!(
                "Shell_NotifyIconW({}) failed for icon {:#x}",
                message.0,
                self.id.value()
            ));
        }
        succeeded
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        let data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: self.id.value(),
            ..Default::default()
        };
        self.notify(NIM_DELETE, &data);
    }
}
