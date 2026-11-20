use std::collections::BTreeMap;

use anyhow::Result;
use windows::Win32::Foundation::HWND;

use super::TrayIconId;
use super::tray_icon::TrayIcon;
use crate::render::IconSpec;

/// All icons this process currently shows in the notification area.
pub struct TrayIcons {
    hwnd: HWND,
    icons: BTreeMap<TrayIconId, TrayIcon>,
}

impl TrayIcons {
    pub fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            icons: BTreeMap::new(),
        }
    }

    /// Adds the icon or updates it in place.
    pub fn show(
        &mut self,
        id: TrayIconId,
        spec: IconSpec,
        tooltip: String,
        size: i32,
    ) -> Result<()> {
        match self.icons.get_mut(&id) {
            Some(icon) => icon.update(spec, tooltip, size),
            None => {
                let icon = TrayIcon::add(self.hwnd, id, spec, tooltip, size)?;
                self.icons.insert(id, icon);
                Ok(())
            }
        }
    }

    pub fn hide(&mut self, id: TrayIconId) {
        self.icons.remove(&id);
    }

    pub fn retain(&mut self, mut keep: impl FnMut(TrayIconId) -> bool) {
        self.icons.retain(|&id, _| keep(id));
    }

    pub fn register_all(&self) {
        for icon in self.icons.values() {
            icon.register();
        }
    }
}
