//! `ObservedChange<TSender, TValue>`.

use crate::binding::interfaces::IObservedChange;

/// One property change: the object that raised it, the path of the property and its value
/// (`ObservedChange<TSender, TValue>`).
///
/// A notification of a property observer ([`ICreatesObservableForProperty`]) carries no value
/// (`V` is `()`): the reader reads the current value from the sender.
///
/// [`ICreatesObservableForProperty`]: crate::binding::interfaces::ICreatesObservableForProperty
#[derive(Debug, Clone, PartialEq)]
pub struct ObservedChange<S, V> {
    /// The object that raised the change (`Sender`).
    pub sender: S,
    /// The path of the property that changed, `Address.Street` (`Expression`).
    pub expression: String,
    /// The value of the property (`Value`).
    pub value: V,
}

impl<S, V> ObservedChange<S, V> {
    /// A change of `expression` on `sender` with `value`
    /// (`ObservedChange(TSender, Expression?, TValue)`).
    pub fn new(sender: S, expression: impl Into<String>, value: V) -> Self {
        Self { sender, expression: expression.into(), value }
    }
}

impl<S, V> IObservedChange<S, V> for ObservedChange<S, V> {
    fn sender(&self) -> &S {
        &self.sender
    }

    fn expression(&self) -> &str {
        &self.expression
    }

    fn value(&self) -> &V {
        &self.value
    }
}
