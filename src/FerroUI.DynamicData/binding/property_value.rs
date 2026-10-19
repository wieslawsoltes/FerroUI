//! `PropertyValue<TObject, TValue>` (`Binding/PropertyValue.cs`).

use std::fmt;

/// A value of a property and the object it was read from (`PropertyValue<TObject, TValue>`).
///
/// Upstream's value of a path that a null step interrupts is the default value of `TValue`, with a
/// flag (`UnobtainableValue`); here it is `None` ([`value`](Self::value)).
#[derive(Clone)]
pub struct PropertyValue<T, V> {
    sender: T,
    value: Option<V>,
}

impl<T, V> PropertyValue<T, V> {
    /// The value of a property of `sender` (`PropertyValue(TObject sender, TValue value)`).
    pub fn new(sender: T, value: V) -> Self {
        Self { sender, value: Some(value) }
    }

    /// A value that could not be read, as a step of the path was null
    /// (`PropertyValue(TObject sender)`).
    pub fn unobtainable(sender: T) -> Self {
        Self { sender, value: None }
    }

    /// The object the value was read from (`Sender`).
    pub fn sender(&self) -> &T {
        &self.sender
    }

    /// The value (`Value`); `None` when it could not be read.
    pub fn value(&self) -> Option<&V> {
        self.value.as_ref()
    }

    /// The value, giving up the rest.
    pub fn into_value(self) -> Option<V> {
        self.value
    }

    /// Whether the value could not be read, as a step of the path was null (`UnobtainableValue`).
    pub fn is_unobtainable(&self) -> bool {
        self.value.is_none()
    }
}

impl<T: PartialEq, V: PartialEq> PartialEq for PropertyValue<T, V> {
    /// `Equals`: two values that could not be read are equal whatever their senders, as are two
    /// null values upstream; otherwise the senders and the values are compared.
    fn eq(&self, other: &Self) -> bool {
        match (&self.value, &other.value) {
            (None, None) => true,
            (Some(value), Some(other_value)) => self.sender == other.sender && value == other_value,
            _ => false,
        }
    }
}

impl<T: fmt::Display, V: fmt::Debug> fmt::Display for PropertyValue<T, V> {
    /// `ToString`: the sender and the value in parentheses.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            Some(value) => write!(f, "{} ({:?})", self.sender, value),
            None => write!(f, "{} ()", self.sender),
        }
    }
}

impl<T: fmt::Debug, V: fmt::Debug> fmt::Debug for PropertyValue<T, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyValue").field("sender", &self.sender).field("value", &self.value).finish()
    }
}
