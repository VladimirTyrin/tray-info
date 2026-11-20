use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::core::w;

/// Colour scheme of the taskbar the icons are drawn on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskbarTheme {
    Dark,
    Light,
}

impl TaskbarTheme {
    /// Reads the "Windows mode" setting; defaults to dark, which is the Windows 11 default.
    pub fn current() -> Self {
        let mut value = 0u32;
        let mut size = size_of::<u32>() as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
                w!("SystemUsesLightTheme"),
                RRF_RT_REG_DWORD,
                None,
                Some((&raw mut value).cast()),
                Some(&mut size),
            )
        };
        if status.is_ok() && value != 0 {
            Self::Light
        } else {
            Self::Dark
        }
    }
}
