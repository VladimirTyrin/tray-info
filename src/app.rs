use std::collections::BTreeSet;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi};
use windows::Win32::UI::WindowsAndMessaging::SM_CXSMICON;

use crate::autostart_request::AutostartRequest;
use crate::render::{IconSpec, TaskbarTheme, battery_icon, power_icon, temperature_icon};
use crate::sensors::{
    BluetoothDevice, ConnectionState, CpuPowerSensor, CpuTemperatureSensor, TemperatureReading,
    Watts, scan_bluetooth_devices,
};
use crate::tray::{MenuCommand, MenuEntry, TrayIconId, TrayIcons};
use crate::win::{Autostart, SingleInstance, debug_log, is_access_denied, run_self_elevated};

/// Application state: sensors, latest readings and the tray icons that display them.
pub struct App {
    theme: TaskbarTheme,
    icons: TrayIcons,
    temperature_sensor: CpuTemperatureSensor,
    power_sensor: Option<CpuPowerSensor>,
    temperature: TemperatureReading,
    power: Option<Watts>,
    devices: Vec<BluetoothDevice>,
    /// `None` when the executable path is unknown; the menu then has no autostart item.
    autostart: Option<Autostart>,
    /// Held for the app's lifetime; dropping it lets a relaunched instance start.
    _instance: SingleInstance,
}

impl App {
    pub fn new(hwnd: HWND, instance: SingleInstance) -> Self {
        let power_sensor = CpuPowerSensor::open()
            .inspect_err(|error| debug_log(&format!("CPU power unsupported: {error:#}")))
            .ok();
        let temperature_sensor = CpuTemperatureSensor::open();
        let autostart = Autostart::for_current_user()
            .inspect_err(|error| debug_log(&format!("autostart unavailable: {error:#}")))
            .ok();
        Self {
            theme: TaskbarTheme::current(),
            icons: TrayIcons::new(hwnd),
            temperature: temperature_sensor.read(),
            temperature_sensor,
            power_sensor,
            power: None,
            devices: Vec::new(),
            autostart,
            _instance: instance,
        }
    }

    pub fn refresh_cpu(&mut self) {
        self.temperature = self.temperature_sensor.read();
        self.power = self.power_sensor.as_mut().and_then(CpuPowerSensor::read);
        self.show_cpu_icons();
    }

    pub fn refresh_bluetooth(&mut self) {
        match scan_bluetooth_devices() {
            Ok(devices) => self.devices = devices,
            // Keep the previous list: a transient enumeration failure should not blank the tray.
            Err(error) => debug_log(&format!("Bluetooth scan failed: {error:#}")),
        }
        self.show_bluetooth_icons();
    }

    /// Theme or display settings changed: colours or icon size may differ now.
    pub fn redraw(&mut self) {
        self.theme = TaskbarTheme::current();
        self.show_cpu_icons();
        self.show_bluetooth_icons();
    }

    /// Explorer restarted and forgot every icon.
    pub fn on_taskbar_created(&self) {
        self.icons.register_all();
    }

    pub fn needs_elevation(&self) -> bool {
        self.temperature == TemperatureReading::NeedsElevation
    }

    /// Actions only: readings live in the icon tooltips, where they cannot be mistaken for
    /// clickable items.
    pub fn menu(&self) -> Vec<MenuEntry> {
        let mut entries = Vec::new();
        if let Some(autostart) = &self.autostart {
            // Read on every menu open: the user may also change it in Task Manager.
            let checked = autostart.is_enabled().unwrap_or_else(|error| {
                debug_log(&format!("cannot read autostart state: {error:#}"));
                false
            });
            entries.push(MenuEntry::Toggle {
                command: MenuCommand::StartWithWindows,
                checked,
            });
        }
        if self.needs_elevation() {
            entries.push(MenuEntry::Command(MenuCommand::RestartAsAdministrator));
        }
        if !entries.is_empty() {
            entries.push(MenuEntry::Separator);
        }
        entries.push(MenuEntry::Command(MenuCommand::Exit));
        entries
    }

    pub fn toggle_autostart(&self) {
        let Some(autostart) = &self.autostart else {
            return;
        };
        let enabled = match autostart.is_enabled() {
            Ok(enabled) => !enabled,
            Err(error) => {
                debug_log(&format!("cannot read autostart state: {error:#}"));
                return;
            }
        };
        let result = match autostart.set_enabled(enabled) {
            // The task runs elevated, so registering it needs administrator rights: let an
            // elevated copy do it. The menu shows the new state the next time it opens.
            Err(error) if is_access_denied(&error) => {
                let request = AutostartRequest {
                    enabled,
                    user: autostart.user().to_owned(),
                };
                run_self_elevated(&request.to_arguments())
            }
            result => result,
        };
        if let Err(error) = result {
            debug_log(&format!("cannot change autostart: {error:#}"));
        }
    }

    fn show_cpu_icons(&mut self) {
        let size = icon_size();
        if self.temperature == TemperatureReading::Unsupported {
            self.icons.hide(TrayIconId::CPU_TEMPERATURE);
        } else {
            let spec = temperature_icon(self.temperature, self.theme);
            self.show(
                TrayIconId::CPU_TEMPERATURE,
                spec,
                temperature_text(self.temperature),
                size,
            );
        }
        if self.power_sensor.is_some() {
            let spec = power_icon(self.power, self.theme);
            self.show(TrayIconId::CPU_POWER, spec, power_text(self.power), size);
        }
    }

    /// One icon per connected device that reports a battery level.
    fn show_bluetooth_icons(&mut self) {
        let size = icon_size();
        let mut shown = BTreeSet::new();
        for device in &self.devices {
            let (ConnectionState::Connected, Some(level)) = (device.connection, device.battery)
            else {
                continue;
            };
            let id = TrayIconId::bluetooth(device.address);
            shown.insert(id);
            let tooltip = format!("{}: {level}", device.name);
            if let Err(error) = self.icons.show(
                id,
                battery_icon(level, device.kind, self.theme),
                tooltip,
                size,
            ) {
                debug_log(&format!("cannot show icon for {}: {error:#}", device.name));
            }
        }
        self.icons
            .retain(|id| !id.is_bluetooth() || shown.contains(&id));
    }

    fn show(&mut self, id: TrayIconId, spec: IconSpec, tooltip: String, size: i32) {
        if let Err(error) = self.icons.show(id, spec, tooltip, size) {
            debug_log(&format!("cannot show icon {:#x}: {error:#}", id.value()));
        }
    }
}

/// Small icon size for the system DPI (16 px at 100%, 24 px at 150%).
fn icon_size() -> i32 {
    unsafe { GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForSystem()) }
}

fn temperature_text(reading: TemperatureReading) -> String {
    match reading {
        TemperatureReading::Value(celsius) => format!("CPU temperature: {celsius}"),
        TemperatureReading::NeedsElevation => {
            "CPU temperature: needs administrator rights (see menu)".to_owned()
        }
        TemperatureReading::Unavailable => "CPU temperature: no data".to_owned(),
        TemperatureReading::Unsupported => "CPU temperature: not supported on this PC".to_owned(),
    }
}

fn power_text(power: Option<Watts>) -> String {
    match power {
        Some(watts) => format!("CPU package power: {watts}"),
        None => "CPU package power: no data yet".to_owned(),
    }
}
