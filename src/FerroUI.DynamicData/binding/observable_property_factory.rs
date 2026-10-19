//! `ObservablePropertyFactory<TObject, TProperty>` (`Binding/ObservablePropertyFactory.cs`): the
//! sequence of the values of a chain of properties on an object.
//!
//! `ObservablePropertyFactoryCache`, which caches the factories of the expressions that
//! reflection parses, is not ported: a [`PropertyChain`] is built once, at compile time, by
//! `prop!`.

use super::{PropertyChain, PropertyValue};
use crate::internal::DeliveryQueue;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::rc::Rc;

/// The values of `chain` on `source` (`ObservablePropertyFactory.Create`): the current value
/// first when `notify_initial` is true, then a value whenever a step of the chain changes. A value
/// that a null step interrupts is [`unobtainable`](PropertyValue::unobtainable).
///
/// The handlers are attached before the value is read, so that a change during the subscription
/// is seen; there is no deduplication of equal values. The values go through a
/// [`DeliveryQueue`], so that a change made by an observer while it handles a value is delivered
/// after it (the `DeliveryQueue` and `SharedDeliveryQueue` of the shallow and deep subscriptions).
///
/// Deviation (DEVIATIONS.md, DynamicData): a notification of every property (an empty name, which
/// is `null` upstream) changes the value, as the steps of a `PropertyPath` see it; upstream
/// compares the name of the notification with the name of the property only.
pub(crate) fn observe_property<S: Clone + 'static, V: 'static>(chain: PropertyChain<S, V>, source: S, notify_initial: bool) -> Rc<dyn IObservable<PropertyValue<S, V>>> {
    Observable::create(move |observer: Rc<dyn IObserver<PropertyValue<S, V>>>| {
        let queue = DeliveryQueue::new(observer);
        let emit_current: Rc<dyn Fn()> = {
            let (chain, source, queue) = (chain.clone(), source.clone(), queue.clone());
            Rc::new(move || {
                let value = match chain.get(&source) {
                    Some(value) => PropertyValue::new(source.clone(), value),
                    None => PropertyValue::unobtainable(source.clone()),
                };
                queue.acquire().enqueue_next(value);
            })
        };
        // Attach before reading so changes during initialization are captured.
        let observation = chain.observe(&source, emit_current.clone());
        if notify_initial {
            emit_current();
        }
        Disposable::create(move || {
            observation.dispose();
            queue.dispose();
        })
    })
}

/// The values of `chain` on `source` as `PropertyValue`s, without the unobtainable ones unless
/// there is a fallback (`NotifyPropertyChangedEx.WhenPropertyChanged`).
pub(crate) fn when_property_changed_of<S: Clone + 'static, V: 'static>(
    chain: PropertyChain<S, V>,
    source: S,
    notify_on_initial_value: bool,
    has_fallback: bool,
) -> Rc<dyn IObservable<PropertyValue<S, V>>> {
    observe_property(chain, source, notify_on_initial_value).where_(move |value: &PropertyValue<S, V>| !value.is_unobtainable() || has_fallback)
}

/// The values of `chain` on `source`, `fallback_value` for an unobtainable one, which is dropped
/// without a fallback (`NotifyPropertyChangedEx.WhenChanged(expression, notifyInitial,
/// fallbackValue)`).
pub(crate) fn when_changed_of<S: Clone + 'static, V: 'static>(
    chain: PropertyChain<S, V>,
    source: S,
    notify_initial: bool,
    fallback_value: Option<Rc<dyn Fn() -> V>>,
) -> Rc<dyn IObservable<V>> {
    let has_fallback = fallback_value.is_some();
    observe_property(chain, source, notify_initial)
        .where_(move |value: &PropertyValue<S, V>| !value.is_unobtainable() || has_fallback)
        .select(move |value: PropertyValue<S, V>| match value.into_value() {
            Some(value) => value,
            None => (fallback_value.as_ref().expect("an unobtainable value passes only with a fallback"))(),
        })
}
