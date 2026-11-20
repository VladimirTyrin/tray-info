/// A `uint32` input parameter of a WMI method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WmiArgument {
    pub name: &'static str,
    pub value: u32,
}
