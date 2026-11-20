use super::BluetoothAddress;

/// Role of a PnP device node in the Bluetooth stack, recognised from its instance ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluetoothNode {
    /// The device itself: `BTHLE\DEV_<addr>\...` or `BTHENUM\DEV_<addr>\...`.
    /// Carries the user-visible name, connection state and (for BLE) the battery level.
    Device(BluetoothAddress),
    /// A classic profile service such as Hands-Free:
    /// `BTHENUM\{uuid}_VID&...\7&2768A9F8&0&<addr>_C00000000`.
    /// Windows reports classic headset battery (HFP AT commands) on this node, not on the device.
    Profile(BluetoothAddress),
}

impl BluetoothNode {
    pub fn parse(instance_id: &str) -> Option<Self> {
        let mut parts = instance_id.split('\\');
        let enumerator = parts.next()?;
        let hardware = parts.next()?;
        let instance = parts.next()?;

        if let Some(address) = strip_prefix_ignore_case(hardware, "DEV_") {
            return BluetoothAddress::parse_hex(address).map(Self::Device);
        }
        if !enumerator.eq_ignore_ascii_case("BTHENUM") {
            return None;
        }
        let last_segment = instance.rsplit('&').next()?;
        let address = last_segment.split('_').next()?;
        BluetoothAddress::parse_hex(address).map(Self::Profile)
    }

    pub fn address(self) -> BluetoothAddress {
        match self {
            Self::Device(address) | Self::Profile(address) => address,
        }
    }
}

fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(hex: &str) -> BluetoothAddress {
        BluetoothAddress::parse_hex(hex).expect("valid test address")
    }

    #[test]
    fn recognises_ble_device_node() {
        let node = BluetoothNode::parse(r"BTHLE\DEV_DAE5E14BCB18\7&2832959&0&DAE5E14BCB18");
        assert_eq!(node, Some(BluetoothNode::Device(address("DAE5E14BCB18"))));
    }

    #[test]
    fn recognises_classic_device_node() {
        let node = BluetoothNode::parse(
            r"BTHENUM\DEV_88C9E8F57D2E\7&2768A9F8&0&BLUETOOTHDEVICE_88C9E8F57D2E",
        );
        assert_eq!(node, Some(BluetoothNode::Device(address("88C9E8F57D2E"))));
    }

    #[test]
    fn recognises_classic_profile_node() {
        let node = BluetoothNode::parse(
            r"BTHENUM\{0000111E-0000-1000-8000-00805F9B34FB}_VID&0002054C_PID&0DF0\7&2768A9F8&0&88C9E8F57D2E_C00000000",
        );
        assert_eq!(node, Some(BluetoothNode::Profile(address("88C9E8F57D2E"))));
    }

    #[test]
    fn ignores_local_services_and_enumerators() {
        assert_eq!(
            BluetoothNode::parse(
                r"BTHENUM\{00001200-0000-1000-8000-00805F9B34FB}_LOCALMFG&0002\7&2768A9F8&0&000000000000_00000000"
            ),
            None
        );
        assert_eq!(BluetoothNode::parse(r"BTH\MS_BTHLE\6&2372791F&0&3"), None);
        assert_eq!(
            BluetoothNode::parse(
                r"BTHLEDEVICE\{0000180F-0000-1000-8000-00805F9B34FB}_DEV_VID&02046D_PID&B034\8&10EFFCF3&0&001B"
            ),
            None
        );
    }
}
