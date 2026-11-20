//! Maps sensor readings to icon specs: what to draw and in which colours.

use super::{IconSpec, Rgb, Symbol, TaskbarTheme};
use crate::sensors::{BatteryLevel, DeviceKind, TemperatureReading, Watts};

const PLACEHOLDER: &str = "--";

const WARM_CELSIUS: f64 = 75.0;
const HOT_CELSIUS: f64 = 90.0;
const LOW_BATTERY_PERCENT: u8 = 20;
const MEDIUM_BATTERY_PERCENT: u8 = 40;

struct Colors {
    foreground: Rgb,
    muted: Rgb,
    track: Rgb,
    power: Rgb,
    warning: Rgb,
    critical: Rgb,
}

fn colors(theme: TaskbarTheme) -> Colors {
    match theme {
        TaskbarTheme::Dark => Colors {
            foreground: Rgb::hex(0xFFFFFF),
            muted: Rgb::hex(0x8A8A8A),
            track: Rgb::hex(0x505A66),
            power: Rgb::hex(0x7CD2FF),
            warning: Rgb::hex(0xFFC83D),
            critical: Rgb::hex(0xFF5F5F),
        },
        TaskbarTheme::Light => Colors {
            foreground: Rgb::hex(0x1A1A1A),
            muted: Rgb::hex(0x7A7A7A),
            track: Rgb::hex(0xC4C8CC),
            power: Rgb::hex(0x005FB8),
            warning: Rgb::hex(0x9D5D00),
            critical: Rgb::hex(0xC42B1C),
        },
    }
}

pub fn temperature_icon(reading: TemperatureReading, theme: TaskbarTheme) -> IconSpec {
    let colors = colors(theme);
    match reading {
        TemperatureReading::Value(celsius) => {
            let color = if celsius.0 >= HOT_CELSIUS {
                colors.critical
            } else if celsius.0 >= WARM_CELSIUS {
                colors.warning
            } else {
                colors.foreground
            };
            reading_icon(Symbol::Thermometer, format!("{:.0}", celsius.0), color)
        }
        TemperatureReading::NeedsElevation
        | TemperatureReading::Unavailable
        | TemperatureReading::Unsupported => placeholder(Symbol::Thermometer, &colors),
    }
}

pub fn power_icon(power: Option<Watts>, theme: TaskbarTheme) -> IconSpec {
    let colors = colors(theme);
    match power {
        Some(watts) => reading_icon(Symbol::Lightning, format!("{:.0}", watts.0), colors.power),
        None => placeholder(Symbol::Lightning, &colors),
    }
}

/// The device symbol stays neutral; the ring shows the level and turns amber, then red, when
/// the battery runs low.
pub fn battery_icon(level: BatteryLevel, kind: DeviceKind, theme: TaskbarTheme) -> IconSpec {
    let colors = colors(theme);
    let fill = if level.percent() <= LOW_BATTERY_PERCENT {
        colors.critical
    } else if level.percent() <= MEDIUM_BATTERY_PERCENT {
        colors.warning
    } else {
        colors.foreground
    };
    IconSpec::Ring {
        symbol: device_symbol(kind),
        symbol_color: colors.foreground,
        fraction: f32::from(level.percent()) / 100.0,
        fill,
        track: colors.track,
    }
}

fn device_symbol(kind: DeviceKind) -> Symbol {
    match kind {
        DeviceKind::Headphones => Symbol::Headphones,
        DeviceKind::Mouse => Symbol::Mouse,
        DeviceKind::Keyboard => Symbol::Keyboard,
        DeviceKind::Gamepad => Symbol::Gamepad,
        DeviceKind::Other => Symbol::Bluetooth,
    }
}

fn reading_icon(symbol: Symbol, text: String, color: Rgb) -> IconSpec {
    IconSpec::Reading {
        symbol,
        text,
        color,
    }
}

fn placeholder(symbol: Symbol, colors: &Colors) -> IconSpec {
    reading_icon(symbol, PLACEHOLDER.to_owned(), colors.muted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensors::Celsius;

    fn battery(percent: u8) -> BatteryLevel {
        BatteryLevel::new(percent).expect("valid test level")
    }

    fn number_of(spec: IconSpec) -> (String, Rgb) {
        match spec {
            IconSpec::Reading { text, color, .. } => (text, color),
            IconSpec::Ring { .. } => panic!("expected a reading icon, got {spec:?}"),
        }
    }

    #[test]
    fn temperature_shows_rounded_degrees_with_thermometer() {
        let theme = TaskbarTheme::Dark;
        let spec = temperature_icon(TemperatureReading::Value(Celsius(71.6)), theme);

        assert_eq!(
            spec,
            IconSpec::Reading {
                symbol: Symbol::Thermometer,
                text: "72".to_owned(),
                color: colors(theme).foreground,
            }
        );
    }

    #[test]
    fn hot_cpu_is_highlighted_as_critical() {
        let theme = TaskbarTheme::Dark;
        let spec = temperature_icon(TemperatureReading::Value(Celsius(95.0)), theme);

        assert_eq!(number_of(spec).1, colors(theme).critical);
    }

    #[test]
    fn missing_temperature_shows_muted_placeholder() {
        let theme = TaskbarTheme::Light;
        let spec = temperature_icon(TemperatureReading::NeedsElevation, theme);

        assert_eq!(
            number_of(spec),
            (PLACEHOLDER.to_owned(), colors(theme).muted)
        );
    }

    #[test]
    fn power_shows_whole_watts_with_lightning() {
        let theme = TaskbarTheme::Dark;
        let spec = power_icon(Some(Watts(19.26)), theme);

        assert_eq!(
            spec,
            IconSpec::Reading {
                symbol: Symbol::Lightning,
                text: "19".to_owned(),
                color: colors(theme).power,
            }
        );
    }

    #[test]
    fn battery_ring_shows_device_and_level() {
        let theme = TaskbarTheme::Dark;
        let spec = battery_icon(battery(95), DeviceKind::Mouse, theme);

        assert_eq!(
            spec,
            IconSpec::Ring {
                symbol: Symbol::Mouse,
                symbol_color: colors(theme).foreground,
                fraction: 0.95,
                fill: colors(theme).foreground,
                track: colors(theme).track,
            }
        );
    }

    #[test]
    fn unknown_device_shows_bluetooth_symbol() {
        let spec = battery_icon(battery(50), DeviceKind::Other, TaskbarTheme::Dark);

        assert!(matches!(
            spec,
            IconSpec::Ring {
                symbol: Symbol::Bluetooth,
                ..
            }
        ));
    }

    #[test]
    fn ring_colour_follows_level() {
        let theme = TaskbarTheme::Dark;
        let cases = [
            (100, colors(theme).foreground),
            (41, colors(theme).foreground),
            (40, colors(theme).warning),
            (21, colors(theme).warning),
            (20, colors(theme).critical),
            (0, colors(theme).critical),
        ];
        for (percent, expected) in cases {
            let spec = battery_icon(battery(percent), DeviceKind::Keyboard, theme);
            let IconSpec::Ring { fill, .. } = spec else {
                panic!("expected a ring icon");
            };
            assert_eq!(fill, expected, "{percent}%");
        }
    }
}
