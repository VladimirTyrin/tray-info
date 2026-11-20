use std::collections::BTreeMap;

use anyhow::Result;
use windows::Win32::Foundation::DEVPROPKEY;
use windows::core::GUID;

use super::bluetooth_node::BluetoothNode;
use super::bluetooth_node_info::BluetoothNodeInfo;
use super::{BatteryLevel, BluetoothAddress, BluetoothDevice, ConnectionState, DeviceKind};
use crate::win::{DevNode, present_device_ids};

/// `DEVPKEY_Bluetooth_Battery`: battery percent maintained by the Windows Bluetooth stack
/// (BLE Battery Service for LE devices, HFP indicators for classic headsets).
const BATTERY_KEY: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0x104ea319_6ee2_4701_bd47_8ddbf425bbe5),
    pid: 2,
};

/// Device "connected" flag shown by Settings → Bluetooth & devices.
const CONNECTED_KEY: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0x83da6326_97a6_4088_9453_a1923f573b29),
    pid: 15,
};

/// `DEVPKEY_Device_FriendlyName`: the name the user sees (and may have renamed) in Settings.
const FRIENDLY_NAME_KEY: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 14,
};

/// `DEVPKEY_Device_DeviceDesc`: driver-provided description, fallback when no friendly name.
/// (`DEVPKEY_NAME` would be simpler but is computed by device query, not stored on the node.)
const DEVICE_DESCRIPTION_KEY: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 2,
};

/// `DEVPKEY_DeviceContainer_Category`: Windows' classification of the device (`Input.Mouse`, …),
/// the same one Settings uses to pick the device picture.
const CATEGORY_KEY: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0x78c34fc8_104a_4aca_9ea4_524d52996e57),
    pid: 90,
};

/// Enumerators whose nodes describe paired devices: BLE devices and classic devices with profiles.
const ENUMERATORS: [&str; 2] = ["BTHLE", "BTHENUM"];

/// Reads all paired Bluetooth devices through the configuration manager.
///
/// This only reads properties Windows already keeps up to date, so it never wakes devices up
/// or opens GATT connections and costs a few cheap API calls per node.
pub fn scan_bluetooth_devices() -> Result<Vec<BluetoothDevice>> {
    let mut nodes = Vec::new();
    for enumerator in ENUMERATORS {
        for instance_id in present_device_ids(enumerator)? {
            let Some(node) = BluetoothNode::parse(&instance_id) else {
                continue;
            };
            // A node can disappear between listing and locating it; it will be seen on the next scan.
            let Some(dev_node) = DevNode::locate(&instance_id) else {
                continue;
            };
            nodes.push(read_node(node, &dev_node));
        }
    }
    Ok(merge_nodes(nodes))
}

fn read_node(node: BluetoothNode, dev_node: &DevNode) -> BluetoothNodeInfo {
    let battery = dev_node.byte(&BATTERY_KEY).and_then(BatteryLevel::new);
    match node {
        BluetoothNode::Device(_) => BluetoothNodeInfo {
            node,
            name: dev_node
                .string(&FRIENDLY_NAME_KEY)
                .or_else(|| dev_node.string(&DEVICE_DESCRIPTION_KEY)),
            kind: Some(DeviceKind::from_categories(
                &dev_node.string_list(&CATEGORY_KEY).unwrap_or_default(),
            )),
            connection: dev_node.boolean(&CONNECTED_KEY).map(|connected| {
                if connected {
                    ConnectionState::Connected
                } else {
                    ConnectionState::Disconnected
                }
            }),
            battery,
        },
        BluetoothNode::Profile(_) => BluetoothNodeInfo {
            node,
            name: None,
            kind: None,
            connection: None,
            battery,
        },
    }
}

