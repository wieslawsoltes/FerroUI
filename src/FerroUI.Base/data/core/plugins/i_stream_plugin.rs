use crate::data::core::WeakValue;
use crate::reactive::{IObservable, Observable};
use crate::{BoxedValue, PropertyValue};
use std::rc::Rc;

/// Defines a plugin that handles the `^` stream binding operator.
pub trait IStreamPlugin {
    /// Checks whether this plugin handles the specified value.
    fn match_(&self, reference: &WeakValue) -> bool;

    /// Starts producing output based on the specified value.
    fn start(&self, reference: &WeakValue) -> Option<Rc<dyn IObservable<Option<BoxedValue>>>>;
}

/// An observable as a value that can be stored in a property and streamed by
/// a binding with the `^` operator. It has identity equality.
#[derive(Clone)]
pub struct ObservableValue(pub Rc<dyn IObservable<Option<BoxedValue>>>);

impl ObservableValue {
    /// Wraps a typed observable, boxing each value.
    pub fn new<T: PropertyValue>(source: Rc<dyn IObservable<T>>) -> Self {
        use crate::reactive::ObservableExt;
        Self(source.select(|v| Some(Rc::new(v) as BoxedValue)))
    }
}

impl PartialEq for ObservableValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Handles binding to [`ObservableValue`]s for the `^` stream binding
/// operator.
pub struct ObservableStreamPlugin;

impl IStreamPlugin for ObservableStreamPlugin {
    fn match_(&self, reference: &WeakValue) -> bool {
        reference.upgrade().is_some_and(|t| t.is::<ObservableValue>())
    }

    fn start(&self, reference: &WeakValue) -> Option<Rc<dyn IObservable<Option<BoxedValue>>>> {
        match reference.upgrade() {
            Some(target) => target.downcast_ref::<ObservableValue>().map(|o| o.0.clone()),
            None => Some(Observable::empty()),
        }
    }
}
