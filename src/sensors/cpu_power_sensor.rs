use anyhow::Result;

use super::Watts;
use crate::win::PdhCounter;

/// Instances of the Windows "Energy Meter" counter set come from the EMI (Energy Metering
/// Interface) driver; on Intel it exposes RAPL domains as `RAPL_Package<N>_<DOMAIN>`
/// (letter case differs between PDH and `Get-Counter`, so matching ignores it).
/// The English path keeps this working on localized Windows.
const ENERGY_METER_POWER: &str = r"\Energy Meter(*)\Power";
const PACKAGE_SUFFIX: &str = "_pkg";

/// CPU package power read from Windows performance counters; works without admin rights.
pub struct CpuPowerSensor {
    counter: PdhCounter,
}

impl CpuPowerSensor {
    pub fn open() -> Result<Self> {
        Ok(Self {
            counter: PdhCounter::open(ENERGY_METER_POWER)?,
        })
    }

    pub fn read(&mut self) -> Option<Watts> {
        let samples = self.counter.sample_instances()?;
        package_power(samples.iter().map(|(name, value)| (name.as_str(), *value)))
    }
}

/// Sums the package domains (one per CPU socket). The counter reports milliwatts.
fn package_power<'a>(samples: impl Iterator<Item = (&'a str, f64)>) -> Option<Watts> {
    let mut total = None;
    for (name, milliwatts) in samples {
        let is_package = name
            .len()
            .checked_sub(PACKAGE_SUFFIX.len())
            .and_then(|start| name.get(start..))
            .is_some_and(|suffix| suffix.eq_ignore_ascii_case(PACKAGE_SUFFIX));
        if is_package {
            *total.get_or_insert(0.0) += milliwatts / 1000.0;
        }
    }
    total.map(Watts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_package_domain_and_converts_milliwatts() {
        let samples = [
            ("RAPL_Package0_DRAM", 0.0),
            ("RAPL_Package0_PKG", 19_261.5),
            ("RAPL_Package0_PP0", 9_162.4),
            ("_Total", 0.0),
        ];

        assert_eq!(package_power(samples.into_iter()), Some(Watts(19.2615)));
    }

    #[test]
    fn sums_packages_of_all_sockets() {
        let samples = [
            ("rapl_package0_pkg", 10_000.0),
            ("rapl_package1_pkg", 5_000.0),
        ];

        assert_eq!(package_power(samples.into_iter()), Some(Watts(15.0)));
    }

    #[test]
    fn no_package_domain_means_no_reading() {
        let samples = [("rapl_package0_pp0", 9_000.0)];

        assert_eq!(package_power(samples.into_iter()), None);
    }
}
