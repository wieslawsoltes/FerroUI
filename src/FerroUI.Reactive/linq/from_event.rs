//! `FromEvent` (`Linq/Observable/FromEvent.cs`), with the producer of the sequences of events that
//! it shares with `FromEventPattern` (`EventProducer<TDelegate, TArgs>`,
//! `ClassicEventProducer<TDelegate, TArgs>`), and its overloads of `QueryLanguage.Events.cs`.
//!
//! The events are those of FerroUI (`ferroui_system::events::Event`): `add_handler` adds a handler
//! and returns the token of the event that `remove_handler` takes. With an `Event<A>`, whose
//! handlers take the arguments by reference:
//!
//! ```ignore
//! let xs = Observable::from_event(
//!     move |handler: Rc<dyn Fn(i32)>| event.add(Rc::new(move |x: &i32| handler(*x))),
//!     move |token| { event.remove(token); },
//! );
//! ```
//!
//! The overloads without a scheduler add and remove the handler on the scheduler of upstream's
//! `GetSchedulerForCurrentContext`, taken when the sequence is made: a
//! [`SynchronizationContextScheduler`] of the current `SynchronizationContext` of the thread that
//! runs its work at once when that context is current, else the immediate scheduler.
//!
//! The overloads of upstream that make the delegate of the event from the handler through
//! reflection (`FromEvent<TDelegate, TEventArgs>(Action<TDelegate>, Action<TDelegate>)`) are not
//! ported: the handler of the port is a closure, which the conversion overloads turn into the
//! handler type of an event.

use crate::concurrency::{IScheduler, SchedulerDefaults, SchedulerExt, SynchronizationContextScheduler};
use crate::disposables::{Disposable, SingleAssignmentDisposable};
use crate::internal::subscribe_basic;
use crate::linq::Observable;
use crate::subjects::Subject;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Makes the handler of the event from the action that notifies its arguments (`GetHandler`); a
/// conversion that fails ends the subscription with its error.
pub(crate) type GetHandler<D, T> = Rc<dyn Fn(Rc<dyn Fn(T)>) -> Result<D, ObservableError>>;

/// Adds the handler to the event and returns the disposable that removes it (`AddHandler`).
pub(crate) type AddHandler<D> = Rc<dyn Fn(D) -> Rc<dyn IDisposable>>;

/// The addition of `ClassicEventProducer<TDelegate, TArgs>`: `add` adds the handler and returns its
/// token, which `remove` takes when the disposable is disposed.
pub(crate) fn classic_add_handler<D: 'static, K: 'static>(add: impl Fn(D) -> K + 'static, remove: impl Fn(K) + 'static) -> AddHandler<D> {
    let remove = Rc::new(remove);
    Rc::new(move |handler: D| {
        let token = add(handler);
        let remove = remove.clone();
        Disposable::create(move || remove(token))
    })
}

/// The scheduler of the overloads without one (`GetSchedulerForCurrentContext`): that of the
/// current synchronization context of the thread, which does not post when the context is current
/// (`alwaysPost: false`), else the immediate scheduler (`ConstantTimeOperations`).
pub(crate) fn scheduler_for_current_context() -> Rc<dyn IScheduler> {
    match ferroui_system::threading::current_synchronization_context() {
        Some(context) => SynchronizationContextScheduler::with_always_post(context, false),
        None => SchedulerDefaults::constant_time_operations(),
    }
}

/// The state of an `EventProducer<TDelegate, TArgs>` that its sessions reach.
struct ProducerState<D, T> {
    scheduler: Rc<dyn IScheduler>,
    get_handler: GetHandler<D, T>,
    add_handler: AddHandler<D>,
    session: RefCell<Option<Rc<Session<D, T>>>>,
}

/// `EventProducer<TDelegate, TArgs>` (a `BasicProducer<TArgs>`): the subscriptions share one
/// handler of the event (a session), which feeds a subject. The handler is added on the scheduler
/// at the first subscription, and removed on it when the last subscription is disposed; a later
/// subscription starts a new session.
struct EventProducer<D, T> {
    state: Rc<ProducerState<D, T>>,
}

