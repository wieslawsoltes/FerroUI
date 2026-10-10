//! The port of `AtSpiNode.RoleMapping.cs`.

use super::at_spi_node::AtSpiNode;
use super::at_spi_role::AtSpiRole;
use ferroui_controls::automation::peers::{AutomationControlType, AutomationPeer};
use ferroui_controls::automation::provider::IToggleProvider;

impl AtSpiNode {
    pub(crate) fn to_at_spi_role(control_type: AutomationControlType, peer: Option<&AutomationPeer>) -> AtSpiRole {
        Self::to_at_spi_role_with(control_type, || {
            peer.is_some_and(|peer| peer.get_provider::<dyn IToggleProvider>().is_some())
        })
    }

    /// The mapping, with whether the peer can be toggled asked only for a button.
    pub(crate) fn to_at_spi_role_with(
        control_type: AutomationControlType,
        has_toggle_provider: impl FnOnce() -> bool,
    ) -> AtSpiRole {
        match control_type {
            AutomationControlType::None => AtSpiRole::Panel,
            AutomationControlType::Button => {
                if has_toggle_provider() {
                    AtSpiRole::ToggleButton
                } else {
                    AtSpiRole::PushButton
                }
            }
            AutomationControlType::Calendar => AtSpiRole::Calendar,
            AutomationControlType::CheckBox => AtSpiRole::CheckBox,
            AutomationControlType::ComboBox => AtSpiRole::ComboBox,
            AutomationControlType::ComboBoxItem => AtSpiRole::ListItem,
            AutomationControlType::Edit => AtSpiRole::Entry,
            AutomationControlType::Hyperlink => AtSpiRole::Link,
            AutomationControlType::Image => AtSpiRole::Image,
            AutomationControlType::ListItem => AtSpiRole::ListItem,
            AutomationControlType::List => AtSpiRole::List,
            AutomationControlType::Menu => AtSpiRole::Menu,
            AutomationControlType::MenuBar => AtSpiRole::MenuBar,
            AutomationControlType::MenuItem => AtSpiRole::MenuItem,
            AutomationControlType::ProgressBar => AtSpiRole::ProgressBar,
            AutomationControlType::RadioButton => AtSpiRole::RadioButton,
            AutomationControlType::ScrollBar => AtSpiRole::ScrollBar,
            AutomationControlType::Slider => AtSpiRole::Slider,
            AutomationControlType::Spinner => AtSpiRole::SpinButton,
            AutomationControlType::StatusBar => AtSpiRole::StatusBar,
            AutomationControlType::Tab => AtSpiRole::PageTabList,
            AutomationControlType::TabItem => AtSpiRole::PageTab,
            AutomationControlType::Text => AtSpiRole::Label,
            AutomationControlType::ToolBar => AtSpiRole::ToolBar,
            AutomationControlType::ToolTip => AtSpiRole::ToolTip,
            AutomationControlType::Tree => AtSpiRole::Tree,
            AutomationControlType::TreeItem => AtSpiRole::TreeItem,
            AutomationControlType::Custom => AtSpiRole::Unknown,
            AutomationControlType::Group => AtSpiRole::Panel,
            AutomationControlType::Thumb => AtSpiRole::PushButton,
            AutomationControlType::DataGrid => AtSpiRole::TreeTable,
            AutomationControlType::DataItem => AtSpiRole::TableCell,
            AutomationControlType::Document => AtSpiRole::Document,
            AutomationControlType::SplitButton => AtSpiRole::PushButton,
            AutomationControlType::Window => AtSpiRole::Frame,
            AutomationControlType::Pane => AtSpiRole::Panel,
            AutomationControlType::Header => AtSpiRole::Header,
            AutomationControlType::HeaderItem => AtSpiRole::ColumnHeader,
            AutomationControlType::Table => AtSpiRole::Table,
            AutomationControlType::TitleBar => AtSpiRole::TitleBar,
            AutomationControlType::Separator => AtSpiRole::Separator,
            AutomationControlType::Expander => AtSpiRole::Panel,
            AutomationControlType::ScrollViewer => AtSpiRole::ScrollPane,
        }
    }

