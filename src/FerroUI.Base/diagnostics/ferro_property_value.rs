//! Port of `Diagnostics/FerroPropertyValue.cs`.

use crate::data::BindingPriority;
use crate::{BoxedValue, FerroProperty};

/// Holds diagnostic-related information about the value of a
/// [`FerroProperty`] on a [`FerroObject`](crate::FerroObject).
#[derive(Clone)]
pub struct FerroPropertyValue {
    property: &'static FerroProperty,
    value: BoxedValue,
    priority: BindingPriority,
    diagnostic: Option<String>,
    is_overridden_current_value: bool,
}

impl FerroPropertyValue {
    pub(crate) fn new(
        property: &'static FerroProperty,
        value: BoxedValue,
        priority: BindingPriority,
        diagnostic: Option<String>,
        is_overridden_current_value: bool,
    ) -> Self {
        Self { property, value, priority, diagnostic, is_overridden_current_value }
    }

    /// Gets the property.
    pub fn property(&self) -> &'static FerroProperty {
        self.property
    }

    /// Gets the current property value: a value of the type of the property.
    pub fn value(&self) -> &BoxedValue {
        &self.value
    }

    /// Gets the priority of the current value.
    pub fn priority(&self) -> BindingPriority {
        self.priority
    }

    /// Gets a diagnostic string.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    /// Gets a value indicating whether the [`value`](Self::value) was
    /// overridden by a call to
    /// [`FerroObject::set_current_value`](crate::FerroObject::set_current_value).
    pub fn is_overridden_current_value(&self) -> bool {
        self.is_overridden_current_value
    }
}
