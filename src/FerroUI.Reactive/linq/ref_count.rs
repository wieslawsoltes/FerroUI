//! `RefCount` (`Linq/Observable/RefCount.cs`): a connectable observable connected while it has
//! observers, with the forms of Rx 6.0 (the minimum number of observers before the first connect,
//! and the delayed disconnect that a new observer cancels).

use crate::concurrency::{IScheduler, SchedulerExt};
use crate::disposables::{Disposable, SingleAssignmentDisposable, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use crate::linq::creation::IdentitySink;
use crate::subjects::IConnectableObservable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// Panics when `min_observers` is zero (`ArgumentOutOfRangeException` of `minObservers`).
pub(crate) fn check_min_observers(min_observers: usize) {
    assert!(min_observers > 0, "Specified argument was out of the range of valid values. (Parameter 'minObservers')");
}

/// The state of one connection (`RefCount<TSource>.Eager.RefConnection`): the number of observers
/// and the connection.
struct RefConnection {
    count: Cell<usize>,
    disposable: SingleAssignmentDisposableValue,
}

/// The state that every subscription of an eager `RefCount` shares (`RefCount<TSource>.Eager`).
struct EagerState<T> {
    source: Rc<dyn IConnectableObservable<T>>,
    /// The active connection, if any (`_connection`).
    connection: RefCell<Option<Rc<RefConnection>>>,
    min_observers: usize,
}

/// `RefCount<TSource>.Eager`: connects when the number of observers reaches the minimum, and
/// disconnects when it drops to zero.
struct EagerRefCount<T> {
    state: Rc<EagerState<T>>,
}

/// The sink of an eager `RefCount` (`RefCount<TSource>.Eager._`, an `IdentitySink`).
struct EagerSink<T> {
    sink: Sink<T>,
    parent: Rc<EagerState<T>>,
    /// The connection the observer subscribed with (`_targetConnection`).
    target_connection: RefCell<Option<Rc<RefConnection>>>,
}

impl<T: 'static> EagerSink<T> {
    fn run(self: &Rc<Self>) {
        let parent = self.parent.clone();
        let connection = parent.connection.borrow_mut().get_or_insert_with(|| Rc::new(RefConnection { count: Cell::new(0), disposable: SingleAssignmentDisposableValue::new() })).clone();

        // Connect if this is the first time the number of observers reaches the minimum since the
        // last time there were none.
        let count = connection.count.get() + 1;
        connection.count.set(count);
        let do_connect = count == parent.min_observers && connection.disposable.disposable().is_none() && !connection.disposable.is_disposed();

        *self.target_connection.borrow_mut() = Some(connection.clone());

        // Subscribe to the source first, then connect if necessary.
        let upstream = parent.source.subscribe(self.clone());
        self.sink.set_upstream(upstream);

        if do_connect && !connection.disposable.is_disposed() {
            // A connection that ends synchronously affects only the connection known here.
            connection.disposable.set_disposable(Some(parent.source.connect()));
        }
    }
}

impl<T: 'static> IObserver<T> for EagerSink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

impl<T: 'static> IDisposable for EagerSink<T> {
    /// Disposes the sink, then disconnects when it was the last observer of the active connection
    /// (`Dispose(bool)`).
    fn dispose(&self) {
        if !self.sink.dispose() {
            return;
        }
        let Some(target_connection) = self.target_connection.borrow_mut().take() else {
            return;
        };
        {
            let mut current = self.parent.connection.borrow_mut();
            if !current.as_ref().is_some_and(|connection| Rc::ptr_eq(connection, &target_connection)) {
                return;
            }
            let count = target_connection.count.get() - 1;
            target_connection.count.set(count);
            if count != 0 {
                return;
            }
            *current = None;
        }
        target_connection.disposable.dispose();
    }
}

impl<T: 'static> IObservable<T> for EagerRefCount<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let parent = self.state.clone();
        subscribe_sink(
            observer,
            move |observer| Rc::new(EagerSink { sink: Sink::new(observer), parent, target_connection: RefCell::new(None) }),
            |sink: &Rc<EagerSink<T>>| sink.run(),
        )
    }
}

