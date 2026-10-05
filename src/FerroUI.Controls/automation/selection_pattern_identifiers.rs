use super::AutomationProperty;

/// Contains values used as identifiers by `ISelectionProvider`.
pub struct SelectionPatternIdentifiers;

impl SelectionPatternIdentifiers {
    /// Identifies the `ISelectionProvider::can_select_multiple` property.
    pub fn can_select_multiple_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the `ISelectionProvider::is_selection_required` property.
    pub fn is_selection_required_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the property that gets the selected items in a container.
    pub fn selection_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }
}
