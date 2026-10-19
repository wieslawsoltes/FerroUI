//! `ICreatesObservableForProperty`.

use crate::binding::interfaces::ObservedChange;
use ferroui_system::component_model::PropertyOwner;
use ferroui_system::IObservable;
use std::rc::Rc;

/// Provides the change notifications of the properties of the objects it supports; of the
/// implementations registered in `AppLocator`, the one with the highest affinity for an object and
/// a property is the one used (`ICreatesObservableForProperty`).
///
/// The object is a [`PropertyOwner`]: the object that owns the property, type-erased, with its
/// `PropertyChanged` notification when it has one. It is what one step of a
/// [`PropertyPath`](ferroui_system::component_model::PropertyPath) gives
/// ([`PathStep::owner`](ferroui_system::component_model::PathStep)).
// Deviation (DEVIATIONS.md, ReactiveUI): upstream computes the affinity on a `Type`; without
// reflection it is computed on the object itself (its `PropertyOwner`). Upstream also passes the
// member `Expression`; a step of a path is named by its property name alone.
pub trait ICreatesObservableForProperty {
    /// How well this implementation can observe `property_name` of `owner`: a positive score when
    /// it can, where a higher score wins; zero or negative when it cannot
    /// (`GetAffinityForObject`).
    fn get_affinity_for_object(&self, owner: &PropertyOwner, property_name: &str, before_changed: bool) -> i32;

    /// A sequence that produces a change each time `property_name` of `owner` changes, or is about
    /// to change when `before_changed` is set (`GetNotificationForProperty`). The changes carry no
    /// value: the caller reads the current value from the object. `suppress_warnings` skips the
    /// warning written for a property that cannot notify.
    fn get_notification_for_property(
        &self,
        owner: &PropertyOwner,
        property_name: &str,
        before_changed: bool,
        suppress_warnings: bool,
    ) -> Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>>;
}
