use std::fmt;

/// 48-bit Bluetooth device address (MAC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BluetoothAddress(u64);

impl BluetoothAddress {
    /// Parses the 12 hex digit form used inside PnP instance IDs, e.g. `D74905FBFAB4`.
    /// The all-zero address marks local (radio-side) services and is rejected.
    pub fn parse_hex(text: &str) -> Option<Self> {
        if text.len() != 12 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let value = u64::from_str_radix(text, 16).ok()?;
        (value != 0).then_some(Self(value))
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

impl fmt::Display for BluetoothAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0.to_be_bytes();
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_twelve_hex_digits() {
        let address = BluetoothAddress::parse_hex("88C9E8F57D2E").expect("valid address");
        assert_eq!(address.to_string(), "88:C9:E8:F5:7D:2E");
    }

    #[test]
    fn rejects_wrong_length_non_hex_and_zero() {
        assert_eq!(BluetoothAddress::parse_hex("88C9E8F57D2"), None);
        assert_eq!(BluetoothAddress::parse_hex("88C9E8F57D2G"), None);
        assert_eq!(BluetoothAddress::parse_hex("000000000000"), None);
    }
}
