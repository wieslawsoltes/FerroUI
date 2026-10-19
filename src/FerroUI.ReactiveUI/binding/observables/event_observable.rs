//! `EventObservable<T>`.

use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// The handler of a parameterless change event (`EventHandler`): an event source calls it when
/// the value may have changed. A handler is compared by identity (`Rc::ptr_eq`), so the handler
/// given to `remove_handler` is the one given to `add_handler`.
type EventHandler = Rc<dyn Fn()>;

/// The value of a property on subscription, then again each time its change event is raised
/// (`EventObservable<T>`).
///
/// The value comes from the getter, which is read on subscription and on every raise, so the
/// arguments of the event are ignored. The value read on subscription is always delivered; with
/// `distinct_until_changed`, a later value equal to the last one delivered is dropped. A getter that
/// panics on subscription detaches the handler before the panic propagates; one that panics while
/// the event is raised propagates to whoever raised it. The sequence never completes.
pub struct EventObservable<T: 'static> {
    add_handler: Rc<dyn Fn(EventHandler)>,
    remove_handler: Rc<dyn Fn(EventHandler)>,
    getter: Rc<dyn Fn() -> T>,
    distinct_until_changed: bool,
}

impl<T: Clone + PartialEq + 'static> EventObservable<T> {
    /// The values `getter` reads, on subscription and each time the event that `add_handler`
    /// subscribes a handler to (and `remove_handler` unsubscribes it from) is raised
    /// (`EventObservable(Action<EventHandler>, Action<EventHandler>, Func<T>, bool)`). A handler is
    /// an `Rc<dyn Fn()>` (`EventHandler`), compared by identity: the handler given to
    /// `remove_handler` is the one given to `add_handler`.
    pub fn new(
        add_handler: impl Fn(Rc<dyn Fn()>) + 'static,
        remove_handler: impl Fn(Rc<dyn Fn()>) + 'static,
        getter: impl Fn() -> T + 'static,
        distinct_until_changed: bool,
    ) -> Self {
        Self { add_handler: Rc::new(add_handler), remove_handler: Rc::new(remove_handler), getter: Rc::new(getter), distinct_until_changed }
    }
}

impl<T: Clone + PartialEq + 'static> IObservable<T> for EventObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let subscription = Rc::new(Subscription {
            remove_handler: self.remove_handler.clone(),
            getter: self.getter.clone(),
            distinct_until_changed: self.distinct_until_changed,
            observer: RefCell::new(Some(observer.clone())),
            handler: RefCell::new(None),
            last_value: RefCell::new(None),
        });
        // The handler holds the subscription, as the event source holds a .NET handler; disposing
        // the subscription releases the handler, and with it the cycle.
        let target = subscription.clone();
        let handler: EventHandler = Rc::new(move || target.on_value_changed());
        *subscription.handler.borrow_mut() = Some(handler.clone());
        (self.add_handler)(handler);

        // A panic of the initial read or delivery detaches the handler before it propagates, as the
        // caller never receives the subscription.
        let mut guard = DisposeOnUnwind(Some(subscription.clone()));
        let initial = (self.getter)();
        *subscription.last_value.borrow_mut() = Some(initial.clone());
        observer.on_next(initial);
        guard.0 = None;
        subscription
    }
}

impl<T: 'static> fmt::Debug for EventObservable<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EventObservable: DistinctUntilChanged = {}", self.distinct_until_changed)
    }
}

/// The event subscription of one observer.
struct Subscription<T: 'static> {
    remove_handler: Rc<dyn Fn(EventHandler)>,
    getter: Rc<dyn Fn() -> T>,
    distinct_until_changed: bool,
    /// The observer, cleared on disposal.
    observer: RefCell<Option<Rc<dyn IObserver<T>>>>,
    /// The handler given to the event, cleared on disposal.
    handler: RefCell<Option<EventHandler>>,
    /// The last value delivered, for the distinct filtering.
    last_value: RefCell<Option<T>>,
}

impl<T: Clone + PartialEq + 'static> Subscription<T> {
    /// Delivers the current value when the event is raised, unless the subscription is disposed or
    /// the value repeats the last one under the distinct filtering (`OnValueChanged`).
    fn on_value_changed(&self) {
        let Some(observer) = self.observer.borrow().clone() else {
            return;
        };
        let value = (self.getter)();
        if self.distinct_until_changed && self.last_value.borrow().as_ref() == Some(&value) {
            return;
        }
        *self.last_value.borrow_mut() = Some(value.clone());
        observer.on_next(value);
    }
}

impl<T: 'static> IDisposable for Subscription<T> {
    /// Unsubscribes the handler on the first call; later calls do nothing.
    fn dispose(&self) {
        if self.observer.borrow_mut().take().is_none() {
            return;
        }
        let handler = self.handler.borrow_mut().take();
        if let Some(handler) = handler {
            (self.remove_handler)(handler);
        }
    }
}

/// Disposes the subscription when it is dropped by a panic of its start; emptied once it started.
struct DisposeOnUnwind<T: 'static>(Option<Rc<Subscription<T>>>);

impl<T: 'static> Drop for DisposeOnUnwind<T> {
    fn drop(&mut self) {
        if let Some(subscription) = self.0.take() {
            subscription.dispose();
        }
    }
}
