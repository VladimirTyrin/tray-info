/// How long a starting instance waits for a previous one to exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceWait {
    /// Normal start: if another instance runs, quit immediately.
    NoWait,
    /// Relaunch (e.g. as administrator): the previous instance is about to exit.
    ForPrevious,
}
