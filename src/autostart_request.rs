use anyhow::{Result, bail};

/// Makes the process switch autostart on or off and exit instead of showing tray icons. The
/// unelevated app starts itself elevated with this request because only administrators may
/// register a task that runs with highest privileges.
const FLAG: &str = "--set-autostart";
const ON: &str = "on";
const OFF: &str = "off";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutostartRequest {
    pub enabled: bool,
    /// `DOMAIN\user` whose sign-in should start the app.
    pub user: String,
}

impl AutostartRequest {
    /// `None` for a normal start; an error when the flag is present but malformed.
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Result<Option<Self>> {
        let mut args = args.into_iter().skip_while(|argument| argument != FLAG);
        if args.next().is_none() {
            return Ok(None);
        }
        let (Some(state), Some(user)) = (args.next(), args.next()) else {
            bail!("{FLAG} needs '{ON}' or '{OFF}' and a user name");
        };
        let enabled = match state.as_str() {
            ON => true,
            OFF => false,
            _ => bail!("{FLAG} expects '{ON}' or '{OFF}', got '{state}'"),
        };
        Ok(Some(Self { enabled, user }))
    }

    /// Command-line arguments that [`Self::from_args`] parses back.
    pub fn to_arguments(&self) -> String {
        let state = if self.enabled { ON } else { OFF };
        // Windows user names cannot contain quotes or end with a backslash, so plain quoting
        // survives the C runtime's argument splitting.
        format!("{FLAG} {state} \"{}\"", self.user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::UI::Shell::CommandLineToArgvW;
    use windows::core::HSTRING;

    /// Splits a command line exactly as the new process will see it.
    fn split(command_line: &str) -> Vec<String> {
        let mut count = 0;
        let argv = unsafe { CommandLineToArgvW(&HSTRING::from(command_line), &mut count) };
        assert!(!argv.is_null(), "CommandLineToArgvW failed");
        let args = (0..count as usize)
            .map(|index| unsafe { (*argv.add(index)).to_string() }.expect("UTF-16 argument"))
            .collect();
        unsafe { LocalFree(Some(HLOCAL(argv.cast()))) };
        args
    }

    #[test]
    fn arguments_round_trip_through_a_command_line() {
        let request = AutostartRequest {
            enabled: true,
            user: r"OFFICE\Jane Doe".to_owned(),
        };

        let args = split(&format!(r"C:\Bin\tray-info.exe {}", request.to_arguments()));

        assert_eq!(
            AutostartRequest::from_args(args).expect("parse"),
            Some(request)
        );
    }

    #[test]
    fn normal_start_is_not_a_request() {
        let args = split(r"C:\Bin\tray-info.exe --wait-for-previous");

        assert_eq!(AutostartRequest::from_args(args).expect("parse"), None);
    }

    #[test]
    fn unknown_state_is_rejected() {
        let args = split(r"C:\Bin\tray-info.exe --set-autostart maybe PC\me");

        assert!(AutostartRequest::from_args(args).is_err());
    }
}
