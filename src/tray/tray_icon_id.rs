use crate::sensors::BluetoothAddress;

/// The `uID` of a tray icon. Windows remembers per-icon settings (such as "show in taskbar")
/// by executable and `uID`, so ids must stay stable across restarts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrayIconId(u32);

/// High bit range reserved for Bluetooth devices so they never collide with fixed ids.
const BLUETOOTH_RANGE: u32 = 0x1000_0000;

impl TrayIconId {
    pub const CPU_TEMPERATURE: Self = Self(1);
    pub const CPU_POWER: Self = Self(2);

    /// Derived from the low 28 bits of the address: stable per device across restarts.
    pub fn bluetooth(address: BluetoothAddress) -> Self {
        Self(BLUETOOTH_RANGE | (address.value() as u32 & (BLUETOOTH_RANGE - 1)))
    }

    pub fn value(self) -> u32 {
        self.0
    }

    pub fn is_bluetooth(self) -> bool {
        self.0 & BLUETOOTH_RANGE != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bluetooth_ids_are_stable_and_distinct_from_fixed_ones() {
        let address = BluetoothAddress::parse_hex("DAE5E14BCB18").expect("valid address");
        let id = TrayIconId::bluetooth(address);

        assert_eq!(id, TrayIconId::bluetooth(address));
        assert!(id.is_bluetooth());
        assert!(!TrayIconId::CPU_TEMPERATURE.is_bluetooth());
        assert!(!TrayIconId::CPU_POWER.is_bluetooth());
    }
}