/// Folds device and profile nodes into one entry per address.
/// Nodes without a device node are dropped: they belong to devices that are no longer paired.
fn merge_nodes(nodes: Vec<BluetoothNodeInfo>) -> Vec<BluetoothDevice> {
    let mut devices: BTreeMap<BluetoothAddress, BluetoothDevice> = BTreeMap::new();
    let mut profile_batteries: BTreeMap<BluetoothAddress, BatteryLevel> = BTreeMap::new();

    for info in nodes {
        let address = info.node.address();
        match info.node {
            BluetoothNode::Device(_) => {
                devices.insert(
                    address,
                    BluetoothDevice {
                        address,
                        name: info.name.unwrap_or_else(|| address.to_string()),
                        kind: info.kind.unwrap_or(DeviceKind::Other),
                        connection: info.connection.unwrap_or(ConnectionState::Disconnected),
                        battery: info.battery,
                    },
                );
            }
            BluetoothNode::Profile(_) => {
                if let Some(level) = info.battery {
                    profile_batteries
                        .entry(address)
                        .and_modify(|known| *known = (*known).max(level))
                        .or_insert(level);
                }
            }
        }
    }

    for (address, level) in profile_batteries {
        if let Some(device) = devices.get_mut(&address) {
            device.battery = device.battery.or(Some(level));
        }
    }

    let mut result: Vec<BluetoothDevice> = devices.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name).then(a.address.cmp(&b.address)));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(hex: &str) -> BluetoothAddress {
        BluetoothAddress::parse_hex(hex).expect("valid test address")
    }

    fn device_node(
        hex: &str,
        name: &str,
        connection: ConnectionState,
        battery: Option<u8>,
    ) -> BluetoothNodeInfo {
        BluetoothNodeInfo {
            node: BluetoothNode::Device(address(hex)),
            name: Some(name.to_owned()),
            kind: Some(DeviceKind::Other),
            connection: Some(connection),
            battery: battery.and_then(BatteryLevel::new),
        }
    }

    fn profile_node(hex: &str, battery: Option<u8>) -> BluetoothNodeInfo {
        BluetoothNodeInfo {
            node: BluetoothNode::Profile(address(hex)),
            name: None,
            kind: None,
            connection: None,
            battery: battery.and_then(BatteryLevel::new),
        }
    }

    #[test]
    fn classic_headset_takes_battery_from_hands_free_profile() {
        let devices = merge_nodes(vec![
            profile_node("88C9E8F57D2E", None),
            profile_node("88C9E8F57D2E", Some(100)),
            BluetoothNodeInfo {
                kind: Some(DeviceKind::Headphones),
                ..device_node(
                    "88C9E8F57D2E",
                    "WH-1000XM5",
                    ConnectionState::Connected,
                    None,
                )
            },
        ]);

        assert_eq!(
            devices,
            vec![BluetoothDevice {
                address: address("88C9E8F57D2E"),
                name: "WH-1000XM5".to_owned(),
                kind: DeviceKind::Headphones,
                connection: ConnectionState::Connected,
                battery: BatteryLevel::new(100),
            }]
        );
    }

    #[test]
    fn ble_device_keeps_its_own_battery() {
        let devices = merge_nodes(vec![device_node(
            "DAE5E14BCB18",
            "MX Master 4",
            ConnectionState::Connected,
            Some(95),
        )]);

        assert_eq!(devices[0].battery, BatteryLevel::new(95));
    }

    #[test]
    fn profile_without_paired_device_is_dropped() {
        let devices = merge_nodes(vec![profile_node("143FA6291C3F", Some(40))]);

        assert!(devices.is_empty());
    }

    #[test]
    fn devices_are_sorted_by_name() {
        let devices = merge_nodes(vec![
            device_node(
                "DAE5E14BCB18",
                "MX Master 4",
                ConnectionState::Connected,
                None,
            ),
            device_node(
                "D74905FBFAB4",
                "MX KEYS S",
                ConnectionState::Connected,
                None,
            ),
        ]);

        let names: Vec<&str> = devices.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["MX KEYS S", "MX Master 4"]);
    }
}
