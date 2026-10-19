//! `INPCObservableForProperty`.

use crate::binding::bindings::BindingAffinity;
use crate::binding::interfaces::{ICreatesObservableForProperty, ObservedChange};
use crate::binding::observables::NotifyPropertyChangedObservable;
use ferroui_reactive::linq::Observable;
use ferroui_system::component_model::PropertyOwner;
use ferroui_system::IObservable;
use std::rc::Rc;

/// Observes the properties of the objects that raise `PropertyChanged`, or `PropertyChanging`
/// before a change (`INPCObservableForProperty`).
#[derive(Debug, Default, Clone, Copy)]
pub struct INPCObservableForProperty;

impl INPCObservableForProperty {
    /// The observer (`new INPCObservableForProperty()`).
    pub fn new() -> Self {
        Self
    }
}

impl ICreatesObservableForProperty for INPCObservableForProperty {
    /// [`BindingAffinity::EXPLICIT`] when the object raises `PropertyChanging` (before a change) or
    /// `PropertyChanged` (after it); zero otherwise. The property name is not consulted.
    fn get_affinity_for_object(&self, owner: &PropertyOwner, _property_name: &str, before_changed: bool) -> i32 {
        let supported = match &owner.notify {
            Some(notify) if before_changed => notify.as_notify_property_changing().is_some(),
            Some(_) => true,
            None => false,
        };
        if supported {
            BindingAffinity::EXPLICIT
        } else {
            0
        }
    }

    /// A sequence of the notifications of `property_name`; an object that does not raise
    /// `PropertyChanging` is observed through `PropertyChanged` before a change too, and an object
    /// that raises neither gives a sequence that never produces anything.
    fn get_notification_for_property(
        &self,
        owner: &PropertyOwner,
        property_name: &str,
        before_changed: bool,
        _suppress_warnings: bool,
    ) -> Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>> {
        // Upstream matches an indexer as `Name[]`; a property path has no indexers.
        if owner.notify.is_some() {
            return Rc::new(NotifyPropertyChangedObservable::new(owner.clone(), property_name, before_changed));
        }
        Observable::never()
    }
}
