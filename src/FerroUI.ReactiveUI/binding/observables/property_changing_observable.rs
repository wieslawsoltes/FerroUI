//! `PropertyChangingObservable<T>`.

use super::DisposeOnPanic;
use ferroui_reactive::disposables::Disposable;
use ferroui_system::component_model::INotifyPropertyChanging;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// The values of one property of an object that raises `PropertyChanging`: the current value on
/// subscription, then the value before each change of the property or of every property (an empty
/// name) (`PropertyChangingObservable<T>`).
///
/// A notification raised while the current value is read on subscription is delivered after it;
/// one raised while a value is being delivered is delivered at once, nested, as upstream's
/// re-entrant delivery does on its own thread.
// Deviation (DEVIATIONS.md, ReactiveUI): the source is held weakly; once it is dropped, the
// sequence produces nothing more.
pub struct PropertyChangingObservable<S: ?Sized + 'static, T: 'static> {
    source: Weak<S>,
    property_name: String,
    getter: Rc<dyn Fn(&Rc<S>) -> T>,
}

impl<S: INotifyPropertyChanging + ?Sized + 'static, T: 'static> PropertyChangingObservable<S, T> {
    /// The values of `property_name` of `source`, read with `getter` before each change.
    pub fn new(source: &Rc<S>, property_name: impl Into<String>, getter: impl Fn(&Rc<S>) -> T + 'static) -> Self {
        Self { source: Rc::downgrade(source), property_name: property_name.into(), getter: Rc::new(getter) }
    }
}

/// One subscription (`Subscription` with its `SerializedDelivery`).
struct Subscription<S: ?Sized + 'static, T: 'static> {
    source: Weak<S>,
    getter: Rc<dyn Fn(&Rc<S>) -> T>,
    observer: Rc<dyn IObserver<T>>,
    reading_initial: Cell<bool>,
    pending: RefCell<VecDeque<T>>,
    disposed: Cell<bool>,
}

impl<S: ?Sized + 'static, T: 'static> Subscription<S, T> {
    fn read(&self) -> Option<T> {
        self.source.upgrade().map(|source| (self.getter)(&source))
    }

    fn on_property_changing(&self) {
        if self.disposed.get() {
            return;
        }
        let Some(value) = self.read() else {
            return;
        };
        if self.reading_initial.get() {
            self.pending.borrow_mut().push_back(value);
            return;
        }
        self.observer.on_next(value);
    }

    /// Delivers the current value, then what was raised while it was read (`Start`).
    fn start(&self) {
        self.reading_initial.set(true);
        let initial = self.read();
        self.reading_initial.set(false);
        if let Some(initial) = initial {
            if !self.disposed.get() {
                self.observer.on_next(initial);
            }
        }
        loop {
            let next = self.pending.borrow_mut().pop_front();
            match next {
                Some(value) if !self.disposed.get() => self.observer.on_next(value),
                Some(_) => {}
                None => break,
            }
        }
    }
}

impl<S: INotifyPropertyChanging + ?Sized + 'static, T: 'static> IObservable<T> for PropertyChangingObservable<S, T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let Some(source) = self.source.upgrade() else {
            return Disposable::empty();
        };
        let subscription = Rc::new(Subscription {
            source: self.source.clone(),
            getter: self.getter.clone(),
            observer,
            reading_initial: Cell::new(false),
            pending: RefCell::new(VecDeque::new()),
            disposed: Cell::new(false),
        });
        let handler_subscription = subscription.clone();
        let name = self.property_name.clone();
        let token = source.property_changing().add(Rc::new(move |notified: &str| {
            if notified.is_empty() || notified == name {
                handler_subscription.on_property_changing();
            }
        }));
        drop(source);
        let weak_source = self.source.clone();
        let state = subscription.clone();
        let disposable = DisposeOnPanic::new(Disposable::create(move || {
            if state.disposed.replace(true) {
                return;
            }
            state.pending.borrow_mut().clear();
            if let Some(source) = weak_source.upgrade() {
                source.property_changing().remove(token);
            }
        }));
        subscription.start();
        disposable.defuse()
    }
}
