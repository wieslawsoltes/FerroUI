//! `ObservedChangedMixins`.

use crate::binding::interfaces::{IObservedChange, ObservedChange};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The values of a sequence of changes (`ObservedChangedMixins.Value()`); implemented for
/// `Rc<dyn IObservable<ObservedChange<S, V>>>`.
pub trait ObservedChangedStreamMixins<S: 'static, V: 'static> {
    /// The value of each change (`Value()`).
    fn value(&self) -> Rc<dyn IObservable<V>>;
}

impl<S: 'static, V: 'static> ObservedChangedStreamMixins<S, V> for Rc<dyn IObservable<ObservedChange<S, V>>> {
    fn value(&self) -> Rc<dyn IObservable<V>> {
        self.select(|change: ObservedChange<S, V>| change.value)
    }
}

/// The name and the value of one change (`ObservedChangedMixins`); implemented for every
/// [`IObservedChange`].
///
/// Upstream reads the value again by reflection when the change carries the default value; a
/// change of the port always carries its value, so `get_value` and `get_value_or_default` give it.
/// `SetValueToProperty` (writing the value through a property chain by reflection) is not ported.
pub trait ObservedChangedMixins<S, V>: IObservedChange<S, V> {
    /// The path of the property that changed (`GetPropertyName()`).
    fn get_property_name(&self) -> String {
        self.expression().to_string()
    }

    /// The value of the change (`GetValue()`).
    fn get_value(&self) -> V
    where
        V: Clone,
    {
        self.value().clone()
    }

    /// The value of the change (`GetValueOrDefault()`).
    fn get_value_or_default(&self) -> V
    where
        V: Clone,
    {
        self.value().clone()
    }
}

impl<S, V, C: IObservedChange<S, V> + ?Sized> ObservedChangedMixins<S, V> for C {}