/// `RefCount(IConnectableObservable<TSource>, int)` (`RefCount<TSource>.Eager`).
pub(crate) fn ref_count<T: 'static>(source: Rc<dyn IConnectableObservable<T>>, min_observers: usize) -> Rc<dyn IObservable<T>> {
    check_min_observers(min_observers);
    Rc::new(EagerRefCount { state: Rc::new(EagerState { source, connection: RefCell::new(None), min_observers }) })
}

/// The state of a lazy `RefCount`, shared by all its subscriptions (`RefCount<TSource>.Lazy.State`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    /// Disconnected with no observers: the initial state, and the state after the delay of the
    /// disconnect elapsed with no new observer.
    DisconnectedNoSubscribers,
    /// Disconnected with fewer observers than the minimum.
    DisconnectedWithSubscribers,
    /// Connected with at least one observer.
    ConnectedWithSubscribers,
    /// Connected with no observers, waiting for the delay of the disconnect or a new observer.
    ConnectedWithNoSubscribers,
}

/// `RefCount<TSource>.Lazy`: connects when the number of observers reaches the minimum, and
/// disconnects `disconnect_time` after it drops to zero unless an observer comes meanwhile.
struct LazyState<T> {
    source: Rc<dyn IConnectableObservable<T>>,
    scheduler: Rc<dyn IScheduler>,
    disconnect_time: Duration,
    min_observers: usize,
    state: Cell<State>,
    /// The slot of the delayed disconnect (`_serial`): replacing it cancels the one scheduled.
    serial: RefCell<Option<Rc<SingleAssignmentDisposable>>>,
    count: Cell<usize>,
    connectable_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl<T: 'static> LazyState<T> {
    /// Runs a sink (`RefCount<TSource>.Lazy._.Run`).
    fn run(self: &Rc<Self>, sink: &Rc<IdentitySink<T>>) {
        // The source may complete synchronously: the sink may be disposed from here on.
        let subscription = self.source.subscribe(sink.clone());

        let observer_count = self.count.get() + 1;
        self.count.set(observer_count);
        let mut should_connect = false;
        let mut should_cancel_delayed_disconnect = false;
        match self.state.get() {
            State::DisconnectedNoSubscribers | State::DisconnectedWithSubscribers => {
                debug_assert!(observer_count <= self.min_observers, "RefCount should never exceed minObservers without already having connected");
                should_connect = observer_count == self.min_observers;
                self.state.set(if should_connect { State::ConnectedWithSubscribers } else { State::DisconnectedWithSubscribers });
            }
            State::ConnectedWithSubscribers => {}
            State::ConnectedWithNoSubscribers => {
                should_cancel_delayed_disconnect = true;
                self.state.set(State::ConnectedWithSubscribers);
            }
        }

        if should_connect {
            debug_assert!(self.connectable_subscription.borrow().is_none(), "RefCount already connected when it should not be");
            // Connecting may complete the observers subscribed already; their count includes this
            // one, whose subscription is not registered yet, so the state stays connected.
            let connection = self.source.connect();
            *self.connectable_subscription.borrow_mut() = Some(connection);
            debug_assert!(self.state.get() == State::ConnectedWithSubscribers, "Unexpected state change in Connect");
        }

        if should_connect || should_cancel_delayed_disconnect {
            // Cancels a delayed disconnect scheduled already, and leaves an unused slot for the
            // next one.
            let previous = self.serial.replace(Some(SingleAssignmentDisposable::new()));
            if let Some(previous) = previous {
                previous.dispose();
            }
        }

        let parent = self.clone();
        sink.sink.set_upstream(Disposable::create(move || {
            subscription.dispose();
            parent.on_observer_disposed();
        }));
    }

    /// The end of a subscription: the last observer schedules the disconnect when connected.
    fn on_observer_disposed(self: &Rc<Self>) {
        let count = self.count.get() - 1;
        self.count.set(count);
        if count != 0 {
            return;
        }
        // The count may drop to zero without the minimum ever being reached, or inside the call to
        // connect when the source completes there: only a connected operator schedules the
        // disconnect.
        if self.state.get() == State::ConnectedWithSubscribers {
            self.state.set(State::ConnectedWithNoSubscribers);

            let cancelable = self.serial.borrow().clone().expect("the slot of the delayed disconnect is set when connected");
            let (parent, target) = (self.clone(), cancelable.clone());
            let work = self.scheduler.schedule_action_after(self.disconnect_time, move || {
                let current = parent.serial.borrow().clone();
                if current.is_some_and(|current| Rc::ptr_eq(&current, &target)) {
                    parent.state.set(State::DisconnectedNoSubscribers);
                    let connectable_subscription = parent.connectable_subscription.borrow_mut().take();
                    if let Some(connectable_subscription) = connectable_subscription {
                        connectable_subscription.dispose();
                    }
                }
            });
            cancelable.set_disposable(Some(work));
        } else {
            debug_assert!(self.state.get() == State::DisconnectedWithSubscribers);
            self.state.set(State::DisconnectedNoSubscribers);
        }
    }
}

