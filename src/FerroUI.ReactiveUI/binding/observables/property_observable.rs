//! `PropertyObservable<T>`, and the delivery of a current value that it shares with
//! `PluginPropertyObservable<T>` (Primitives' `CurrentValueDelivery<T>`).

use crate::binding::observable_for_property::ValueComparer;
use ferroui_reactive::disposables::Disposable;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The terminal notification requested of a [`CurrentValueDelivery`].
enum Terminal {
    None,
    Requested(Option<ObservableError>),
    Delivered,
}

/// Delivers the current value of a property: a change marks the value as changed, and the value
/// is read and delivered by the one delivery running; a change raised while a value is being
/// delivered is delivered after it, read again (`CurrentValueDelivery<T>` with its gate).
pub(crate) struct CurrentValueDelivery<T: 'static> {
    observer: Rc<dyn IObserver<T>>,
    reader: Rc<dyn Fn() -> Option<T>>,
    comparer: Option<ValueComparer<T>>,
    last: RefCell<Option<T>>,
    changed: Cell<bool>,
    started: Cell<bool>,
    stopped: Cell<bool>,
    signaled: Cell<bool>,
    draining: Cell<bool>,
    terminal: RefCell<Terminal>,
}

/// Disposes a subscription when its start panics, as upstream disposes it in a `catch` before
/// rethrowing; [`defuse`](Self::defuse) hands the subscription over once it started.
pub(crate) struct DisposeOnPanic(Option<Rc<dyn IDisposable>>);

impl DisposeOnPanic {
    pub(crate) fn new(subscription: Rc<dyn IDisposable>) -> Self {
        Self(Some(subscription))
    }

    pub(crate) fn defuse(mut self) -> Rc<dyn IDisposable> {
        self.0.take().expect("the subscription is held until defused")
    }
}

impl Drop for DisposeOnPanic {
    fn drop(&mut self) {
        if let Some(subscription) = self.0.take() {
            subscription.dispose();
        }
    }
}

/// Leaves the gate when the delivery ends, a panic included.
struct GateExit<'a>(&'a Cell<bool>);

impl Drop for GateExit<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl<T: Clone + 'static> CurrentValueDelivery<T> {
    /// A delivery to `observer` of the values `reader` reads (`None` once the source is gone);
    /// `comparer`, when given, drops a value equal to the last one.
    pub(crate) fn new(observer: Rc<dyn IObserver<T>>, reader: Rc<dyn Fn() -> Option<T>>, comparer: Option<ValueComparer<T>>) -> Self {
        Self {
            observer,
            reader,
            comparer,
            last: RefCell::new(None),
            changed: Cell::new(false),
            started: Cell::new(false),
            stopped: Cell::new(false),
            signaled: Cell::new(false),
            draining: Cell::new(false),
            terminal: RefCell::new(Terminal::None),
        }
    }

    /// Whether the terminal notification was delivered or the delivery was stopped
    /// (`IsTerminated`).
    pub(crate) fn is_terminated(&self) -> bool {
        matches!(*self.terminal.borrow(), Terminal::Delivered) || self.stopped.get()
    }

    /// Starts the delivery with the current value (`Start`).
    pub(crate) fn start(&self) {
        self.started.set(true);
        self.changed();
    }

    /// Marks the value as changed and delivers it (`Changed`).
    pub(crate) fn changed(&self) {
        if !self.started.get() || self.stopped.get() {
            return;
        }
        self.changed.set(true);
        self.signal();
    }

    /// Requests the error `error` after the pending value (`Fault`).
    pub(crate) fn fault(&self, error: ObservableError) {
        self.request_terminal(Some(error));
    }

    /// Requests the completion after the pending value (`Complete`).
    pub(crate) fn complete(&self) {
        self.request_terminal(None);
    }

    /// Stops the delivery (`Stop`).
    pub(crate) fn stop(&self) {
        self.stopped.set(true);
        self.changed.set(false);
    }

    fn request_terminal(&self, error: Option<ObservableError>) {
        {
            let mut terminal = self.terminal.borrow_mut();
            if !matches!(*terminal, Terminal::None) {
                return;
            }
            *terminal = Terminal::Requested(error);
        }
        if self.started.get() {
            self.signal();
        }
    }

    /// Drains while the delivery is signaled; a signal raised during a delivery is drained by the
    /// delivery running (`DeliveryGate.Signal`).
    fn signal(&self) {
        self.signaled.set(true);
        if self.draining.replace(true) {
            return;
        }
        let _exit = GateExit(&self.draining);
        while self.signaled.replace(false) {
            if self.drain() {
                break;
            }
        }
    }

    /// Delivers the changed value, then the terminal notification if one is requested; returns
    /// whether the terminal notification was delivered (`DrainTo`).
    fn drain(&self) -> bool {
        if self.is_terminated() {
            return false;
        }
        if self.changed.replace(false) {
            self.deliver_current();
        }
        if self.stopped.get() {
            return false;
        }
        if self.changed.get() {
            // A change raised during the delivery is delivered before the terminal notification
            // (`DeliveryObserver.DeliverPendingValue`); the signal it raised drains it next.
            return false;
        }
        let requested = {
            let mut terminal = self.terminal.borrow_mut();
            match std::mem::replace(&mut *terminal, Terminal::Delivered) {
                Terminal::Requested(error) => Some(error),
                other => {
                    *terminal = other;
                    None
                }
            }
        };
        match requested {
            Some(Some(error)) => {
                self.observer.on_error(error);
                true
            }
            Some(None) => {
                self.observer.on_completed();
                true
            }
            None => false,
        }
    }

    /// Reads the value and delivers it, unless it repeats the last one (`DeliverCurrent`).
    fn deliver_current(&self) {
        let Some(value) = (self.reader)() else {
            return;
        };
        if let Some(comparer) = &self.comparer {
            let repeats = self.last.borrow().as_ref().is_some_and(|last| comparer(&value, last));
            if repeats {
                return;
            }
        }
        if self.comparer.is_some() {
            *self.last.borrow_mut() = Some(value.clone());
        }
        if self.stopped.get() {
            return;
        }
        self.observer.on_next(value);
    }
}

