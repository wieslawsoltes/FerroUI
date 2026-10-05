use super::AutomationProperty;

/// Contains values used as identifiers by `IToggleProvider`.
pub struct TogglePatternIdentifiers;

impl TogglePatternIdentifiers {
    /// Identifies the `IToggleProvider::toggle_state` automation property.
    pub fn toggle_state_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }
}