    pub(crate) fn to_at_spi_role_name(role: AtSpiRole) -> &'static str {
        match role {
            AtSpiRole::Application => "application",
            AtSpiRole::Frame => "frame",
            AtSpiRole::PushButton => "push button",
            AtSpiRole::ToggleButton => "toggle button",
            AtSpiRole::CheckBox => "check box",
            AtSpiRole::ComboBox => "combo box",
            AtSpiRole::Entry => "entry",
            AtSpiRole::Label => "label",
            AtSpiRole::Image => "image",
            AtSpiRole::List => "list",
            AtSpiRole::ListItem => "list item",
            AtSpiRole::Menu => "menu",
            AtSpiRole::MenuBar => "menu bar",
            AtSpiRole::MenuItem => "menu item",
            AtSpiRole::ProgressBar => "progress bar",
            AtSpiRole::RadioButton => "radio button",
            AtSpiRole::ScrollBar => "scroll bar",
            AtSpiRole::ScrollPane => "scroll pane",
            AtSpiRole::Slider => "slider",
            AtSpiRole::SpinButton => "spin button",
            AtSpiRole::StatusBar => "status bar",
            AtSpiRole::PageTab => "page tab",
            AtSpiRole::PageTabList => "page tab list",
            AtSpiRole::ToolBar => "tool bar",
            AtSpiRole::ToolTip => "tool tip",
            AtSpiRole::Tree => "tree",
            AtSpiRole::TreeItem => "tree item",
            AtSpiRole::Panel => "panel",
            AtSpiRole::Separator => "separator",
            AtSpiRole::Table => "table",
            AtSpiRole::TableCell => "table cell",
            AtSpiRole::TreeTable => "tree table",
            AtSpiRole::ColumnHeader => "column header",
            AtSpiRole::Header => "header",
            AtSpiRole::TitleBar => "title bar",
            AtSpiRole::Document => "document frame",
            AtSpiRole::Link => "link",
            AtSpiRole::Calendar => "calendar",
            AtSpiRole::Window => "window",
            AtSpiRole::Unknown => "unknown",
            _ => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this project.
    use super::*;

    fn role(control_type: AutomationControlType) -> AtSpiRole {
        AtSpiNode::to_at_spi_role_with(control_type, || false)
    }

    #[test]
    fn control_types_map_to_roles() {
        assert_eq!(role(AutomationControlType::None), AtSpiRole::Panel);
        assert_eq!(role(AutomationControlType::Button), AtSpiRole::PushButton);
        assert_eq!(AtSpiNode::to_at_spi_role_with(AutomationControlType::Button, || true), AtSpiRole::ToggleButton);
        // Only a button asks whether it can be toggled: a check box is a check box.
        assert_eq!(AtSpiNode::to_at_spi_role_with(AutomationControlType::CheckBox, || true), AtSpiRole::CheckBox);
        assert_eq!(role(AutomationControlType::Edit), AtSpiRole::Entry);
        assert_eq!(role(AutomationControlType::Text), AtSpiRole::Label);
        assert_eq!(role(AutomationControlType::Window), AtSpiRole::Frame);
        assert_eq!(role(AutomationControlType::Tab), AtSpiRole::PageTabList);
        assert_eq!(role(AutomationControlType::TabItem), AtSpiRole::PageTab);
        assert_eq!(role(AutomationControlType::ComboBoxItem), AtSpiRole::ListItem);
        assert_eq!(role(AutomationControlType::DataGrid), AtSpiRole::TreeTable);
        assert_eq!(role(AutomationControlType::DataItem), AtSpiRole::TableCell);
        assert_eq!(role(AutomationControlType::HeaderItem), AtSpiRole::ColumnHeader);
        assert_eq!(role(AutomationControlType::Custom), AtSpiRole::Unknown);
        assert_eq!(role(AutomationControlType::Expander), AtSpiRole::Panel);
        assert_eq!(role(AutomationControlType::ScrollViewer), AtSpiRole::ScrollPane);
        assert_eq!(role(AutomationControlType::Thumb), AtSpiRole::PushButton);
        assert_eq!(role(AutomationControlType::SplitButton), AtSpiRole::PushButton);
    }

    #[test]
    fn roles_have_the_numbers_of_the_specification() {
        assert_eq!(AtSpiRole::Frame as u32, 23);
        assert_eq!(AtSpiRole::PushButton as u32, 43);
        assert_eq!(AtSpiRole::CheckBox as u32, 7);
        assert_eq!(AtSpiRole::Entry as u32, 79);
        assert_eq!(AtSpiRole::Application as u32, 75);
        assert_eq!(AtSpiRole::Label as u32, 29);
        assert_eq!(AtSpiRole::Static as u32, 116);
    }

    #[test]
    fn roles_have_names() {
        assert_eq!(AtSpiNode::to_at_spi_role_name(AtSpiRole::PushButton), "push button");
        assert_eq!(AtSpiNode::to_at_spi_role_name(AtSpiRole::Document), "document frame");
        assert_eq!(AtSpiNode::to_at_spi_role_name(AtSpiRole::Entry), "entry");
        // A role the reference gives no name.
        assert_eq!(AtSpiNode::to_at_spi_role_name(AtSpiRole::Dialog), "unknown");
    }
}
