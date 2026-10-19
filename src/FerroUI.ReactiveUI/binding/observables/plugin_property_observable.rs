//! `PluginPropertyObservable<T>`.

use super::{CurrentValueDelivery, DisposeOnPanic, WeakPropertyOwner};
use crate::binding::interfaces::{ICreatesObservableForProperty, ObservedChange};
use crate::binding::observable_for_property::ValueComparer;
use ferroui_reactive::disposables::{Disposable, SerialDisposable};
use ferroui_system::component_model::PropertyOwner;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// Observes one property through a registered property observer that outranks the mechanism the
/// caller chose, reading the value with the caller's getter (`PluginPropertyObservable<T>`).
///
/// The observer says when the property changed; the value is read with the getter, never from the
/// notification. The current value is produced on subscription and again after each
/// notification. An error or the completion of the observer's notifications is passed on after
/// any value still waiting to be read.
// Deviation (DEVIATIONS.md, ReactiveUI): the object is held weakly; the getter returns `None` once
// the object it reads is dropped.
pub struct PluginPropertyObservable<T: 'static> {
    plugin: Rc<dyn ICreatesObservableForProperty>,
    source: WeakPropertyOwner,
    property_name: String,
    getter: Rc<dyn Fn() -> Option<T>>,
    before_change: bool,
    comparer: Option<ValueComparer<T>>,
}

impl<T: Clone + 'static> PluginPropertyObservable<T> {
    /// The values of `property_name` of `source`, read with `getter` after each notification of
    /// `plugin`; with `distinct_until_changed`, a value equal to the last one is dropped.
    pub fn new(
        plugin: Rc<dyn ICreatesObservableForProperty>,
        source: &PropertyOwner,
        property_name: impl Into<String>,
        getter: impl Fn() -> Option<T> + 'static,
        before_change: bool,
        distinct_until_changed: bool,
    ) -> Self
    where
        T: PartialEq,
    {
        let comparer: Option<ValueComparer<T>> =
            if distinct_until_changed { Some(Rc::new(|first: &T, second: &T| first == second)) } else { None };
        Self::with_comparer(plugin, source, property_name, getter, before_change, comparer)
    }

    /// [`new`](Self::new) with the comparison of the distinct gate as a closure (`None`: no gate).
    pub fn with_comparer(
        plugin: Rc<dyn ICreatesObservableForProperty>,
        source: &PropertyOwner,
        property_name: impl Into<String>,
        getter: impl Fn() -> Option<T> + 'static,
        before_change: bool,
        comparer: Option<ValueComparer<T>>,
    ) -> Self {
        Self {
            plugin,
            source: WeakPropertyOwner::new(source),
            property_name: property_name.into(),
            getter: Rc::new(getter),
            before_change,
            comparer,
        }
    }
}

/// Relays the notifications of the observer into the delivery (`Subscription`).
struct NotificationObserver<T: 'static> {
    delivery: Rc<CurrentValueDelivery<T>>,
}

impl<T: Clone + 'static> IObserver<ObservedChange<PropertyOwner, ()>> for NotificationObserver<T> {
    fn on_next(&self, _value: ObservedChange<PropertyOwner, ()>) {
        if self.delivery.is_terminated() {
            return;
        }
        self.delivery.changed();
    }

    fn on_error(&self, error: ObservableError) {
        self.delivery.fault(error);
    }

    fn on_completed(&self) {
        self.delivery.complete();
    }
}

impl<T: Clone + 'static> IObservable<T> for PluginPropertyObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let Some(source) = self.source.upgrade() else {
            return Disposable::empty();
        };
        let delivery = Rc::new(CurrentValueDelivery::new(observer, self.getter.clone(), self.comparer.clone()));
        let inner = SerialDisposable::new();
        // The observer is attached before the initial value is delivered (`Start`).
        let notifications = self.plugin.get_notification_for_property(&source, &self.property_name, self.before_change, false);
        drop(source);
        inner.set_disposable(Some(notifications.subscribe(Rc::new(NotificationObserver { delivery: delivery.clone() }))));
        let stop = delivery.clone();
        let subscription = DisposeOnPanic::new(Disposable::create(move || {
            stop.stop();
            inner.dispose();
        }));
        delivery.start();
        subscription.defuse()
    }
}
