//! Port of `Diagnostics/FerroObjectExtensions.cs` (with the parts of
//! `FerroObject.GetDiagnosticInternal` and `ValueStore.GetDiagnostic` it
//! is built on).

use super::{FerroPropertyValue, ValueStoreDiagnostic};
use crate::data::BindingPriority;
use crate::{FerroObject, FerroProperty};

/// Defines diagnostic extensions on [`FerroObject`]s.
pub trait FerroObjectDiagnosticExtensions {
    /// Gets a diagnostic for a [`FerroProperty`] on a [`FerroObject`]: a
    /// [`FerroPropertyValue`] that can be used to diagnose the state of the
    /// property on the object.
    fn get_diagnostic(&self, property: &'static FerroProperty) -> FerroPropertyValue;

    /// Gets a [`ValueStoreDiagnostic`]: the frames of values that are
    /// applied to the object.
    fn get_value_store_diagnostic(&self) -> ValueStoreDiagnostic;
}

impl FerroObjectDiagnosticExtensions for FerroObject {
    fn get_diagnostic(&self, property: &'static FerroProperty) -> FerroPropertyValue {
        // `FerroObject.GetDiagnosticInternal`
        if property.is_direct() {
            return FerroPropertyValue::new(
                property,
                self.get_value_untyped(property),
                BindingPriority::LocalValue,
                None,
                false,
            );
        }

        // `ValueStore.GetDiagnostic`: the effective value of the object, else
        // the value it inherits, else the default value of the property.
        let values = self.values();
        let (value, priority, overridden) = match values.get_effective_value(property) {
            Some(effective) => (effective.boxed_value(), effective.priority(), effective.is_overriden_current_value()),
            None => {
                let inherited =
                    if property.inherits() { values.try_get_inherited_value(property) } else { None };
                match inherited {
                    Some(inherited) => (inherited.boxed_value(), BindingPriority::Inherited, false),
                    None => (self.get_value_untyped(property), BindingPriority::Unset, false),
                }
            }
        };

        FerroPropertyValue::new(property, value, priority, None, overridden)
    }

    fn get_value_store_diagnostic(&self) -> ValueStoreDiagnostic {
        self.values().get_store_diagnostic()
    }
}
