//! `ObservableAsPropertyHelper<T>`.

use crate::RxState;
use ferroui_reactive::concurrency::{IScheduler, ImmediateScheduler, SchedulerExt};
use ferroui_reactive::disposables::Disposable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// A callback of the helper, run with each new value.
type Callback<T> = Rc<dyn Fn(T)>;

/// The initial value of an [`ObservableAsPropertyHelper`]: a value, or a factory read when the
/// helper starts (upstream's `T? initialValue` and `Func<T?> getInitialValue` overloads).
pub enum ObservableAsPropertyInitialValue<T> {
    /// The value itself.
    Value(T),
    /// A factory run when the helper starts, or on the first read of a deferred helper.
    Factory(Rc<dyn Fn() -> T>),
}

impl<T: Clone> Clone for ObservableAsPropertyInitialValue<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Value(value) => Self::Value(value.clone()),
            Self::Factory(factory) => Self::Factory(factory.clone()),
        }
    }
}

impl<T: Clone> ObservableAsPropertyInitialValue<T> {
    /// A factory of the initial value.
    pub fn factory(factory: impl Fn() -> T + 'static) -> Self {
        Self::Factory(Rc::new(factory))
    }

    /// Reads the initial value (`InitialValue.Read`).
    fn read(&self) -> T {
        match self {
            Self::Value(value) => value.clone(),
            Self::Factory(factory) => factory(),
        }
    }
}

impl<T: Default> Default for ObservableAsPropertyInitialValue<T> {
    fn default() -> Self {
        Self::Value(T::default())
    }
}

/// Backs a read-only output property with an observable: the property reports each new value
/// through the callbacks it was created with, so that its owner raises its own change
/// notifications (`ObservableAsPropertyHelper<T>`).
///
/// It is normally made by [`to_property`](crate::binding::mixins::ToPropertyMixins::to_property),
/// whose callbacks raise the owner's notifications for the named property; it can also be made
/// directly with callbacks of one's own. A view model stores it (in a `OnceCell` or a
/// `RefCell<Option<..>>`) and reads [`value`](Self::value) in its getter.
///
/// Values equal to the previous one are skipped. Without a scheduler, or with the immediate one,
/// each value is delivered at once; a value produced while another is being delivered waits for
/// that delivery to end, so the callbacks never nest.
///
/// The helper is a shared handle: its clones are the same helper.
// Deviation (DEVIATIONS.md, ReactiveUI): the subscription to the source refers to the helper
// weakly, so a helper that every handle has dropped stops (there is no garbage collector to
// collect the cycle through the source).
pub struct ObservableAsPropertyHelper<T: 'static> {
    core: Rc<Core<T>>,
}

impl<T: 'static> Clone for ObservableAsPropertyHelper<T> {
    fn clone(&self) -> Self {
        Self { core: self.core.clone() }
    }
}