impl<D: 'static, T: Clone + 'static> IObservable<T> for EventProducer<D, T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let state = self.state.clone();
        // `Run`.
        subscribe_basic(observer, move |observer| {
            let session = {
                let mut slot = state.session.borrow_mut();
                slot.get_or_insert_with(|| Rc::new(Session::new(state.clone()))).clone()
            };
            session.connect(observer)
        })
    }
}

/// `EventProducer<TDelegate, TArgs>.Session`: one handler of the event, feeding a subject, and the
/// count of the subscriptions to the subject.
struct Session<D, T> {
    parent: Rc<ProducerState<D, T>>,
    subject: Rc<Subject<T>>,
    remove_handler: Rc<SingleAssignmentDisposable>,
    count: Cell<usize>,
}

impl<D: 'static, T: Clone + 'static> Session<D, T> {
    fn new(parent: Rc<ProducerState<D, T>>) -> Self {
        Self { parent, subject: Subject::new(), remove_handler: SingleAssignmentDisposable::new(), count: Cell::new(0) }
    }

    /// `Connect`: the observer is subscribed to the subject before the handler is added, so that no
    /// event is missed between the two (with the immediate scheduler).
    fn connect(self: &Rc<Self>, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let connection = self.subject.subscribe(observer.clone());

        let count = self.count.get() + 1;
        self.count.set(count);
        if count == 1 {
            if let Err(error) = self.initialize() {
                self.count.set(self.count.get() - 1);
                connection.dispose();
                observer.on_error(error);
                return Disposable::empty();
            }
        }

        let session = self.clone();
        Disposable::create(move || {
            connection.dispose();
            let count = session.count.get() - 1;
            session.count.set(count);
            if count == 0 {
                let remove_handler = session.remove_handler.clone();
                session.parent.scheduler.schedule_action(move || remove_handler.dispose());
                *session.parent.session.borrow_mut() = None;
            }
        })
    }

    /// `Initialize`: the handler is made at once (the conversion is a pure function), and added on
    /// the scheduler.
    fn initialize(self: &Rc<Self>) -> Result<(), ObservableError> {
        let subject = self.subject.clone();
        let on_next = (self.parent.get_handler)(Rc::new(move |value: T| subject.on_next(value)))?;
        let session = self.clone();
        self.parent.scheduler.schedule_action(move || session.add_handler(on_next));
        Ok(())
    }

    /// `AddHandler`: the disposable that removes the handler is kept for the end of the session.
    fn add_handler(&self, on_next: D) {
        let remove_handler = (self.parent.add_handler)(on_next);
        self.remove_handler.set_disposable(Some(remove_handler));
    }
}

/// The sequence of the events that `add_handler` subscribes a handler to (an `EventProducer`).
pub(crate) fn event_producer<D: 'static, T: Clone + 'static>(
    get_handler: GetHandler<D, T>,
    add_handler: AddHandler<D>,
    scheduler: Rc<dyn IScheduler>,
) -> Rc<dyn IObservable<T>> {
    Rc::new(EventProducer { state: Rc::new(ProducerState { scheduler, get_handler, add_handler, session: RefCell::new(None) }) })
}

impl Observable {
    /// The arguments of the events of an event whose handlers take them
    /// (`FromEvent<TEventArgs>(Action<Action<TEventArgs>>, Action<Action<TEventArgs>>)`):
    /// `add_handler` adds a handler and returns its token, `remove_handler` removes the handler of a
    /// token. The subscriptions share one handler, added at the first subscription and removed when
    /// the last one is disposed.
    pub fn from_event<T: Clone + 'static, K: 'static>(
        add_handler: impl Fn(Rc<dyn Fn(T)>) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Self::from_event_on(add_handler, remove_handler, scheduler_for_current_context())
    }

