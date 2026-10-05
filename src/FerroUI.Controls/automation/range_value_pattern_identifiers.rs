use super::AutomationProperty;

/// Contains values used as identifiers by `IRangeValueProvider`.
pub struct RangeValuePatternIdentifiers;

impl RangeValuePatternIdentifiers {
    /// Identifies the `IRangeValueProvider::is_read_only` automation property.
    pub fn is_read_only_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the `IRangeValueProvider::minimum` automation property.
    pub fn minimum_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the `IRangeValueProvider::maximum` automation property.
    pub fn maximum_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the `IRangeValueProvider::value` automation property.
    pub fn value_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }
}