impl<T: Clone + PartialEq + 'static> ObservableAsPropertyHelper<T> {
    /// A helper that follows `observable` from the default value of `T`, running `on_changed`
    /// with each new value (`ObservableAsPropertyHelper(IObservable<T?>, Action<T?>)`).
    pub fn new(observable: Rc<dyn IObservable<T>>, on_changed: impl Fn(T) + 'static) -> Self
    where
        T: Default,
    {
        Self::with_options(observable, on_changed, None, ObservableAsPropertyInitialValue::default(), false, None)
    }

    /// A helper that starts from `initial_value`
    /// (`ObservableAsPropertyHelper(IObservable<T?>, Action<T?>, T?)`).
    pub fn with_initial_value(observable: Rc<dyn IObservable<T>>, on_changed: impl Fn(T) + 'static, initial_value: T) -> Self {
        Self::with_options(observable, on_changed, None, ObservableAsPropertyInitialValue::Value(initial_value), false, None)
    }

    /// A helper that starts from `initial_value` and delivers its values on `scheduler`
    /// (`ObservableAsPropertyHelper(IObservable<T?>, Action<T?>, T?, ISequencer?)`).
    pub fn with_initial_value_on(
        observable: Rc<dyn IObservable<T>>,
        on_changed: impl Fn(T) + 'static,
        initial_value: T,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Self {
        Self::with_options(observable, on_changed, None, ObservableAsPropertyInitialValue::Value(initial_value), false, scheduler)
    }

    /// A helper that starts from `initial_value` and, when `defer_subscription` is set, subscribes
    /// to `observable` on the first read of [`value`](Self::value)
    /// (`ObservableAsPropertyHelper(IObservable<T?>, Action<T?>, T?, bool)`).
    pub fn with_initial_value_deferred(
        observable: Rc<dyn IObservable<T>>,
        on_changed: impl Fn(T) + 'static,
        initial_value: T,
        defer_subscription: bool,
    ) -> Self {
        Self::with_options(observable, on_changed, None, ObservableAsPropertyInitialValue::Value(initial_value), defer_subscription, None)
    }

    /// A helper with a callback run before each new value is stored, as well as the one run after
    /// (`ObservableAsPropertyHelper(IObservable<T?>, Action<T?>, Action<T?>?)`).
    pub fn with_changing(observable: Rc<dyn IObservable<T>>, on_changed: impl Fn(T) + 'static, on_changing: impl Fn(T) + 'static) -> Self
    where
        T: Default,
    {
        Self::with_options(observable, on_changed, Some(Rc::new(on_changing)), ObservableAsPropertyInitialValue::default(), false, None)
    }

    /// The helper with every option: `on_changing` runs before each new value is stored and
    /// `on_changed` after; `initial_value` is a value or a factory; `defer_subscription` delays the
    /// subscription to the first read of [`value`](Self::value); `scheduler` delivers the values
    /// (the other constructors of upstream, which this one covers).
    pub fn with_options(
        observable: Rc<dyn IObservable<T>>,
        on_changed: impl Fn(T) + 'static,
        on_changing: Option<Rc<dyn Fn(T)>>,
        initial_value: ObservableAsPropertyInitialValue<T>,
        defer_subscription: bool,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Self {
        Self::from_core(Core::new(observable, Rc::new(on_changed), on_changing, initial_value, scheduler), defer_subscription)
    }

    /// A helper whose callbacks take its owner, which the helper holds weakly: once the owner is
    /// dropped, the callbacks are not run (`Create(IObservable<T?>, object, Action<object?, T?>,
    /// Action<object?, T?>?, T?, bool, ISequencer?)`, the factory that `ToProperty` uses).
    pub fn create<O: 'static>(
        observable: Rc<dyn IObservable<T>>,
        owner: &Rc<O>,
        on_changed: impl Fn(&Rc<O>, T) + 'static,
        on_changing: Option<Rc<dyn Fn(&Rc<O>, T)>>,
        initial_value: ObservableAsPropertyInitialValue<T>,
        defer_subscription: bool,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Self {
        let changed_owner = Rc::downgrade(owner);
        let on_changed = move |value: T| {
            if let Some(owner) = changed_owner.upgrade() {
                on_changed(&owner, value);
            }
        };
        let on_changing = on_changing.map(|on_changing| {
            let changing_owner: Weak<O> = Rc::downgrade(owner);
            Rc::new(move |value: T| {
                if let Some(owner) = changing_owner.upgrade() {
                    on_changing(&owner, value);
                }
            }) as Rc<dyn Fn(T)>
        });
        Self::with_options(observable, on_changed, on_changing, initial_value, defer_subscription, scheduler)
    }

    /// A helper that holds `initial_value` and never changes (`Default(T?)`).
    pub fn default_with_initial_value(initial_value: T) -> Self {
        Self::default_with_initial_value_on(initial_value, None)
    }

    /// A helper that holds `initial_value`, delivered on `scheduler`, and never changes
    /// (`Default(T?, ISequencer?)`).
    pub fn default_with_initial_value_on(initial_value: T, scheduler: Option<Rc<dyn IScheduler>>) -> Self {
        Self::from_core(
            Core::new(
                ferroui_reactive::linq::Observable::never::<T>(),
                Rc::new(|_: T| {}),
                None,
                ObservableAsPropertyInitialValue::Value(initial_value),
                scheduler,
            ),
            false,
        )
    }

    fn from_core(core: Rc<Core<T>>, defer_subscription: bool) -> Self {
        if !defer_subscription {
            core.start();
        }
        Self { core }
    }

    /// The last value of the property; a deferred helper subscribes on its first read (`Value`).
    pub fn value(&self) -> T {
        self.core.read_value()
    }

    /// Whether the helper has subscribed to its source (`IsSubscribed`).
    pub fn is_subscribed(&self) -> bool {
        self.core.activated.get()
    }

    /// The errors of the source (`ThrownExceptions`). While it has no observer, an error goes to
    /// [`RxState::default_exception_handler`](crate::RxState).
    pub fn thrown_exceptions(&self) -> Rc<dyn IObservable<ObservableError>> {
        Rc::new(ExceptionStream { core: self.core.clone() })
    }
}

impl<T: Clone + PartialEq + Default + 'static> Default for ObservableAsPropertyHelper<T> {
    /// A helper that holds the default value of `T` and never changes (`Default()`).
    fn default() -> Self {
        Self::default_with_initial_value(T::default())
    }
}

impl<T: Clone + PartialEq + 'static> IDisposable for ObservableAsPropertyHelper<T> {
    /// Ends the subscription to the source and drops the values waiting for delivery (`Dispose`).
    fn dispose(&self) {
        self.core.dispose();
    }
}

