use crate::data::BindingPriority;
use crate::{FerroObject, FerroProperty};
use std::any::Any;

/// Provides information for a property change.
///
/// The values are borrowed for the duration of the notification; use the typed
/// accessors to copy them out.
pub struct FerroPropertyChangedEventArgs<'a> {
    sender: &'a FerroObject,
    property: &'static FerroProperty,
    old_value: Option<&'a dyn Any>,
    new_value: &'a dyn Any,
    priority: BindingPriority,
    is_effective_value_change: bool,
}

impl<'a> FerroPropertyChangedEventArgs<'a> {
    pub(crate) fn new(
        sender: &'a FerroObject,
        property: &'static FerroProperty,
        old_value: Option<&'a dyn Any>,
        new_value: &'a dyn Any,
        priority: BindingPriority,
        is_effective_value_change: bool,
    ) -> Self {
        Self { sender, property, old_value, new_value, priority, is_effective_value_change }
    }

    /// The object that the property changed on.
    #[inline]
    pub fn sender(&self) -> &'a FerroObject {
        self.sender
    }

    /// The property that changed.
    #[inline]
    pub fn property(&self) -> &'static FerroProperty {
        self.property
    }

    /// The old value of the property, untyped. `None` when only the base
    /// value changed.
    #[inline]
    pub fn old_value(&self) -> Option<&'a dyn Any> {
        self.old_value
    }

    /// The new value of the property, untyped.
    #[inline]
    pub fn new_value(&self) -> &'a dyn Any {
        self.new_value
    }

    /// The priority of the binding that produced the value.
    #[inline]
    pub fn priority(&self) -> BindingPriority {
        self.priority
    }

    /// Whether the change represents a change to the effective value of the
    /// property (as opposed to a change of a lower-priority base value).
    #[inline]
    pub fn is_effective_value_change(&self) -> bool {
        self.is_effective_value_change
    }

    /// Gets the old value of the property. Panics if `T` is not the type of
    /// the property.
    pub fn get_old_value<T: Clone + 'static>(&self) -> Option<T> {
        self.old_value.map(|v| Self::cast::<T>(self.property, v).clone())
    }

    /// Gets the new value of the property. Panics if `T` is not the type of
    /// the property.
    pub fn get_new_value<T: Clone + 'static>(&self) -> T {
        Self::cast::<T>(self.property, self.new_value).clone()
    }

    /// Gets the old and new values of the property. Panics if `T` is not the
    /// type of the property or if there is no old value.
    pub fn get_old_and_new_value<T: Clone + 'static>(&self) -> (T, T) {
        (self.get_old_value::<T>().expect("change has no old value"), self.get_new_value::<T>())
    }

    /// Returns the same change retargeted at another sender, used when an
    /// inherited value change propagates down the tree.
    pub(crate) fn with_sender<'b>(&'b self, sender: &'b FerroObject) -> FerroPropertyChangedEventArgs<'b> {
        FerroPropertyChangedEventArgs {
            sender,
            property: self.property,
            old_value: self.old_value,
            new_value: self.new_value,
            priority: self.priority,
            is_effective_value_change: self.is_effective_value_change,
        }
    }

    fn cast<'v, T: 'static>(property: &FerroProperty, value: &'v dyn Any) -> &'v T {
        value.downcast_ref::<T>().unwrap_or_else(|| {
            panic!(
                "property '{}' is of type {}, not {}",
                property.name(),
                property.property_type_name(),
                std::any::type_name::<T>()
            )
        })
    }
}
