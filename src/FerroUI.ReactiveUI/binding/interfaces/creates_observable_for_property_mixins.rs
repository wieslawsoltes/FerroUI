//! `CreatesObservableForPropertyMixins`.

use crate::binding::interfaces::{ICreatesObservableForProperty, ObservedChange};
use ferroui_system::component_model::PropertyOwner;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The members of [`ICreatesObservableForProperty`] with the before-change and warning flags set
/// to `false` (`CreatesObservableForPropertyMixins`).
///
/// Rust has no overloads, so the shorter forms have names of their own.
pub trait CreatesObservableForPropertyMixins: ICreatesObservableForProperty {
    /// The affinity for the after-change observation of `property_name`
    /// (`GetAffinityForObject(Type, string)`).
    fn get_affinity_for_object_after_change(&self, owner: &PropertyOwner, property_name: &str) -> i32 {
        self.get_affinity_for_object(owner, property_name, false)
    }

    /// The after-change notification of `property_name`, without suppressing warnings
    /// (`GetNotificationForProperty(object, Expression, string)`).
    fn get_notification_for_property_after_change(
        &self,
        owner: &PropertyOwner,
        property_name: &str,
    ) -> Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>> {
        self.get_notification_for_property(owner, property_name, false, false)
    }

    /// The notification of `property_name`, before or after the change, without suppressing
    /// warnings (`GetNotificationForProperty(object, Expression, string, bool)`).
    fn get_notification_for_property_with_warnings(
        &self,
        owner: &PropertyOwner,
        property_name: &str,
        before_changed: bool,
    ) -> Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>> {
        self.get_notification_for_property(owner, property_name, before_changed, false)
    }
}

impl<F: ICreatesObservableForProperty + ?Sized> CreatesObservableForPropertyMixins for F {}