/// The state and the source subscription behind a helper (`Core`).
struct Core<T: 'static> {
    source: Rc<dyn IObservable<T>>,
    on_changed: Callback<T>,
    on_changing: Option<Callback<T>>,
    initial_value: ObservableAsPropertyInitialValue<T>,
    scheduler: Option<Rc<dyn IScheduler>>,
    exceptions: RefCell<Vec<(u64, Rc<dyn IObserver<ObservableError>>)>>,
    next_exception_observer: Cell<u64>,
    pending: RefCell<VecDeque<T>>,
    distinct_previous: RefCell<Option<T>>,
    delivering: Cell<bool>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
    activated: Cell<bool>,
    last_value: RefCell<Option<T>>,
    this: Weak<Core<T>>,
}

/// The observer of the source, which refers to the helper weakly.
struct CoreObserver<T: 'static> {
    core: Weak<Core<T>>,
}

impl<T: Clone + PartialEq + 'static> IObserver<T> for CoreObserver<T> {
    fn on_next(&self, value: T) {
        if let Some(core) = self.core.upgrade() {
            core.on_next(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        if let Some(core) = self.core.upgrade() {
            core.on_error(error);
        }
    }

    fn on_completed(&self) {}
}

/// Clears the delivery state when a callback panics: a later value must not be delivered ahead of
/// the ones the failure stranded, so they are dropped (`DeliverSerialized`'s `catch`).
struct DeliveryGuard<'a, T: 'static> {
    core: &'a Core<T>,
    finished: bool,
}

impl<T: 'static> Drop for DeliveryGuard<'_, T> {
    fn drop(&mut self) {
        if !self.finished {
            self.core.delivering.set(false);
            self.core.pending.borrow_mut().clear();
        }
    }
}

impl<T: Clone + PartialEq + 'static> Core<T> {
    fn new(
        source: Rc<dyn IObservable<T>>,
        on_changed: Callback<T>,
        on_changing: Option<Callback<T>>,
        initial_value: ObservableAsPropertyInitialValue<T>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Rc<Self> {
        // The immediate scheduler delivers as no scheduler does, so it is dropped (`IsImmediate`).
        let scheduler = scheduler.filter(|scheduler| !Rc::ptr_eq(scheduler, &ImmediateScheduler::instance()));
        Rc::new_cyclic(|this| Self {
            source,
            on_changed,
            on_changing,
            initial_value,
            scheduler,
            exceptions: RefCell::new(Vec::new()),
            next_exception_observer: Cell::new(0),
            pending: RefCell::new(VecDeque::new()),
            distinct_previous: RefCell::new(None),
            delivering: Cell::new(false),
            subscription: RefCell::new(None),
            disposed: Cell::new(false),
            activated: Cell::new(false),
            last_value: RefCell::new(None),
            this: this.clone(),
        })
    }

    fn observer(&self) -> Rc<dyn IObserver<T>> {
        Rc::new(CoreObserver { core: self.this.clone() })
    }

    /// Announces the initial value and subscribes to the source (`Start`).
    fn start(&self) {
        let initial = self.initial_value.read();
        *self.last_value.borrow_mut() = Some(initial.clone());
        *self.distinct_previous.borrow_mut() = Some(initial.clone());
        self.activated.set(true);
        self.publish(initial);
        let subscription = self.source.subscribe(self.observer());
        self.store_subscription(subscription);
    }

    /// The last value; the first read of a deferred helper subscribes (`ReadValue`).
    fn read_value(&self) -> T {
        if !self.activated.get() {
            self.activated.set(true);
            self.activate();
        }
        let last = self.last_value.borrow().clone();
        match last {
            Some(value) => value,
            None => self.initial_value.read(),
        }
    }

    /// The deferred start: the initial value is held without being announced, so a source that
    /// opens by repeating it changes nothing (`Activate`).
    fn activate(&self) {
        let initial = self.initial_value.read();
        *self.last_value.borrow_mut() = Some(initial.clone());
        if self.disposed.get() {
            return;
        }
        *self.distinct_previous.borrow_mut() = Some(initial);
        let subscription = self.source.subscribe(self.observer());
        self.store_subscription(subscription);
    }

    fn store_subscription(&self, subscription: Rc<dyn IDisposable>) {
        if self.disposed.get() {
            subscription.dispose();
            return;
        }
        *self.subscription.borrow_mut() = Some(subscription);
    }

    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.pending.borrow_mut().clear();
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn on_next(&self, value: T) {
        if self.disposed.get() {
            return;
        }
        {
            let mut previous = self.distinct_previous.borrow_mut();
            if previous.as_ref() == Some(&value) {
                return;
            }
            *previous = Some(value.clone());
        }
        if self.scheduler.is_some() {
            self.schedule(value);
            return;
        }
        if self.try_claim_delivery(&value) {
            self.deliver_serialized(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        let observers: Vec<Rc<dyn IObserver<ObservableError>>> = self.exceptions.borrow().iter().map(|(_, observer)| observer.clone()).collect();
        if observers.is_empty() {
            // Deviation (DEVIATIONS.md, ReactiveUI): upstream rethrows the error; the port hands it
            // to the default exception handler of ReactiveUI, which panics by default.
            RxState::default_exception_handler().on_next(error);
            return;
        }
        for observer in observers {
            observer.on_next(error.clone());
        }
    }

    /// Delivers the initial value (`Publish`).
    fn publish(&self, value: T) {
        if self.scheduler.is_some() {
            self.schedule(value);
            return;
        }
        if self.try_claim_delivery(&value) {
            self.deliver_serialized(value);
        }
    }

    /// Claims the delivery, or queues `value` behind the delivery running (`TryClaimDelivery`).
    fn try_claim_delivery(&self, value: &T) -> bool {
        if !self.delivering.get() {
            self.delivering.set(true);
            return true;
        }
        self.pending.borrow_mut().push_back(value.clone());
        false
    }

    /// Delivers `value`, then the values queued while it was delivered (`DeliverSerialized`).
    fn deliver_serialized(&self, value: T) {
        let mut guard = DeliveryGuard { core: self, finished: false };
        let mut value = value;
        loop {
            self.deliver(value);
            let next = if self.disposed.get() { None } else { self.pending.borrow_mut().pop_front() };
            match next {
                Some(next) => value = next,
                None => {
                    self.delivering.set(false);
                    guard.finished = true;
                    return;
                }
            }
        }
    }

    fn schedule(&self, value: T) {
        let Some(scheduler) = self.scheduler.clone() else {
            return;
        };
        let Some(core) = self.this.upgrade() else {
            return;
        };
        let _ = scheduler.schedule_action(move || core.deliver(value));
    }

    /// Runs the callbacks around storing the new value (`Deliver`).
    fn deliver(&self, value: T) {
        if let Some(on_changing) = &self.on_changing {
            on_changing(value.clone());
        }
        *self.last_value.borrow_mut() = Some(value.clone());
        (self.on_changed)(value);
    }
}

/// The errors of the source of a helper (`ExceptionStream`).
struct ExceptionStream<T: 'static> {
    core: Rc<Core<T>>,
}

impl<T: 'static> IObservable<ObservableError> for ExceptionStream<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<ObservableError>>) -> Rc<dyn IDisposable> {
        let id = self.core.next_exception_observer.get();
        self.core.next_exception_observer.set(id + 1);
        self.core.exceptions.borrow_mut().push((id, observer));
        let core = Rc::downgrade(&self.core);
        Disposable::create(move || {
            if let Some(core) = core.upgrade() {
                core.exceptions.borrow_mut().retain(|(observer_id, _)| *observer_id != id);
            }
        })
    }
}
