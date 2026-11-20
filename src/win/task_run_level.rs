/// Rights a Task Scheduler task starts its program with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskRunLevel {
    /// Elevated for administrators, without a UAC prompt; registering it needs admin rights.
    Highest,
    /// The user's normal rights; any user may register such a task for themselves.
    #[cfg_attr(not(test), expect(dead_code, reason = "only tests run unelevated"))]
    Least,
}

impl TaskRunLevel {
    /// Value of the task XML `RunLevel` element.
    pub fn xml_value(self) -> &'static str {
        match self {
            Self::Highest => "HighestAvailable",
            Self::Least => "LeastPrivilege",
        }
    }
}
