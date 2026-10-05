use super::AutomationProperty;

/// Contains values used as identifiers by `IExpandCollapseProvider`.
pub struct ExpandCollapsePatternIdentifiers;

impl ExpandCollapsePatternIdentifiers {
    /// Identifies the `IExpandCollapseProvider::expand_collapse_state` automation property.
    pub fn expand_collapse_state_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }
}
