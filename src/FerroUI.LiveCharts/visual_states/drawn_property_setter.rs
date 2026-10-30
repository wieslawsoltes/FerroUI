use crate::motion::PropertyValue;

/// A named value assignment applied through an animatable's type-checked registry.
#[derive(Clone, Debug)]
pub struct DrawnPropertySetter {
    pub property_name: String,
    pub value: PropertyValue,
}

impl DrawnPropertySetter {
    pub fn new<T: 'static>(property_name: impl Into<String>, value: T) -> Self {
        Self {
            property_name: property_name.into(),
            value: PropertyValue::new(value),
        }
    }
    pub fn erased(property_name: impl Into<String>, value: PropertyValue) -> Self {
        Self {
            property_name: property_name.into(),
            value,
        }
    }
    /// Stores owned text, including text to be parsed by a numeric property.
    pub fn text(property_name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(property_name, value.into())
    }
}
