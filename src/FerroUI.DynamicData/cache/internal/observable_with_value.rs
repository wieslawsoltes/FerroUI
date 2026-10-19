//! `ObservableWithValue<TObject, TValue>` (`Cache/Internal/ObservableWithValue.cs`).

use ferroui_system::IObservable;
use std::cell::RefCell;
use std::rc::Rc;

/// An item, an observable of a value of it, and the latest value that the observable has given
/// while subscribed (`ObservableWithValue<TObject, TValue>`).
pub(crate) struct ObservableWithValue<T, V> {
    item: T,
    latest_value: RefCell<Option<V>>,
    observable: Rc<dyn IObservable<V>>,
}

impl<T, V: Clone> ObservableWithValue<T, V> {
    pub(crate) fn new(item: T, source: Rc<dyn IObservable<V>>) -> Self {
        Self { item, latest_value: RefCell::new(None), observable: source }
    }

    /// The item (`Item`).
    pub(crate) fn item(&self) -> &T {
        &self.item
    }

    /// The latest value (`LatestValue`).
    pub(crate) fn latest_value(&self) -> Option<V> {
        self.latest_value.borrow().clone()
    }

    /// Records a value of the observable (the `Do` of `Observable`).
    pub(crate) fn set_latest_value(&self, value: V) {
        *self.latest_value.borrow_mut() = Some(value);
    }

    /// The observable of the values (`Observable`, without the recording, which the subscriber
    /// does through [`set_latest_value`](Self::set_latest_value)).
    pub(crate) fn observable(&self) -> &Rc<dyn IObservable<V>> {
        &self.observable
    }
}