/// The values of one property of an object that raises `PropertyChanged`: the current value on
/// subscription, then the value after each notification of the property or of every property
/// (`PropertyObservable<T>`).
///
/// A notification raised while a value is being delivered is delivered after it, with the value
/// read again; with `distinct_until_changed`, a value equal to the last one is dropped.
// Deviation (DEVIATIONS.md, ReactiveUI): the source is held weakly; once it is dropped, the
// sequence produces nothing more.
pub struct PropertyObservable<S: ?Sized + 'static, T: 'static> {
    source: Weak<S>,
    property_name: String,
    getter: Rc<dyn Fn(&Rc<S>) -> T>,
    comparer: Option<ValueComparer<T>>,
}

impl<S: INotifyPropertyChanged + ?Sized + 'static, T: Clone + 'static> PropertyObservable<S, T> {
    /// The values of `property_name` of `source`, read with `getter`.
    pub fn new(source: &Rc<S>, property_name: impl Into<String>, getter: impl Fn(&Rc<S>) -> T + 'static, distinct_until_changed: bool) -> Self
    where
        T: PartialEq,
    {
        let comparer: Option<ValueComparer<T>> = if distinct_until_changed { Some(Rc::new(|first: &T, second: &T| first == second)) } else { None };
        Self::with_comparer(source, property_name, getter, comparer)
    }

    /// The values of `property_name` of `source`, dropping a value that `comparer` finds equal to
    /// the last one.
    pub fn with_comparer(source: &Rc<S>, property_name: impl Into<String>, getter: impl Fn(&Rc<S>) -> T + 'static, comparer: Option<ValueComparer<T>>) -> Self {
        Self { source: Rc::downgrade(source), property_name: property_name.into(), getter: Rc::new(getter), comparer }
    }
}

impl<S: INotifyPropertyChanged + ?Sized + 'static, T: Clone + 'static> IObservable<T> for PropertyObservable<S, T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let Some(source) = self.source.upgrade() else {
            return Disposable::empty();
        };
        let reader_source = self.source.clone();
        let getter = self.getter.clone();
        let reader: Rc<dyn Fn() -> Option<T>> = Rc::new(move || reader_source.upgrade().map(|source| getter(&source)));
        let delivery = Rc::new(CurrentValueDelivery::new(observer, reader, self.comparer.clone()));
        let handler_delivery = delivery.clone();
        let name = self.property_name.clone();
        let token = source.property_changed().add(Rc::new(move |notified: &str| {
            if notified.is_empty() || notified == name {
                handler_delivery.changed();
            }
        }));
        drop(source);
        let weak_source = self.source.clone();
        let stop = delivery.clone();
        let subscription = DisposeOnPanic::new(Disposable::create(move || {
            stop.stop();
            if let Some(source) = weak_source.upgrade() {
                source.property_changed().remove(token);
            }
        }));
        delivery.start();
        subscription.defuse()
    }
}
