use super::{BatteryLevel, BluetoothAddress, ConnectionState, DeviceKind};

/// A paired Bluetooth device with whatever battery information Windows has for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothDevice {
    pub address: BluetoothAddress,
    pub name: String,
    pub kind: DeviceKind,
    pub connection: ConnectionState,
    /// For disconnected devices this is the last value Windows cached, not a live reading.
    pub battery: Option<BatteryLevel>,
}
