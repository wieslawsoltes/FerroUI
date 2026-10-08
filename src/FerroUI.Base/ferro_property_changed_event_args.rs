use crate::data::core::{ValueType, ValueTypes};
use crate::data::BindingPriority;
use crate::{BoxedValue, FerroObject, FerroProperty, Ref};
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

    /// The change held by value: the form a handler receives that cannot
    /// borrow the values (a handler attached from markup).
    pub fn to_owned_args(&self) -> OwnedFerroPropertyChangedEventArgs {
        let routes = self.property.routes();
        OwnedFerroPropertyChangedEventArgs {
            sender: self.sender.to_ref(),
            property: self.property,
            old_value: self.old_value.and_then(|value| routes.route_box_value(value)).and_then(untyped),
            new_value: routes.route_box_value(self.new_value).and_then(untyped),
            priority: self.priority,
            is_effective_value_change: self.is_effective_value_change,
        }
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

/// The untyped form of a boxed property value: null or the contents of a
/// nullable, the value itself otherwise.
fn untyped(value: BoxedValue) -> Option<BoxedValue> {
    match ValueTypes::try_cast(&value, ValueType::of::<Option<BoxedValue>>()) {
        Some(untyped) => untyped.downcast_ref::<Option<BoxedValue>>().cloned().flatten(),
        None => Some(value),
    }
}

/// A property change held by value
/// ([`FerroPropertyChangedEventArgs::to_owned_args`]): the object, the
/// property and copies of the values in their untyped form.
///
/// This is what the handler of the `PropertyChanged` event of an object
/// receives when it is attached from markup.
#[derive(Clone)]
pub struct OwnedFerroPropertyChangedEventArgs {
    sender: Ref<FerroObject>,
    property: &'static FerroProperty,
    old_value: Option<BoxedValue>,
    new_value: Option<BoxedValue>,
    priority: BindingPriority,
    is_effective_value_change: bool,
}

impl OwnedFerroPropertyChangedEventArgs {
    /// The object that the property changed on.
    pub fn sender(&self) -> Ref<FerroObject> {
        self.sender.clone()
    }

    /// The property that changed.
    pub fn property(&self) -> &'static FerroProperty {
        self.property
    }

    /// The old value of the property. Null when only the base value changed.
    pub fn old_value(&self) -> Option<BoxedValue> {
        self.old_value.clone()
    }

    /// The new value of the property.
    pub fn new_value(&self) -> Option<BoxedValue> {
        self.new_value.clone()
    }

    /// The priority of the binding that produced the value.
    pub fn priority(&self) -> BindingPriority {
        self.priority
    }

    /// Whether the change represents a change to the effective value of the
    /// property (as opposed to a change of a lower-priority base value).
    pub fn is_effective_value_change(&self) -> bool {
        self.is_effective_value_change
    }
}

impl PartialEq for OwnedFerroPropertyChangedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        self.sender == other.sender
            && self.property == other.property
            && ValueTypes::identity_equals(self.old_value.as_ref(), other.old_value.as_ref())
            && ValueTypes::identity_equals(self.new_value.as_ref(), other.new_value.as_ref())
            && self.priority == other.priority
            && self.is_effective_value_change == other.is_effective_value_change
    }
}
