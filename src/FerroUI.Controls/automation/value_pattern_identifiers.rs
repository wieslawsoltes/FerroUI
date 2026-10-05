use super::AutomationProperty;

/// Contains values used as identifiers by `IValueProvider`.
pub struct ValuePatternIdentifiers;

impl ValuePatternIdentifiers {
    /// Identifies the `IValueProvider::is_read_only` automation property.
    pub fn is_read_only_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the `IValueProvider::value` automation property.
    pub fn value_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }
}
