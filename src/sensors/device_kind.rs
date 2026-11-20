/// What a Bluetooth device is, as far as the tray icon cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Headphones,
    Mouse,
    Keyboard,
    Gamepad,
    Other,
}

impl DeviceKind {
    /// Classifies by `DEVPKEY_DeviceContainer_Category` values such as `Input.Mouse` or
    /// `Communication.Headset.Bluetooth`; the first recognised category wins.
    pub fn from_categories<S: AsRef<str>>(categories: &[S]) -> Self {
        categories
            .iter()
            .find_map(|category| Self::from_category(category.as_ref()))
            .unwrap_or(Self::Other)
    }

    fn from_category(category: &str) -> Option<Self> {
        let category = category.to_ascii_lowercase();
        let is = |prefix: &str| category.starts_with(prefix);
        if is("input.mouse") || is("input.touchpad") {
            Some(Self::Mouse)
        } else if is("input.keyboard") {
            Some(Self::Keyboard)
        } else if is("input.gaming") {
            Some(Self::Gamepad)
        } else if is("audio.headphone") || is("audio.headset") || is("communication.headset") {
            Some(Self::Headphones)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_windows_categories() {
        let cases = [
            ("Input.Mouse", DeviceKind::Mouse),
            ("Input.Keyboard", DeviceKind::Keyboard),
            ("Input.Gaming", DeviceKind::Gamepad),
            ("Communication.Headset.Bluetooth", DeviceKind::Headphones),
            ("Audio.Headphone", DeviceKind::Headphones),
            ("Other.Thing", DeviceKind::Other),
        ];
        for (category, expected) in cases {
            assert_eq!(
                DeviceKind::from_categories(&[category]),
                expected,
                "{category}"
            );
        }
    }

    #[test]
    fn first_recognised_category_wins() {
        let kind = DeviceKind::from_categories(&["Other.Thing", "Input.Keyboard", "Input.Mouse"]);

        assert_eq!(kind, DeviceKind::Keyboard);
    }

    #[test]
    fn no_categories_means_other() {
        assert_eq!(DeviceKind::from_categories::<&str>(&[]), DeviceKind::Other);
    }
}
