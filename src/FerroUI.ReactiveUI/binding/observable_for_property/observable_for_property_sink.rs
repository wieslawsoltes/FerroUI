//! `ObservableForPropertySink<TSender, TValue>`.

use super::expression_chain_parameters::ValueComparer;
use crate::binding::interfaces::ObservedChange;
use ferroui_system::component_model::PropertyOwner;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Produces the value of a property on subscription, unless skipped, and again on each
/// notification of a property observer (`ObservableForPropertySink<TSender, TValue>`).
///
/// The value is read with the getter on every notification; the value of the notification is not
/// used. The errors and the completion of the notifications pass through.
// Deviation (DEVIATIONS.md, ReactiveUI): the sender is held weakly; once it is dropped, nothing is
// produced.
pub struct ObservableForPropertySink<T: 'static, V: 'static> {
    sender: Weak<T>,
    expression: String,
    notifications: Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>>,
    read_value: Rc<dyn Fn(&Rc<T>) -> V>,
    skip_initial: bool,
    comparer: Option<ValueComparer<V>>,
}

impl<T: 'static, V: Clone + 'static> ObservableForPropertySink<T, V> {
    /// The values of the property `expression` of `sender`, read with `read_value` on each of
    /// `notifications`; `comparer`, when given, drops a value equal to the last one
    /// (`isDistinct`).
    pub fn new(
        sender: &Rc<T>,
        expression: impl Into<String>,
        notifications: Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>>,
        read_value: impl Fn(&Rc<T>) -> V + 'static,
        skip_initial: bool,
        comparer: Option<ValueComparer<V>>,
    ) -> Self {
        Self {
            sender: Rc::downgrade(sender),
            expression: expression.into(),
            notifications,
            read_value: Rc::new(read_value),
            skip_initial,
            comparer,
        }
    }
}

impl<T: 'static, V: Clone + 'static> IObservable<ObservedChange<Rc<T>, V>> for ObservableForPropertySink<T, V> {
    fn subscribe(&self, observer: Rc<dyn IObserver<ObservedChange<Rc<T>, V>>>) -> Rc<dyn IDisposable> {
        let sink = Rc::new(Sink {
            downstream: observer,
            sender: self.sender.clone(),
            expression: self.expression.clone(),
            read_value: self.read_value.clone(),
            comparer: self.comparer.clone(),
            last: RefCell::new(None),
        });
        if !self.skip_initial {
            sink.emit();
        }
        self.notifications.subscribe(sink)
    }
}

/// Reads the value on each notification (`Sink`).
struct Sink<T: 'static, V: 'static> {
    downstream: Rc<dyn IObserver<ObservedChange<Rc<T>, V>>>,
    sender: Weak<T>,
    expression: String,
    read_value: Rc<dyn Fn(&Rc<T>) -> V>,
    comparer: Option<ValueComparer<V>>,
    last: RefCell<Option<V>>,
}

impl<T: 'static, V: Clone + 'static> Sink<T, V> {
    /// Reads the value and produces it, unless it repeats the last one under the distinct gate
    /// (`Emit`).
    fn emit(&self) {
        let Some(sender) = self.sender.upgrade() else {
            return;
        };
        let current = (self.read_value)(&sender);
        if let (Some(comparer), Some(last)) = (&self.comparer, self.last.borrow().as_ref()) {
            if comparer(&current, last) {
                return;
            }
        }
        *self.last.borrow_mut() = Some(current.clone());
        self.downstream.on_next(ObservedChange::new(sender, self.expression.clone(), current));
    }
}

impl<T: 'static, V: Clone + 'static> IObserver<ObservedChange<PropertyOwner, ()>> for Sink<T, V> {
    fn on_next(&self, _value: ObservedChange<PropertyOwner, ()>) {
        self.emit();
    }

    fn on_error(&self, error: ObservableError) {
        self.downstream.on_error(error);
    }

    fn on_completed(&self) {
        self.downstream.on_completed();
    }
}