    /// [`from_event`](Self::from_event), which adds and removes the handler on `scheduler`
    /// (`FromEvent<TEventArgs>(Action<Action<TEventArgs>>, Action<Action<TEventArgs>>, IScheduler)`).
    pub fn from_event_on<T: Clone + 'static, K: 'static>(
        add_handler: impl Fn(Rc<dyn Fn(T)>) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        event_producer(
            Rc::new(|handler: Rc<dyn Fn(T)>| Ok::<_, ObservableError>(handler)),
            classic_add_handler(add_handler, remove_handler),
            scheduler,
        )
    }

    /// The arguments of the events of an event whose handlers are of the type `D`, which
    /// `conversion` makes from the action that notifies the arguments
    /// (`FromEvent<TDelegate, TEventArgs>(Func<Action<TEventArgs>, TDelegate>, Action<TDelegate>, Action<TDelegate>)`).
    pub fn from_event_conversion<D: 'static, T: Clone + 'static, K: 'static>(
        conversion: impl Fn(Rc<dyn Fn(T)>) -> D + 'static,
        add_handler: impl Fn(D) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Self::from_event_conversion_on(conversion, add_handler, remove_handler, scheduler_for_current_context())
    }

    /// [`from_event_conversion`](Self::from_event_conversion) on `scheduler`
    /// (`FromEvent<TDelegate, TEventArgs>(Func<Action<TEventArgs>, TDelegate>, Action<TDelegate>, Action<TDelegate>, IScheduler)`).
    pub fn from_event_conversion_on<D: 'static, T: Clone + 'static, K: 'static>(
        conversion: impl Fn(Rc<dyn Fn(T)>) -> D + 'static,
        add_handler: impl Fn(D) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        event_producer(
            Rc::new(move |handler: Rc<dyn Fn(T)>| Ok::<_, ObservableError>(conversion(handler))),
            classic_add_handler(add_handler, remove_handler),
            scheduler,
        )
    }

    /// [`from_event_conversion`](Self::from_event_conversion) with a conversion that may fail; a
    /// failure ends the subscription with its error (a conversion that throws).
    pub fn try_from_event_conversion<D: 'static, T: Clone + 'static, K: 'static>(
        conversion: impl Fn(Rc<dyn Fn(T)>) -> Result<D, ObservableError> + 'static,
        add_handler: impl Fn(D) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Self::try_from_event_conversion_on(conversion, add_handler, remove_handler, scheduler_for_current_context())
    }

    /// [`try_from_event_conversion`](Self::try_from_event_conversion) on `scheduler`.
    pub fn try_from_event_conversion_on<D: 'static, T: Clone + 'static, K: 'static>(
        conversion: impl Fn(Rc<dyn Fn(T)>) -> Result<D, ObservableError> + 'static,
        add_handler: impl Fn(D) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        event_producer(Rc::new(conversion), classic_add_handler(add_handler, remove_handler), scheduler)
    }

    /// The events of an event whose handlers take no arguments, as `()`s
    /// (`FromEvent(Action<Action>, Action<Action>)`).
    pub fn from_event_action<K: 'static>(
        add_handler: impl Fn(Rc<dyn Fn()>) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
    ) -> Rc<dyn IObservable<()>> {
        Self::from_event_action_on(add_handler, remove_handler, scheduler_for_current_context())
    }

    /// [`from_event_action`](Self::from_event_action) on `scheduler`
    /// (`FromEvent(Action<Action>, Action<Action>, IScheduler)`).
    pub fn from_event_action_on<K: 'static>(
        add_handler: impl Fn(Rc<dyn Fn()>) -> K + 'static,
        remove_handler: impl Fn(K) + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<()>> {
        Self::from_event_conversion_on(
            |handler: Rc<dyn Fn(())>| -> Rc<dyn Fn()> { Rc::new(move || handler(())) },
            add_handler,
            remove_handler,
            scheduler,
        )
    }
}
