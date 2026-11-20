use super::MenuCommand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    Separator,
    Command(MenuCommand),
    /// A command shown with a check mark when its setting is on.
    Toggle {
        command: MenuCommand,
        checked: bool,
    },
}