/// `RefCount<TSource>.Lazy` as an observable.
struct LazyRefCount<T> {
    state: Rc<LazyState<T>>,
}

impl<T: 'static> IObservable<T> for LazyRefCount<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let parent = self.state.clone();
        subscribe_sink(observer, IdentitySink::new, move |sink: &Rc<IdentitySink<T>>| parent.run(sink))
    }
}

/// `RefCount(IConnectableObservable<TSource>, int, TimeSpan, IScheduler)` (`RefCount<TSource>.Lazy`).
pub(crate) fn ref_count_with_delay<T: 'static>(
    source: Rc<dyn IConnectableObservable<T>>,
    min_observers: usize,
    disconnect_time: Duration,
    scheduler: Rc<dyn IScheduler>,
) -> Rc<dyn IObservable<T>> {
    check_min_observers(min_observers);
    Rc::new(LazyRefCount {
        state: Rc::new(LazyState {
            source,
            scheduler,
            disconnect_time,
            min_observers,
            state: Cell::new(State::DisconnectedNoSubscribers),
            serial: RefCell::new(None),
            count: Cell::new(0),
            connectable_subscription: RefCell::new(None),
        }),
    })
}

/// The methods of `RefCount` (provided methods of
/// [`ConnectableObservableExt`](super::multicast::ConnectableObservableExt)).
///
/// `_min` names the number of observers that connects (1 by default; panics when zero), and
/// `_with_delay` the delay of the disconnect after the last observer leaves, on the default
/// scheduler unless `_on` names one.
macro_rules! ref_count_methods {
    () => {
        /// A sequence that connects when it gets its first observer and disconnects when the last
        /// one leaves (`RefCount()`).
        fn ref_count(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::ref_count::ref_count(self.connectable(), 1)
        }

        /// [`ref_count`](Self::ref_count) that connects when it has `min_observers` observers
        /// (`RefCount(int)`). Panics when `min_observers` is zero.
        fn ref_count_min(&self, min_observers: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::ref_count::ref_count(self.connectable(), min_observers)
        }

        /// [`ref_count`](Self::ref_count) that disconnects `disconnect_delay` after the last
        /// observer leaves, unless an observer comes meanwhile, on the default scheduler
        /// (`RefCount(TimeSpan)`).
        fn ref_count_with_delay(&self, disconnect_delay: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::ref_count::ref_count_with_delay(
                self.connectable(),
                1,
                disconnect_delay,
                crate::concurrency::SchedulerDefaults::default_scheduler(),
            )
        }

        /// [`ref_count_with_delay`](Self::ref_count_with_delay) on `scheduler`
        /// (`RefCount(TimeSpan, IScheduler)`).
        fn ref_count_with_delay_on(
            &self,
            disconnect_delay: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::ref_count::ref_count_with_delay(self.connectable(), 1, disconnect_delay, scheduler)
        }

        /// [`ref_count_with_delay`](Self::ref_count_with_delay) that connects when it has
        /// `min_observers` observers (`RefCount(int, TimeSpan)`). Panics when `min_observers` is
        /// zero.
        fn ref_count_min_with_delay(&self, min_observers: usize, disconnect_delay: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::ref_count::ref_count_with_delay(
                self.connectable(),
                min_observers,
                disconnect_delay,
                crate::concurrency::SchedulerDefaults::default_scheduler(),
            )
        }

        /// [`ref_count_min_with_delay`](Self::ref_count_min_with_delay) on `scheduler`
        /// (`RefCount(int, TimeSpan, IScheduler)`). Panics when `min_observers` is zero.
        fn ref_count_min_with_delay_on(
            &self,
            min_observers: usize,
            disconnect_delay: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::ref_count::ref_count_with_delay(self.connectable(), min_observers, disconnect_delay, scheduler)
        }
    };
}
pub(crate) use ref_count_methods;
