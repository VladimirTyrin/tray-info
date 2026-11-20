//! Decoding of UTF-16 text returned by Win32 APIs as raw little-endian bytes.

pub fn utf16_units(bytes: &[u8]) -> Vec<u16> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair))
        .collect()
}

/// A `REG_SZ`-style string: everything up to the first NUL (or the whole buffer).
pub fn string_from_le_bytes(bytes: &[u8]) -> String {
    let units = utf16_units(bytes);
    let end = units
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le_bytes(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    #[test]
    fn string_stops_at_first_nul() {
        assert_eq!(string_from_le_bytes(&le_bytes("Mouse\0junk")), "Mouse");
    }

    #[test]
    fn string_without_nul_takes_whole_buffer() {
        assert_eq!(string_from_le_bytes(&le_bytes("Наушники")), "Наушники");
    }
}
