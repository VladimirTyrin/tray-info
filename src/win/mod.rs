//! Thin safe wrappers over the Win32 APIs the app needs.

mod autostart;
mod debug_log;
mod dev_node;
mod elevation;
mod instance_wait;
mod multi_sz;
mod pdh_counter;
mod single_instance;
mod task_run_level;
mod wide_text;
mod wmi_argument;
mod wmi_failure;
mod wmi_method;

pub use autostart::{Autostart, is_access_denied};
pub use debug_log::debug_log;
pub use dev_node::{DevNode, present_device_ids};
pub use elevation::{WAIT_FOR_PREVIOUS_FLAG, relaunch_elevated, run_self_elevated};
pub use instance_wait::InstanceWait;
pub use pdh_counter::PdhCounter;
pub use single_instance::SingleInstance;
pub use task_run_level::TaskRunLevel;
pub use wmi_argument::WmiArgument;
pub use wmi_failure::WmiFailure;
pub use wmi_method::WmiMethod;
