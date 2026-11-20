/// Splits a Win32 `MULTI_SZ` buffer (strings separated by NUL, terminated by an empty string).
pub fn parse_multi_sz(buffer: &[u16]) -> Vec<String> {
    buffer
        .split(|&unit| unit == 0)
        .take_while(|item| !item.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn splits_until_empty_string() {
        let buffer = wide("BTHLE\\DEV_1\0BTHLE\\DEV_2\0\0garbage\0");

        assert_eq!(parse_multi_sz(&buffer), ["BTHLE\\DEV_1", "BTHLE\\DEV_2"]);
    }

    #[test]
    fn empty_list() {
        assert!(parse_multi_sz(&[0, 0]).is_empty());
        assert!(parse_multi_sz(&[]).is_empty());
    }
}
