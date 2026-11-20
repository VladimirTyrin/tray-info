use super::Celsius;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TemperatureReading {
    Value(Celsius),
    /// The sensor exists but Windows only lets administrators read it.
    NeedsElevation,
    /// The sensor exists but did not return a plausible value this time.
    Unavailable,
    /// No supported temperature source on this machine.
    Unsupported,
}
