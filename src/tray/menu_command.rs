/// Actions available from the tray context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    StartWithWindows,
    RestartAsAdministrator,
    Exit,
}

impl MenuCommand {
    const ALL: [Self; 3] = [
        Self::StartWithWindows,
        Self::RestartAsAdministrator,
        Self::Exit,
    ];

    /// Menu item id; 0 is reserved by Windows for "nothing selected".
    pub fn id(self) -> u32 {
        match self {
            Self::RestartAsAdministrator => 1,
            Self::Exit => 2,
            Self::StartWithWindows => 3,
        }
    }

    pub fn from_id(id: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::RestartAsAdministrator => "Restart as administrator (CPU temperature)",
            Self::Exit => "Exit",
            Self::StartWithWindows => "Start with Windows",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for command in MenuCommand::ALL {
            assert_eq!(MenuCommand::from_id(command.id()), Some(command));
        }
        assert_eq!(MenuCommand::from_id(0), None);
    }
}
