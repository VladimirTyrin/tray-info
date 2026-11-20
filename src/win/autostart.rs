use anyhow::{Context, Result};
use windows::Win32::Foundation::{E_ACCESSDENIED, ERROR_FILE_NOT_FOUND};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::System::TaskScheduler::{
    IExecAction, ITaskFolder, ITaskService, TASK_CREATE_OR_UPDATE, TASK_LOGON_INTERACTIVE_TOKEN,
    TaskScheduler,
};
use windows::Win32::System::Variant::VARIANT;
use windows::core::{BSTR, HRESULT, Interface};

use super::TaskRunLevel;

const TASK_NAME: &str = "tray-info";

/// Starts the app at the user's sign-in through a Task Scheduler logon task. Unlike the `Run`
/// registry key, a task can start the program elevated without a UAC prompt, which the CPU
/// temperature sensor needs. Requires COM to be initialised on the calling thread.
pub struct Autostart {
    // Owned so tests can register under a throw-away name and point at a fake executable.
    task_name: String,
    executable: String,
    /// `DOMAIN\user` whose sign-in starts the app.
    user: String,
    run_level: TaskRunLevel,
}

impl Autostart {
    /// Autostart of this executable for the user running it.
    pub fn for_current_user() -> Result<Self> {
        let domain = std::env::var("USERDOMAIN").context("USERDOMAIN is not set")?;
        let name = std::env::var("USERNAME").context("USERNAME is not set")?;
        Self::for_user(format!(r"{domain}\{name}"))
    }

    /// Autostart of this executable for `user`: an elevated helper may run under another
    /// administrator account (over-the-shoulder UAC), so it is told whose sign-in to use.
    pub fn for_user(user: String) -> Result<Self> {
        let exe = std::env::current_exe().context("cannot resolve own executable path")?;
        Ok(Self {
            task_name: TASK_NAME.to_owned(),
            executable: exe.display().to_string(),
            user,
            run_level: TaskRunLevel::Highest,
        })
    }

    pub fn user(&self) -> &str {
        &self.user
    }

    /// True only when an enabled task starts this very executable, so a task left by a moved
    /// copy shows as off and turning autostart on repoints it.
    pub fn is_enabled(&self) -> Result<bool> {
        let task = match unsafe { root_folder()?.GetTask(&BSTR::from(&self.task_name)) } {
            Ok(task) => task,
            Err(error) if is_not_found(&error) => return Ok(false),
            Err(error) => return Err(error).context("cannot open the autostart task"),
        };
        if !unsafe { task.Enabled() }?.as_bool() {
            return Ok(false);
        }
        let action = unsafe { task.Definition()?.Actions()?.get_Item(1) }
            .context("autostart task has no action")?;
        let Ok(exec) = action.cast::<IExecAction>() else {
            return Ok(false);
        };
        let mut path = BSTR::new();
        unsafe { exec.Path(&mut path) }?;
        Ok(path.to_string().eq_ignore_ascii_case(&self.executable))
    }

    /// Fails with an access-denied error (see [`is_access_denied`]) when an elevated task is
    /// registered or removed without administrator rights.
    pub fn set_enabled(&self, enabled: bool) -> Result<()> {
        let folder = root_folder()?;
        let name = BSTR::from(&self.task_name);
        if enabled {
            let empty = VARIANT::default();
            unsafe {
                folder.RegisterTask(
                    &name,
                    &BSTR::from(self.task_xml()),
                    TASK_CREATE_OR_UPDATE.0,
                    &empty,
                    &empty,
                    TASK_LOGON_INTERACTIVE_TOKEN,
                    &empty,
                )
            }
            .context("cannot register the autostart task")?;
            return Ok(());
        }
        match unsafe { folder.DeleteTask(&name, 0) } {
            Err(error) if is_not_found(&error) => Ok(()),
            result => result.context("cannot delete the autostart task"),
        }
    }

    fn task_xml(&self) -> String {
        let user = escape_xml(&self.user);
        let executable = escape_xml(&self.executable);
        let run_level = self.run_level.xml_value();
        // Defaults that would break a tray app are overridden: tasks are killed after 72 hours
        // and do not start on battery power.
        format!(
            r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Tray indicators: Bluetooth battery levels, CPU temperature and power.</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>{run_level}</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Enabled>true</Enabled>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{executable}</Command>
    </Exec>
  </Actions>
</Task>"#
        )
    }
}

/// True when the error says the caller lacks the rights for the operation.
pub fn is_access_denied(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<windows::core::Error>()
        .is_some_and(|error| error.code() == E_ACCESSDENIED)
}

fn is_not_found(error: &windows::core::Error) -> bool {
    error.code() == HRESULT::from_win32(ERROR_FILE_NOT_FOUND.0)
}

fn root_folder() -> Result<ITaskFolder> {
    let service: ITaskService =
        unsafe { CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER) }
            .context("cannot create the Task Scheduler service object")?;
    let local = VARIANT::default();
    unsafe { service.Connect(&local, &local, &local, &local) }
        .context("cannot connect to Task Scheduler")?;
    unsafe { service.GetFolder(&BSTR::from(r"\")) }.context("cannot open the task root folder")
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

    /// An unelevated task under a throw-away name, pointing at a program that does not exist;
    /// deleted when dropped.
    struct Fixture {
        autostart: Autostart,
    }

    impl Fixture {
        fn new(test: &str) -> Self {
            // S_FALSE (already initialised on this thread) is fine too.
            unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
                .ok()
                .expect("COM initialisation");
            let current = Autostart::for_current_user().expect("current user");
            let fixture = Self {
                autostart: Autostart {
                    task_name: format!("tray-info-test-{test}"),
                    executable: r"C:\tray-info-test\missing & <odd>.exe".to_owned(),
                    run_level: TaskRunLevel::Least,
                    ..current
                },
            };
            // Leftovers of an earlier, aborted run would skew the result.
            fixture.autostart.set_enabled(false).expect("clean up");
            fixture
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            if let Err(error) = self.autostart.set_enabled(false) {
                eprintln!("cannot delete test task: {error:#}");
            }
        }
    }

    #[test]
    fn disabled_when_never_registered() {
        let fixture = Fixture::new("never_registered");

        assert!(!fixture.autostart.is_enabled().expect("read"));
    }

    #[test]
    fn enabling_and_disabling_round_trip() {
        let fixture = Fixture::new("round_trip");

        fixture.autostart.set_enabled(true).expect("enable");
        assert!(fixture.autostart.is_enabled().expect("read"));

        fixture.autostart.set_enabled(false).expect("disable");
        assert!(!fixture.autostart.is_enabled().expect("read"));
    }

    #[test]
    fn task_of_another_copy_counts_as_disabled() {
        let fixture = Fixture::new("another_copy");
        fixture.autostart.set_enabled(true).expect("enable");

        let moved = Autostart {
            task_name: fixture.autostart.task_name.clone(),
            executable: r"D:\Elsewhere\tray-info.exe".to_owned(),
            user: fixture.autostart.user.clone(),
            run_level: TaskRunLevel::Least,
        };

        assert!(!moved.is_enabled().expect("read"));
    }
}
