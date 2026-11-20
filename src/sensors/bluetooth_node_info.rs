use super::bluetooth_node::BluetoothNode;
use super::{BatteryLevel, ConnectionState, DeviceKind};

/// Properties read from one Bluetooth PnP node before nodes are merged into devices.
#[derive(Debug, Clone)]
pub struct BluetoothNodeInfo {
    pub node: BluetoothNode,
    pub name: Option<String>,
    /// Known only for device nodes; profile nodes carry no category.
    pub kind: Option<DeviceKind>,
    pub connection: Option<ConnectionState>,
    pub battery: Option<BatteryLevel>,
}
