use windows::Win32::Foundation::E_ACCESSDENIED;
use windows::Win32::System::Wmi::WBEM_E_ACCESS_DENIED;

/// WMI errors split by what the caller can do about them.
#[derive(Debug)]
pub enum WmiFailure {
    /// The class or method is restricted to administrators.
    AccessDenied,
    Other(anyhow::Error),
}

impl WmiFailure {
    pub fn from_windows(error: windows::core::Error, context: &str) -> Self {
        let code = error.code();
        if code.0 == WBEM_E_ACCESS_DENIED.0 || code == E_ACCESSDENIED {
            Self::AccessDenied
        } else {
            Self::Other(anyhow::Error::new(error).context(context.to_owned()))
        }
    }
}
