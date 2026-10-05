/// Defines how a menu item or a native menu item reacts to clicks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MenuItemToggleType {
    /// Normal menu item.
    #[default]
    None,

    /// Toggleable menu item with a checkbox.
    CheckBox,

    /// Menu item representing single option of radio group.
    Radio,
}
