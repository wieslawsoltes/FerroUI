use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{any::Any, cell::RefCell};

/// Storage of the source abstract ObservableObject base; concrete models compose it.
///
/// The native notification contract carries a non-null property name and no sender.
pub struct ObservableObject {
    property_changed: Event<str>,
}

/// Virtual extension point of the original base, dispatched on the concrete model.
pub trait ObservableObjectImpl: INotifyPropertyChanged {
    fn on_property_changed(&self, property_name: &str) {
        self.property_changed().raise(property_name);
    }
}

impl ObservableObject {
    /// Field initialization for a derived model, not an abstract-type constructor.
    pub fn construct() -> Self {
        Self {
            property_changed: Event::new(),
        }
    }

    /// Store before virtual notification; no field borrow crosses equality or user code.
    /// CallerMemberName is supplied explicitly by each original setter.
    pub fn set_property<T: Clone + PartialEq + 'static>(
        owner: &(impl ObservableObjectImpl + ?Sized),
        field: &RefCell<T>,
        value: T,
        property_name: &str,
    ) -> bool {
        let old = field.borrow().clone();
        if default_equals(&old, &value) {
            return false;
        }
        let old = field.replace(value);
        drop(old);
        owner.on_property_changed(property_name);
        true
    }
}

impl INotifyPropertyChanged for ObservableObject {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

// .NET EqualityComparer<double/float>.Default uses Equals, which equates NaNs.
// Reference-class PartialEq implementations must supply source identity equality.
pub(crate) fn default_equals<T: PartialEq + 'static>(a: &T, b: &T) -> bool {
    if let (Some(a), Some(b)) = (
        (a as &dyn Any).downcast_ref::<f64>(),
        (b as &dyn Any).downcast_ref::<f64>(),
    ) {
        return a == b || a.is_nan() && b.is_nan();
    }
    if let (Some(a), Some(b)) = (
        (a as &dyn Any).downcast_ref::<f32>(),
        (b as &dyn Any).downcast_ref::<f32>(),
    ) {
        return a == b || a.is_nan() && b.is_nan();
    }
    a == b
}
