use std::fmt;

/// Battery charge in percent, guaranteed to be within `0..=100`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BatteryLevel(u8);

impl BatteryLevel {
    pub fn new(percent: u8) -> Option<Self> {
        (percent <= 100).then_some(Self(percent))
    }

    pub fn percent(self) -> u8 {
        self.0
    }
}

impl fmt::Display for BatteryLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_percentages_up_to_one_hundred() {
        assert_eq!(BatteryLevel::new(100).map(BatteryLevel::percent), Some(100));
        assert_eq!(BatteryLevel::new(0).map(BatteryLevel::percent), Some(0));
    }

    #[test]
    fn rejects_values_above_one_hundred() {
        assert_eq!(BatteryLevel::new(101), None);
        assert_eq!(BatteryLevel::new(255), None);
    }
}
