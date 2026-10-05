use super::AutomationProperty;
use ferroui_base::BoxedValue;

/// The data of the property changed event of an automation peer.
#[derive(Clone)]
pub struct AutomationPropertyChangedEventArgs {
    property: &'static AutomationProperty,
    old_value: Option<BoxedValue>,
    new_value: Option<BoxedValue>,
}

impl AutomationPropertyChangedEventArgs {
    pub fn new(
        property: &'static AutomationProperty,
        old_value: Option<BoxedValue>,
        new_value: Option<BoxedValue>,
    ) -> Self {
        Self { property, old_value, new_value }
    }

    /// The property that changed.
    pub fn property(&self) -> &'static AutomationProperty {
        self.property
    }

    /// The old value of the property.
    pub fn old_value(&self) -> Option<&BoxedValue> {
        self.old_value.as_ref()
    }

    /// The new value of the property.
    pub fn new_value(&self) -> Option<&BoxedValue> {
        self.new_value.as_ref()
    }
}
