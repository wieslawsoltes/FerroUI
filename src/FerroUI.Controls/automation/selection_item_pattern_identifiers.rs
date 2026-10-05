use super::AutomationProperty;

/// Contains values used as identifiers by `ISelectionItemProvider`.
pub struct SelectionItemPatternIdentifiers;

impl SelectionItemPatternIdentifiers {
    /// Indicates the element is currently selected.
    pub fn is_selected_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Indicates the element is currently selected.
    pub fn selection_container_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }
}
