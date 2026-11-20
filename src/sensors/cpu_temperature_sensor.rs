use super::{Celsius, TemperatureReading};
use crate::win::{WmiArgument, WmiFailure, WmiMethod, debug_log};

/// Feature id of the CPU temperature for `LENOVO_OTHER_METHOD.GetFeatureValue` (as used by
/// Lenovo Vantage and Legion Toolkit on 2022+ Legion models).
const CPU_TEMPERATURE_FEATURE: u32 = 0x0504_0000;

/// Readings outside this range are treated as glitches of the firmware call.
const PLAUSIBLE_CELSIUS: std::ops::RangeInclusive<i32> = 1..=150;

/// CPU temperature source.
///
/// Windows has no unprivileged generic CPU temperature API: ACPI thermal zones are often absent
/// (as on Lenovo Legion) and MSR access needs a kernel driver. Lenovo gaming laptops expose the
/// EC reading through WMI methods callable by admins only: newer models through
/// `LENOVO_OTHER_METHOD.GetFeatureValue`, older ones through `LENOVO_GAMEZONE_DATA.GetCPUTemp`
/// (which newer models still have but answer with 0).
pub enum CpuTemperatureSensor {
    LenovoWmi(WmiMethod),
    NeedsElevation,
    Unsupported,
}

impl CpuTemperatureSensor {
    pub fn open() -> Self {
        let feature = WmiArgument {
            name: "IDs",
            value: CPU_TEMPERATURE_FEATURE,
        };
        let method = match WmiMethod::bind(
            r"ROOT\WMI",
            "LENOVO_OTHER_METHOD",
            "GetFeatureValue",
            Some(feature),
            "value",
        ) {
            Err(WmiFailure::Other(error)) => {
                debug_log(&format!("LENOVO_OTHER_METHOD unavailable: {error:#}"));
                WmiMethod::bind(
                    r"ROOT\WMI",
                    "LENOVO_GAMEZONE_DATA",
                    "GetCPUTemp",
                    None,
                    "Data",
                )
            }
            result => result,
        };
        match method {
            Ok(method) => Self::LenovoWmi(method),
            Err(WmiFailure::AccessDenied) => Self::NeedsElevation,
            Err(WmiFailure::Other(error)) => {
                debug_log(&format!("CPU temperature unsupported: {error:#}"));
                Self::Unsupported
            }
        }
    }

    pub fn read(&self) -> TemperatureReading {
        match self {
            Self::LenovoWmi(method) => match method.call_i32() {
                Ok(value) if PLAUSIBLE_CELSIUS.contains(&value) => {
                    TemperatureReading::Value(Celsius(f64::from(value)))
                }
                Ok(value) => {
                    debug_log(&format!("CPU temperature implausible: {value}"));
                    TemperatureReading::Unavailable
                }
                Err(WmiFailure::AccessDenied) => TemperatureReading::NeedsElevation,
                Err(WmiFailure::Other(error)) => {
                    debug_log(&format!("CPU temperature read failed: {error:#}"));
                    TemperatureReading::Unavailable
                }
            },
            Self::NeedsElevation => TemperatureReading::NeedsElevation,
            Self::Unsupported => TemperatureReading::Unsupported,
        }
    }
}
