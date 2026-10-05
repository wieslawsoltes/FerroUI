/// Contains values that specify the `ExpandCollapseState` automation property value of a UI
/// Automation element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ExpandCollapseState {
    /// No child nodes, controls, or content of the UI Automation element are displayed.
    Collapsed,

    /// All child nodes, controls or content of the UI Automation element are displayed.
    Expanded,

    /// The UI Automation element has no child nodes, controls, or content to display.
    LeafNode,

    /// Some, but not all, child nodes, controls, or content of the UI Automation element are
    /// displayed.
    PartiallyExpanded,
}
