//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/RefCountTest.cs` of Rx.NET.
//! Adapted: the test helpers `MySubject.cs` and `ConnectableObservable.cs`, and the connectable
//! observables `SerialSingleNotificationConnectable<T>` and `SerialConnectableIgnoringConnect<T>`
//! of the upstream file, are ported below. The overloads are `ref_count`, `ref_count_min` (with
//! `minObservers`), `ref_count_with_delay` and `ref_count_min_with_delay` (with a disconnect delay),
//! with `_on` for the scheduler. The forms whose delayed disconnect runs on `Scheduler.Default`
//! (the thread pool upstream, after the test is over) run with the default scheduler of the thread
//! set to a test scheduler that is never started ([`DefaultSchedulerGuard`]): the current-thread
//! scheduler would block the test for the delay. `using` blocks dispose their subscription at the
//! end of the block. The data rows of a test run in the one test function.

use ferroui_reactive::concurrency::{Scheduler, SchedulerDefaults};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::linq::ConnectableObservableExt;
use ferroui_reactive::subjects::{BehaviorSubject, IConnectableObservable, ISubjectHandle, ReplaySubject, Subject};
use ferroui_reactive::{IDisposable, IObservable, IObserver, Notification, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ticks_to_duration, MockObserver, ReactiveTest as RT, TestScheduler};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

/// A subject that disconnects a connection when it receives a given value (`MySubject` of the
/// upstream tests).
struct MySubject {
    dispose_on: RefCell<HashMap<i32, Rc<dyn IDisposable>>>,
    observers: Rc<RefCell<Vec<Rc<dyn IObserver<i32>>>>>,
    subscribe_count: Cell<i32>,
    disposed: Rc<Cell<bool>>,
}

impl MySubject {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            dispose_on: RefCell::new(HashMap::new()),
            observers: Rc::new(RefCell::new(Vec::new())),
            subscribe_count: Cell::new(0),
            disposed: Rc::new(Cell::new(false)),
        })
    }

    /// `DisposeOn`.
    #[allow(dead_code)]
    fn dispose_on(&self, value: i32, disposable: Rc<dyn IDisposable>) {
        self.dispose_on.borrow_mut().insert(value, disposable);
    }

    /// `SubscribeCount`.
    fn subscribe_count(&self) -> i32 {
        self.subscribe_count.get()
    }

    /// `Disposed`.
    fn disposed(&self) -> bool {
        self.disposed.get()
    }
}

impl IObserver<i32> for MySubject {
    fn on_next(&self, value: i32) {
        let observers = self.observers.borrow().clone();
        for observer in observers {
            observer.on_next(value);
        }

        let disconnect = self.dispose_on.borrow().get(&value).cloned();
        if let Some(disconnect) = disconnect {
            disconnect.dispose();
        }
    }

    fn on_error(&self, error: ObservableError) {
        let observers = self.observers.borrow().clone();
        for observer in observers {
            observer.on_error(error.clone());
        }
    }

    fn on_completed(&self) {
        let observers = self.observers.borrow().clone();
        for observer in observers {
            observer.on_completed();
        }
    }
}

impl IObservable<i32> for MySubject {
    fn subscribe(&self, observer: Rc<dyn IObserver<i32>>) -> Rc<dyn IDisposable> {
        self.subscribe_count.set(self.subscribe_count.get() + 1);
        self.observers.borrow_mut().push(observer.clone());
        let (observers, disposed) = (self.observers.clone(), self.disposed.clone());
        Disposable::create(move || {
            {
                let mut observers = observers.borrow_mut();
                if let Some(index) = observers.iter().position(|o| Rc::ptr_eq(o, &observer)) {
                    observers.remove(index);
                }
            }
            disposed.set(true);
        })
    }
}

/// A connectable observable over `o.Multicast(s)` (`ConnectableObservable<T>` of the upstream
/// tests).
struct ConnectableObservable<T> {
    o: Rc<dyn IConnectableObservable<T>>,
}

impl<T: 'static> ConnectableObservable<T> {
    fn new(o: Rc<dyn IObservable<T>>, s: Rc<dyn ISubjectHandle<T, T>>) -> Rc<Self> {
        Rc::new(Self { o: o.multicast(s) })
    }
}

impl<T: 'static> IConnectableObservable<T> for ConnectableObservable<T> {
    fn connect(&self) -> Rc<dyn IDisposable> {
        self.o.connect()
    }
}

impl<T: 'static> IObservable<T> for ConnectableObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        self.o.subscribe(observer)
    }
}

/// A connection of [`SerialSingleNotificationConnectable`] (`Connection`).
struct SerialConnection<T> {
    disposed: Cell<bool>,
    source: RefCell<Rc<dyn IObservable<T>>>,
    /// The subject the source is, until [`replace_source`](Self::replace_source) replaces it.
    subject: RefCell<Option<Rc<Subject<T>>>>,
}

impl<T: Clone + 'static> SerialConnection<T> {
    fn new(subject: Rc<Subject<T>>) -> Rc<Self> {
        Rc::new(Self { disposed: Cell::new(false), source: RefCell::new(subject.as_observable()), subject: RefCell::new(Some(subject)) })
    }

    /// `Disposed`.
    fn disposed(&self) -> bool {
        self.disposed.get()
    }

    /// `ReplaceSource`: the source further observers of this connection subscribe to.
    fn replace_source(&self, source: Rc<dyn IObservable<T>>) {
        *self.source.borrow_mut() = source;
        *self.subject.borrow_mut() = None;
    }
}

impl<T> IDisposable for SerialConnection<T> {
    fn dispose(&self) {
        self.disposed.set(true);
    }
}

/// A connectable observable that delivers one notification on each connection, which may differ
/// from one connection to the next (`SerialSingleNotificationConnectable<T>`): it can complete the
/// observers of one connection and deliver values to those of the next.
struct SerialSingleNotificationConnectable<T> {
    notification_at_next_connect: RefCell<Notification<T>>,
    source_for_next_connect: RefCell<Rc<Subject<T>>>,
    next_connection_in_progress: RefCell<Rc<SerialConnection<T>>>,
    connections: RefCell<Vec<Rc<SerialConnection<T>>>>,
}

impl<T: Clone + 'static> SerialSingleNotificationConnectable<T> {
    fn new(initial_notification_at_next_connect: Notification<T>) -> Rc<Self> {
        let source = Subject::new();
        Rc::new(Self {
            notification_at_next_connect: RefCell::new(initial_notification_at_next_connect),
            next_connection_in_progress: RefCell::new(SerialConnection::new(source.clone())),
            source_for_next_connect: RefCell::new(source),
            connections: RefCell::new(Vec::new()),
        })
    }

    /// `Connections.Count`.
    fn connection_count(&self) -> usize {
        self.connections.borrow().len()
    }

    /// `Connections[index]`.
    fn connection(&self, index: usize) -> Rc<SerialConnection<T>> {
        self.connections.borrow()[index].clone()
    }

    /// `ActiveConnection`: the last connection, unless it is disposed.
    fn active_connection(&self) -> Option<Rc<SerialConnection<T>>> {
        self.connections.borrow().last().filter(|connection| !connection.disposed()).cloned()
    }

    /// `CurrentConnection`.
    fn current_connection(&self) -> Rc<SerialConnection<T>> {
        self.active_connection().unwrap_or_else(|| self.next_connection_in_progress.borrow().clone())
    }

    /// `SetNotificationForNextConnect`.
    fn set_notification_for_next_connect(&self, notification: Notification<T>) {
        *self.notification_at_next_connect.borrow_mut() = notification;
    }

    /// `DeliverNotificationForActiveConnection`.
    fn deliver_notification_for_active_connection(&self, notification: Notification<T>) {
        let active_connection = self.active_connection().expect("No connection is currently active");
        let source = active_connection.subject.borrow().clone().expect(
            "Active connection's source has been replaced and is no longer a Subject<T>, so it is not possible to deliver further notifications to current subscribers",
        );
        notification.accept(&*source);
    }
}

impl<T: Clone + 'static> IConnectableObservable<T> for SerialSingleNotificationConnectable<T> {
    fn connect(&self) -> Rc<dyn IDisposable> {
        let connecting = self.next_connection_in_progress.borrow().clone();
        let notification = self.notification_at_next_connect.borrow().clone();
        let source = self.source_for_next_connect.borrow().clone();

        let next_source = Subject::new();
        *self.next_connection_in_progress.borrow_mut() = SerialConnection::new(next_source.clone());
        *self.source_for_next_connect.borrow_mut() = next_source;
        self.connections.borrow_mut().push(connecting.clone());

        notification.accept(&*source);

        connecting
    }
}

impl<T: Clone + 'static> IObservable<T> for SerialSingleNotificationConnectable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let connection = self.current_connection();
        let source = connection.source.borrow().clone();
        source.subscribe(observer)
    }
}

/// A connection of [`SerialConnectableIgnoringConnect`] (`Connection`).
struct IgnoredConnection {
    disposed: Cell<bool>,
}

impl IgnoredConnection {
    /// `Disposed`.
    fn disposed(&self) -> bool {
        self.disposed.get()
    }
}

impl IDisposable for IgnoredConnection {
    fn dispose(&self) {
        self.disposed.set(true);
    }
}

/// A connectable observable that records the calls to connect but otherwise ignores them, and
/// subscribes observers to its current source (`SerialConnectableIgnoringConnect<T>`): observers
/// may receive notifications before the connect.
struct SerialConnectableIgnoringConnect<T> {
    source: RefCell<Rc<dyn IObservable<T>>>,
    connections: RefCell<Vec<Rc<IgnoredConnection>>>,
}

impl<T: 'static> SerialConnectableIgnoringConnect<T> {
    fn new(initial_source: Rc<dyn IObservable<T>>) -> Rc<Self> {
        Rc::new(Self { source: RefCell::new(initial_source), connections: RefCell::new(Vec::new()) })
    }

    /// `SetSource`.
    fn set_source(&self, source: Rc<dyn IObservable<T>>) {
        *self.source.borrow_mut() = source;
    }

    /// `Connections.Count`.
    fn connection_count(&self) -> usize {
        self.connections.borrow().len()
    }

    /// `Connections[index]`.
    fn connection(&self, index: usize) -> Rc<IgnoredConnection> {
        self.connections.borrow()[index].clone()
    }
}

impl<T: 'static> IConnectableObservable<T> for SerialConnectableIgnoringConnect<T> {
    fn connect(&self) -> Rc<dyn IDisposable> {
        let connection = Rc::new(IgnoredConnection { disposed: Cell::new(false) });
        self.connections.borrow_mut().push(connection.clone());
        connection
    }
}

impl<T: 'static> IObservable<T> for SerialConnectableIgnoringConnect<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.borrow().clone();
        source.subscribe(observer)
    }
}

/// Sets the default scheduler of the thread to a test scheduler that never runs, for the forms
/// whose delayed disconnect runs on `Scheduler.Default`, and restores it when dropped.
struct DefaultSchedulerGuard {
    _scheduler: TestScheduler,
}

impl DefaultSchedulerGuard {
    fn new() -> Self {
        let scheduler = TestScheduler::new();
        SchedulerDefaults::set_default_scheduler(Some(scheduler.scheduler()));
        Self { _scheduler: scheduler }
    }
}

impl Drop for DefaultSchedulerGuard {
    fn drop(&mut self) {
        SchedulerDefaults::set_default_scheduler(None);
    }
}

/// A connectable observable as the handle of its operators.
fn connectable<C: IConnectableObservable<i32> + 'static>(c: &Rc<C>) -> Rc<dyn IConnectableObservable<i32>> {
    c.clone()
}

/// A list that `source` appends its values to (`source.Subscribe(list.Add)`), and the
/// subscription.
fn subscribe_list(source: &Rc<dyn IObservable<i32>>) -> (Rc<RefCell<Vec<i32>>>, Rc<dyn IDisposable>) {
    let list = Rc::new(RefCell::new(Vec::new()));
    let subscription = {
        let list = list.clone();
        source.subscribe_next(move |x| list.borrow_mut().push(x))
    };
    (list, subscription)
}

/// `refCount.Subscribe(value => seen = value, () => terminated = true)`.
fn subscribe_seen(source: &Rc<dyn IObservable<i32>>, seen: &Rc<Cell<i32>>, terminated: &Rc<Cell<bool>>) -> Rc<dyn IDisposable> {
    let (seen, terminated) = (seen.clone(), terminated.clone());
    source.subscribe_next_completed(move |value| seen.set(value), move || terminated.set(true))
}

/// `scheduler.ScheduleAbsolute(subscribe, () => d = res.Subscribe(o))` and
/// `scheduler.ScheduleAbsolute(dispose, () => d.Dispose())`.
fn subscribe_between(scheduler: &TestScheduler, res: &Rc<dyn IObservable<i32>>, subscribe: i64, dispose: i64) -> Rc<MockObserver<i32>> {
    let o = scheduler.create_observer::<i32>();
    let d: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
    {
        let (res, o, d) = (res.clone(), o.clone(), d.clone());
        scheduler.schedule_absolute_action(subscribe, move || *d.borrow_mut() = Some(res.subscribe(o)));
    }
    {
        let d = d.clone();
        scheduler.schedule_absolute_action(dispose, move || {
            let subscription = d.borrow().clone().expect("subscribed");
            subscription.dispose();
        });
    }
    o
}

/// `scheduler.ScheduleAbsolute(subscribe, () => d = res.Subscribe(o))`, never disposed.
fn subscribe_at(scheduler: &TestScheduler, res: &Rc<dyn IObservable<i32>>, subscribe: i64) -> Rc<MockObserver<i32>> {
    let o = scheduler.create_observer::<i32>();
    let (res, observer) = (res.clone(), o.clone());
    scheduler.schedule_absolute_action(subscribe, move || {
        res.subscribe(observer);
    });
    o
}

// Immediate disconnect

#[test]
fn ref_count_no_delay_argument_checking() {
    // The checks of null arguments are not ported (argument validation); the minimum observers
    // of zero are (the negative ones cannot be expressed with usize).
    ferroui_reactive_testing::assert_throws(|| {
        Observable::never::<i32>().publish().ref_count_min(0);
    });
}

#[test]
fn ref_count_no_delay_connects_on_first() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_completed(250),
    ]);

    let subject = MySubject::new();

    let conn = ConnectableObservable::<i32>::new(xs.as_observable(), subject.clone());

    let res = scheduler.start({
        let conn = connectable(&conn);
        move || conn.ref_count()
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_completed::<i32>(250),
    );

    assert!(subject.disposed());
}

#[test]
fn ref_count_no_delay_min_observers_connects_on_observer_threshold_reached() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_completed(250),
    ]);

    let subject = MySubject::new();
    let conn = ConnectableObservable::<i32>::new(xs.as_observable(), subject.clone());
    let res = connectable(&conn).ref_count_min(2);

    let o1 = subscribe_at(&scheduler, &res, 205);

    let _o2 = subscribe_at(&scheduler, &res, 225);

    scheduler.run();

    assert_messages!(o1.messages(), RT::on_next(230, 3), RT::on_next(240, 4), RT::on_completed::<i32>(250));

    assert!(subject.disposed());
}

#[test]
fn ref_count_no_delay_source_produces_values_and_completes_in_connect() {
    let connected = Rc::new(Cell::new(0));
    let source = {
        let connected = connected.clone();
        Observable::defer(move || {
            connected.set(connected.get() + 1);
            Observable::range(1, 5)
        })
        .publish()
        .ref_count()
    };

    assert_eq!(0, connected.get());

    let (list1, _s1) = subscribe_list(&source);

    assert_eq!(1, connected.get());

    let expected1 = vec![1, 2, 3, 4, 5];

    assert_eq!(expected1, *list1.borrow());

    let (list2, _s2) = subscribe_list(&source);

    assert_eq!(1, connected.get());
    assert!(list2.borrow().is_empty());
}

#[test]
fn ref_count_no_delay_min_observers_source_produces_values_and_completes_in_connect() {
    let connected = Rc::new(Cell::new(0));
    let source = {
        let connected = connected.clone();
        Observable::defer(move || {
            connected.set(connected.get() + 1);
            Observable::range(1, 5)
        })
        .publish()
        .ref_count_min(2)
    };

    assert_eq!(0, connected.get());

    let (list1, _s1) = subscribe_list(&source);

    assert_eq!(0, connected.get());
    assert!(list1.borrow().is_empty());

    let (list2, _s2) = subscribe_list(&source);

    assert_eq!(1, connected.get());

    let expected = vec![1, 2, 3, 4, 5];

    assert_eq!(expected, *list1.borrow());
    assert_eq!(expected, *list2.borrow());
}

#[test]
fn ref_count_no_delay_source_completes_with_no_values_in_connect() {
    let connectable_source = SerialSingleNotificationConnectable::<i32>::new(Notification::OnCompleted);
    let ref_count = connectable(&connectable_source).ref_count();

    let s1 = ref_count.subscribe_ignore();
    assert_eq!(1, connectable_source.connection_count());

    // Since the source immediately completed, the RefCount goes back to zero subscribers
    // inside the call to Connect, so we expect to be disconnected.
    assert!(connectable_source.connection(0).disposed());

    let s2 = ref_count.subscribe_ignore();
    assert_eq!(2, connectable_source.connection_count());
    assert!(connectable_source.connection(1).disposed());

    s2.dispose();
    s1.dispose();
}

#[test]
fn ref_count_no_delay_min_observers_source_completes_with_no_values_in_connect() {
    let connectable_source = SerialSingleNotificationConnectable::<i32>::new(Notification::OnCompleted);
    let ref_count = connectable(&connectable_source).ref_count_min(2);

    let s1 = ref_count.subscribe_ignore();
    assert_eq!(0, connectable_source.connection_count());

    let s2 = ref_count.subscribe_ignore();
    assert_eq!(1, connectable_source.connection_count());

    // Since the source completes immediately, we will have no active subscribers, so
    // we expect to be disconnected.
    assert!(connectable_source.connection(0).disposed());

    s1.dispose();
    s2.dispose();

    // Disposing subscriptions should change nothing because they self-completed.
    assert_eq!(1, connectable_source.connection_count());

    // We're now back in the initial disconnected state, so nothing more should
    // happen until we get up to minObservers.
    let s3 = ref_count.subscribe_ignore();
    assert_eq!(1, connectable_source.connection_count());

    let s4 = ref_count.subscribe_ignore();
    assert_eq!(2, connectable_source.connection_count());
    assert!(connectable_source.connection(1).disposed());

    s4.dispose();
    s3.dispose();
}

#[test]
fn ref_count_no_delay_not_connected() {
    let disconnected = Rc::new(Cell::new(false));
    let count = Rc::new(Cell::new(0));
    let xs = {
        let (disconnected, count) = (disconnected.clone(), count.clone());
        Observable::defer(move || {
            count.set(count.get() + 1);
            let disconnected = disconnected.clone();
            Observable::create_with_action(move |_obs: Rc<dyn IObserver<i32>>| {
                let disconnected = disconnected.clone();
                move || disconnected.set(true)
            })
        })
    };

    let subject = MySubject::new();

    let conn = ConnectableObservable::<i32>::new(xs, subject.clone());
    let refd = connectable(&conn).ref_count();

    let dis1 = refd.subscribe_ignore();
    assert_eq!(1, count.get());
    assert_eq!(1, subject.subscribe_count());
    assert!(!disconnected.get());

    let dis2 = refd.subscribe_ignore();
    assert_eq!(1, count.get());
    assert_eq!(2, subject.subscribe_count());
    assert!(!disconnected.get());

    dis1.dispose();
    assert!(!disconnected.get());
    dis2.dispose();
    assert!(disconnected.get());
    disconnected.set(false);

    let dis3 = refd.subscribe_ignore();
    assert_eq!(2, count.get());
    assert_eq!(3, subject.subscribe_count());
    assert!(!disconnected.get());

    dis3.dispose();
    assert!(disconnected.get());
}

#[test]
fn ref_count_no_delay_min_observers_not_connected() {
    let connected = Rc::new(Cell::new(0));
    let source = {
        let connected = connected.clone();
        Observable::defer(move || {
            connected.set(connected.get() + 1);
            Observable::never::<i32>()
        })
        .publish()
        .ref_count_min(2)
    };

    assert_eq!(0, connected.get());

    source.subscribe_ignore();

    assert_eq!(0, connected.get());
}

#[test]
fn ref_count_no_delay_on_error() {
    let ex = new_error("ex");
    let xs = Observable::throw_on::<i32>(ex.clone(), Scheduler::immediate());

    let res = xs.publish().ref_count();

    for _ in 0..2 {
        let ex = ex.clone();
        res.subscribe_all(|_| panic!("OnNext unexpected"), move |ex_| assert!(Rc::ptr_eq(&ex, &ex_)), || panic!("OnCompleted unexpected"));
    }
}

#[test]
fn ref_count_no_delay_min_observers_on_error() {
    let ex = new_error("ex");
    let xs = Observable::throw_on::<i32>(ex.clone(), Scheduler::immediate());

    let res = xs.publish().ref_count_min(2);

    let exceptions_received: Rc<RefCell<Vec<ObservableError>>> = Rc::new(RefCell::new(Vec::new()));
    let add_subscriber = || {
        let (exceptions_received, ex) = (exceptions_received.clone(), ex.clone());
        res.subscribe_all(
            |_| panic!("OnNext unexpected"),
            move |_ex| exceptions_received.borrow_mut().push(ex.clone()),
            || panic!("OnComplete unexpected"),
        );
    };

    add_subscriber();
    assert_eq!(0, exceptions_received.borrow().len());

    add_subscriber();
    assert_eq!(2, exceptions_received.borrow().len());
    assert!(Rc::ptr_eq(&ex, &exceptions_received.borrow()[0]));
    assert!(Rc::ptr_eq(&ex, &exceptions_received.borrow()[1]));
}

#[test]
fn ref_count_no_delay_hot_source_multiple_subscribers() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_next(260, 6),
        RT::on_next(270, 7),
        RT::on_next(280, 8),
        RT::on_next(290, 9),
        RT::on_completed(300),
    ]);

    let res = xs.as_observable().publish().ref_count();

    let o1 = subscribe_between(&scheduler, &res, 215, 235);
    let o2 = subscribe_between(&scheduler, &res, 225, 275);
    let o3 = subscribe_between(&scheduler, &res, 255, 265);
    let o4 = subscribe_between(&scheduler, &res, 285, 320);

    scheduler.run();

    assert_messages!(o1.messages(), RT::on_next(220, 2), RT::on_next(230, 3));

    assert_messages!(o2.messages(), RT::on_next(230, 3), RT::on_next(240, 4), RT::on_next(250, 5), RT::on_next(260, 6), RT::on_next(270, 7));

    assert_messages!(o3.messages(), RT::on_next(260, 6));

    assert_messages!(o4.messages(), RT::on_next(290, 9), RT::on_completed::<i32>(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(215, 275), RT::subscribe(285, 300));
}

#[test]
fn ref_count_no_delay_min_observers_hot_source_multiple_subscribers() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),   // 0 subscribers
        RT::on_next(220, 2),   // 1 subscriber
        RT::on_next(230, 3),   // 2 subscribers
        RT::on_next(240, 4),   // 1 subscriber
        RT::on_next(250, 5),   // 1 subscriber
        RT::on_next(260, 6),   // 2 subscribers
        RT::on_next(270, 7),   // 1 subscribers
        RT::on_next(280, 8),   // 0 subscribers
        RT::on_next(290, 9),   // 1 subscribers
        RT::on_next(300, 10),  // 2 subscribers
        RT::on_completed(310),
    ]);

    let res = xs.as_observable().publish().ref_count_min(2);

    let o1 = subscribe_between(&scheduler, &res, 215, 235);
    let o2 = subscribe_between(&scheduler, &res, 225, 275);
    let o3 = subscribe_between(&scheduler, &res, 255, 265);
    let o4 = subscribe_between(&scheduler, &res, 285, 320);
    let o5 = subscribe_between(&scheduler, &res, 295, 320);

    scheduler.run();

    assert_messages!(o1.messages(), RT::on_next(230, 3));

    assert_messages!(o2.messages(), RT::on_next(230, 3), RT::on_next(240, 4), RT::on_next(250, 5), RT::on_next(260, 6), RT::on_next(270, 7));

    assert_messages!(o3.messages(), RT::on_next(260, 6));

    assert_messages!(o4.messages(), RT::on_next(300, 10), RT::on_completed::<i32>(310));

    assert_messages!(o5.messages(), RT::on_next(300, 10), RT::on_completed::<i32>(310));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(225, 275), RT::subscribe(295, 310));
}

#[test]
fn ref_count_no_delay_min_observers_subscriptions_drop_below_threshold_but_not_to_zero() {
    let subject = ReplaySubject::<i32>::with_buffer_size(5);
    let connected = Rc::new(Cell::new(0));
    let source = {
        let (subject, connected) = (subject.clone(), connected.clone());
        Observable::defer(move || {
            connected.set(connected.get() + 1);
            subject.as_observable()
        })
        .publish()
        .ref_count_min(2)
    };

    subject.on_next(1);

    assert_eq!(0, connected.get());

    let (list1, sub1) = subscribe_list(&source);
    assert_eq!(0, connected.get());
    assert!(list1.borrow().is_empty());

    subject.on_next(2);

    let (list2, sub2) = subscribe_list(&source);

    // Since connection only occurred with the 2nd subscriber, we expect both to get everything
    // the ReplaySubject has stored.
    let expected_sub1 = vec![1, 2];

    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub1, *list2.borrow());
    assert_eq!(1, connected.get());

    subject.on_next(3);

    // Both subscribers should have received the new item.
    let (expected_sub1, expected_sub2) = (vec![1, 2, 3], vec![1, 2, 3]);
    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub2, *list2.borrow());
    assert_eq!(1, connected.get());

    let (list3, _sub3) = subscribe_list(&source);

    // Since we were already connected, the 3rd subscriber just gets added to the observers of
    // the Publish multicast output, and no new connection should occur to the underlying ReplaySubject.
    // So for this 3rd subscription, no new items should be received by any of the subscribers
    let expected_sub3: Vec<i32> = vec![];
    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub2, *list2.borrow());
    assert_eq!(expected_sub3, *list3.borrow());
    assert_eq!(1, connected.get());

    subject.on_next(4);

    // All the current subscribers should have received that latest item.
    let (expected_sub1, expected_sub2, expected_sub3) = (vec![1, 2, 3, 4], vec![1, 2, 3, 4], vec![4]);
    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub2, *list2.borrow());
    assert_eq!(expected_sub3, *list3.borrow());
    assert_eq!(1, connected.get());

    sub1.dispose();

    subject.on_next(5);

    // The two remaining subscribers should have received that new item, but the one that just
    // unsubscribed should not.
    let (expected_sub1, expected_sub2, expected_sub3) = (vec![1, 2, 3, 4], vec![1, 2, 3, 4, 5], vec![4, 5]);
    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub2, *list2.borrow());
    assert_eq!(expected_sub3, *list3.borrow());
    assert_eq!(1, connected.get());

    sub2.dispose();

    subject.on_next(6);

    // We are now below the minObservers threshold of 2, but that threshold only governs when we move
    // from a disconnected state to a connected state. We should remain connected as long as there is
    // at least one subscriber, so we expect the remaining subscriber to receive that last item.
    let (expected_sub1, expected_sub2, expected_sub3) = (vec![1, 2, 3, 4], vec![1, 2, 3, 4, 5], vec![4, 5, 6]);
    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub2, *list2.borrow());
    assert_eq!(expected_sub3, *list3.borrow());
    assert_eq!(1, connected.get());
}

#[test]
fn ref_count_no_delay_subscriptions_drop_below_threshold_and_then_back_above() {
    let source_after_initial = Subject::<i32>::new();
    let connected = Rc::new(Cell::new(0));
    let source = {
        let (source_after_initial, connected) = (source_after_initial.clone(), connected.clone());
        Observable::defer(move || {
            connected.set(connected.get() + 1);
            Observable::range(1, 5).concat(source_after_initial.as_observable())
        })
        .publish()
        .ref_count_min(2)
    };

    assert_eq!(0, connected.get());

    let (list1, sub1) = subscribe_list(&source);

    assert_eq!(0, connected.get());
    assert!(list1.borrow().is_empty());

    let (list2, _sub2) = subscribe_list(&source);

    assert_eq!(1, connected.get());

    source_after_initial.on_next(6);

    sub1.dispose();
    source_after_initial.on_next(7);

    assert_eq!(1, connected.get());

    let (list3, _sub3) = subscribe_list(&source);

    // This is the distinguishing feature of this test. With that last subscription, we went from 1
    // subscriber (below minObservers) but still connected (because we already hit minObservers once
    // and never dropped to zero), and now we're passing through minObservers again. We used to have
    // a bug where we would erroneously attempt to reconnect at this point.
    assert_eq!(1, connected.get());
    source_after_initial.on_next(8);

    let expected_sub1 = vec![1, 2, 3, 4, 5, 6];
    let expected_sub2 = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let expected_sub3 = vec![8];

    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub2, *list2.borrow());
    assert_eq!(expected_sub3, *list3.borrow());
}

#[test]
fn ref_count_no_delay_values_during_and_after_subscribe() {
    let subject = ReplaySubject::<i32>::with_buffer_size(5);
    let source = subject.as_observable().publish().ref_count();

    subject.on_next(1);

    // Although the source is a ReplaySubject, the use of Publish means there will only be
    // a single subscription to the ReplaySubject, so it will only replay one. (It will replay
    // that first value on the initial connect.) So we expect each subscriber to see fewer and
    // fewer values.
    let expected1 = vec![1];

    let (list1, _s1) = subscribe_list(&source);
    assert_eq!(expected1, *list1.borrow());

    subject.on_next(2);

    let (list2, _s2) = subscribe_list(&source);

    let expected1 = vec![1, 2];
    let expected2: Vec<i32> = vec![];

    assert_eq!(expected1, *list1.borrow());
    assert_eq!(expected2, *list2.borrow());

    subject.on_next(3);

    let (list3, _s3) = subscribe_list(&source);

    let expected1 = vec![1, 2, 3];
    let expected2 = vec![3];
    let expected3: Vec<i32> = vec![];

    assert_eq!(expected1, *list1.borrow());
    assert_eq!(expected2, *list2.borrow());
    assert_eq!(expected3, *list3.borrow());

    subject.on_next(4);
    let expected1 = vec![1, 2, 3, 4];
    let expected2 = vec![3, 4];
    let expected3 = vec![4];
    assert_eq!(expected1, *list1.borrow());
    assert_eq!(expected2, *list2.borrow());
    assert_eq!(expected3, *list3.borrow());
}

#[test]
fn ref_count_no_delay_min_observers_values_during_and_after_subscribe() {
    let subject = ReplaySubject::<i32>::with_buffer_size(5);
    let source = subject.as_observable().publish().ref_count_min(2);

    subject.on_next(1);

    let (list1, _s1) = subscribe_list(&source);
    assert!(list1.borrow().is_empty());

    subject.on_next(2);

    let expected1and2 = vec![1, 2];

    let (list2, _s2) = subscribe_list(&source);

    assert_eq!(expected1and2, *list1.borrow());
    assert_eq!(expected1and2, *list2.borrow());

    subject.on_next(3);
    let expected1and2 = vec![1, 2, 3];
    assert_eq!(expected1and2, *list1.borrow());
    assert_eq!(expected1and2, *list2.borrow());

    let (list3, _s3) = subscribe_list(&source);

    let expected3: Vec<i32> = vec![];
    assert_eq!(expected1and2, *list1.borrow());
    assert_eq!(expected1and2, *list2.borrow());
    assert_eq!(expected3, *list3.borrow());

    subject.on_next(4);
    let expected1and2 = vec![1, 2, 3, 4];
    let expected3 = vec![4];
    assert_eq!(expected1and2, *list1.borrow());
    assert_eq!(expected1and2, *list2.borrow());
    assert_eq!(expected3, *list3.borrow());
}

#[test]
fn ref_count_no_delay_can_connect_again_if_previous_subscription_terminated_from_subscribe_by_completion() {
    let seen = Rc::new(Cell::new(0));
    let terminated = Rc::new(Cell::new(false));

    // On initial subscription, the source will produce one value and will not complete.
    let connectable_source = SerialSingleNotificationConnectable::<i32>::new(Notification::OnNext(36));
    let ref_count = connectable(&connectable_source).ref_count();

    {
        let s = subscribe_seen(&ref_count, &seen, &terminated);
        assert_eq!(36, seen.get());
        s.dispose();
    }

    seen.set(0);
    terminated.set(false);

    // This time around, the source will complete when subscribed to.
    connectable_source.set_notification_for_next_connect(Notification::OnCompleted);

    {
        let s = subscribe_seen(&ref_count, &seen, &terminated);
        assert_eq!(0, seen.get());
        assert!(terminated.get());
        s.dispose();
    }

    seen.set(0);
    terminated.set(false);

    // Now we go back to the initial behaviour in which the source produces one value and does not complete.
    connectable_source.set_notification_for_next_connect(Notification::OnNext(42));

    {
        let s = subscribe_seen(&ref_count, &seen, &terminated);
        assert_eq!(42, seen.get());
        assert!(!terminated.get());
        s.dispose();
    }
}

#[test]
fn ref_count_no_delay_min_observers_can_connect_again_if_previous_subscription_terminated_from_subscribe_by_completion() {
    let (seen1, seen2) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let (terminated1, terminated2) = (Rc::new(Cell::new(false)), Rc::new(Cell::new(false)));

    // On initial subscription, the source will produce one value and will not complete.
    let connectable_source = SerialSingleNotificationConnectable::<i32>::new(Notification::OnNext(36));
    let ref_count = connectable(&connectable_source).ref_count_min(2);

    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        assert_eq!(0, seen1.get());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(36, seen1.get());
            assert_eq!(36, seen2.get());
            s2.dispose();
        }
        s1.dispose();
    }

    seen1.set(0);
    seen2.set(0);
    terminated1.set(false);
    terminated2.set(false);

    // This time around, the source will complete when subscribed to.
    connectable_source.set_notification_for_next_connect(Notification::OnCompleted);

    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        assert!(!terminated1.get());
        assert!(!terminated2.get());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(0, seen1.get());
            assert_eq!(0, seen2.get());
            assert!(terminated1.get());
            assert!(terminated2.get());
            s2.dispose();
        }
        s1.dispose();
    }

    seen1.set(0);
    seen2.set(0);
    terminated1.set(false);
    terminated2.set(false);

    // Now we go back to the initial behaviour in which the source produces one value and does not complete.
    connectable_source.set_notification_for_next_connect(Notification::OnNext(42));

    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        assert_eq!(0, seen1.get());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(42, seen1.get());
            assert_eq!(42, seen2.get());
            assert!(!terminated1.get());
            assert!(!terminated2.get());
            s2.dispose();
        }
        s1.dispose();
    }
}

// Delayed disconnect

#[test]
fn ref_count_delayed_disconnect_argument_checking() {
    // The checks of null arguments are not ported (argument validation); the minimum observers
    // of zero are (the negative ones cannot be expressed with usize).
    let _default = DefaultSchedulerGuard::new();
    ferroui_reactive_testing::assert_throws(|| {
        Observable::never::<i32>().publish().ref_count_min_with_delay(0, Duration::from_secs(2));
    });
    ferroui_reactive_testing::assert_throws(|| {
        Observable::never::<i32>().publish().ref_count_min_with_delay_on(0, Duration::from_secs(2), SchedulerDefaults::default_scheduler());
    });
}

#[test]
fn ref_count_delayed_disconnect_connects_on_first() {
    let _default = DefaultSchedulerGuard::new();
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_completed(250),
    ]);

    let subject = MySubject::new();

    let conn = ConnectableObservable::<i32>::new(xs.as_observable(), subject.clone());

    let res = scheduler.start({
        let conn = connectable(&conn);
        move || conn.ref_count_with_delay(Duration::from_secs(2))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_completed::<i32>(250),
    );

    assert!(subject.disposed());
}

#[test]
fn ref_count_delayed_disconnect_min_observers_connects_on_observer_threshold_reached() {
    let _default = DefaultSchedulerGuard::new();
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_completed(250),
    ]);

    let subject = MySubject::new();
    let conn = ConnectableObservable::<i32>::new(xs.as_observable(), subject.clone());
    let res = connectable(&conn).ref_count_min_with_delay(2, ticks_to_duration(300));

    let o1 = subscribe_at(&scheduler, &res, 210);

    let _o2 = subscribe_at(&scheduler, &res, 225);

    scheduler.run();

    assert_messages!(o1.messages(), RT::on_next(230, 3), RT::on_next(240, 4), RT::on_completed::<i32>(250));

    assert!(subject.disposed());
}

#[test]
fn ref_count_delayed_disconnect_min_observers_source_produces_values_and_completes_in_subscribe() {
    let _default = DefaultSchedulerGuard::new();
    let connected = Rc::new(Cell::new(0));
    let source = {
        let connected = connected.clone();
        Observable::defer(move || {
            connected.set(connected.get() + 1);
            Observable::range(1, 5)
        })
        .publish()
        .ref_count_min_with_delay(2, Duration::from_secs(60))
    };

    assert_eq!(0, connected.get());

    let (list1, _s1) = subscribe_list(&source);

    assert_eq!(0, connected.get());
    assert!(list1.borrow().is_empty());

    let (list2, _s2) = subscribe_list(&source);

    assert_eq!(1, connected.get());

    let expected = vec![1, 2, 3, 4, 5];

    assert_eq!(expected, *list1.borrow());
    assert_eq!(expected, *list2.borrow());
}

#[test]
fn ref_count_delayed_disconnect_source_completes_with_no_values_in_subscribe() {
    let _default = DefaultSchedulerGuard::new();
    let subscribed = Rc::new(Cell::new(0));
    let unsubscribed = Rc::new(Cell::new(0));

    let o1 = {
        let (subscribed, unsubscribed) = (subscribed.clone(), unsubscribed.clone());
        Observable::create(move |observer: Rc<dyn IObserver<String>>| {
            subscribed.set(subscribed.get() + 1);
            observer.on_completed();

            let unsubscribed = unsubscribed.clone();
            Disposable::create(move || unsubscribed.set(unsubscribed.get() + 1))
        })
    };

    let o2 = o1.publish().ref_count_with_delay(Duration::from_secs(20));

    let _s1 = o2.subscribe_ignore();
    assert_eq!(1, subscribed.get());
    assert_eq!(1, unsubscribed.get());

    let _s2 = o2.subscribe_ignore();
    assert_eq!(1, subscribed.get());
    assert_eq!(1, unsubscribed.get());
}

#[test]
fn ref_count_delayed_disconnect_min_observers_source_completes_with_no_values_in_subscribe() {
    let scheduler = TestScheduler::new();
    let subscribed = Rc::new(Cell::new(0));
    let unsubscribed = Rc::new(Cell::new(0));

    let o1 = {
        let (subscribed, unsubscribed) = (subscribed.clone(), unsubscribed.clone());
        Observable::create(move |observer: Rc<dyn IObserver<String>>| {
            subscribed.set(subscribed.get() + 1);
            observer.on_completed();

            let unsubscribed = unsubscribed.clone();
            Disposable::create(move || unsubscribed.set(unsubscribed.get() + 1))
        })
    };

    let o2 = o1.publish().ref_count_min_with_delay_on(2, ticks_to_duration(10), scheduler.scheduler());

    let s1 = o2.subscribe_ignore();
    assert_eq!(0, subscribed.get());
    assert_eq!(0, unsubscribed.get());

    // Note that although we've got a delayed disconnect, we don't need to call AdvanceBy
    // here because the source itself completes. The disconnect is triggered by the source,
    // not the RefCount in this test.
    let s2 = o2.subscribe_ignore();
    assert_eq!(1, subscribed.get());
    assert_eq!(1, unsubscribed.get());

    s1.dispose();
    s2.dispose();

    // At this point, the RefCount has 0 subscribers, and will have disconnected from its source.
    // When we add a new subscriber, the count will be at 0, which is below minObservers, so we
    // don't expect a new connection. The subject of Publish completed already, so it completes
    // further subscriptions at once: the count goes up to 1 and back down to 0 before this call
    // to subscribe returns.
    let _s3 = o2.subscribe_ignore();
    assert_eq!(1, subscribed.get());
    assert_eq!(1, unsubscribed.get());

    // For the reasons above, o2 completed in the last call to subscribe, so the count is zero
    // here and a second subscriber does not reach the threshold either.
    let _s4 = o2.subscribe_ignore();
    assert_eq!(1, subscribed.get());
    assert_eq!(1, unsubscribed.get());
}

#[test]
fn ref_count_delayed_disconnect_not_connected() {
    let scheduler = TestScheduler::new();
    let disconnected = Rc::new(Cell::new(false));
    let count = Rc::new(Cell::new(0));
    let xs = {
        let (disconnected, count) = (disconnected.clone(), count.clone());
        Observable::defer(move || {
            count.set(count.get() + 1);
            let disconnected = disconnected.clone();
            Observable::create_with_action(move |_obs: Rc<dyn IObserver<i32>>| {
                let disconnected = disconnected.clone();
                move || disconnected.set(true)
            })
        })
    };

    let subject = MySubject::new();

    let conn = ConnectableObservable::<i32>::new(xs, subject.clone());
    let refd = connectable(&conn).ref_count_with_delay_on(ticks_to_duration(20), scheduler.scheduler());

    let dis1 = refd.subscribe_ignore();
    assert_eq!(1, count.get());
    assert_eq!(1, subject.subscribe_count());
    assert!(!disconnected.get());

    let dis2 = refd.subscribe_ignore();
    assert_eq!(1, count.get());
    assert_eq!(2, subject.subscribe_count());
    assert!(!disconnected.get());

    dis1.dispose();
    assert!(!disconnected.get());
    dis2.dispose();
    assert!(!disconnected.get());

    scheduler.advance_by(19);
    assert!(!disconnected.get());

    scheduler.advance_by(1);
    assert!(disconnected.get());
    disconnected.set(false);

    let dis3 = refd.subscribe_ignore();
    assert_eq!(2, count.get());
    assert_eq!(3, subject.subscribe_count());
    assert!(!disconnected.get());

    dis3.dispose();
    scheduler.advance_by(20);
    assert!(disconnected.get());
}

#[test]
fn ref_count_delayed_disconnect_min_observers_not_connected() {
    let _default = DefaultSchedulerGuard::new();
    let connected = Rc::new(Cell::new(0));
    let source = {
        let connected = connected.clone();
        Observable::defer(move || {
            connected.set(connected.get() + 1);
            Observable::never::<i32>()
        })
        .publish()
        .ref_count_min_with_delay(2, Duration::from_secs(60))
    };

    assert_eq!(0, connected.get());

    source.subscribe_ignore();

    assert_eq!(0, connected.get());
}

#[test]
fn ref_count_delayed_disconnect_on_error() {
    let _default = DefaultSchedulerGuard::new();
    let ex = new_error("ex");
    let xs = Observable::throw_on::<i32>(ex.clone(), Scheduler::immediate());

    let res = xs.publish().ref_count_with_delay(Duration::from_secs(2));

    for _ in 0..2 {
        let ex = ex.clone();
        res.subscribe_all(|_| panic!("OnNext unexpected"), move |ex_| assert!(Rc::ptr_eq(&ex, &ex_)), || panic!("OnCompleted unexpected"));
    }
}

#[test]
fn ref_count_delayed_disconnect_min_observers_on_error() {
    let _default = DefaultSchedulerGuard::new();
    let ex = new_error("ex");
    let xs = Observable::throw_on::<i32>(ex.clone(), Scheduler::immediate());

    let res = xs.publish().ref_count_min_with_delay(2, Duration::from_secs(200));

    let exceptions_received: Rc<RefCell<Vec<ObservableError>>> = Rc::new(RefCell::new(Vec::new()));
    let add_subscriber = || {
        let (exceptions_received, ex) = (exceptions_received.clone(), ex.clone());
        res.subscribe_all(
            |_| panic!("OnNext unexpected"),
            move |_ex| exceptions_received.borrow_mut().push(ex.clone()),
            || panic!("OnComplete unexpected"),
        );
    };

    add_subscriber();
    assert_eq!(0, exceptions_received.borrow().len());

    add_subscriber();
    assert_eq!(2, exceptions_received.borrow().len());
    assert!(Rc::ptr_eq(&ex, &exceptions_received.borrow()[0]));
    assert!(Rc::ptr_eq(&ex, &exceptions_received.borrow()[1]));
}

#[test]
fn ref_count_delayed_disconnect_hot_source_multiple_subscribers() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_next(250, 5),
        RT::on_next(260, 6),
        RT::on_next(270, 7),
        RT::on_next(280, 8),
        RT::on_next(290, 9),
        RT::on_completed(300),
    ]);

    let res = xs.as_observable().publish().ref_count_with_delay_on(ticks_to_duration(9), scheduler.scheduler());

    let o1 = subscribe_between(&scheduler, &res, 215, 235);
    let o2 = subscribe_between(&scheduler, &res, 225, 275);
    let o3 = subscribe_between(&scheduler, &res, 255, 265);
    let o4 = subscribe_between(&scheduler, &res, 285, 320);

    scheduler.run();

    assert_messages!(o1.messages(), RT::on_next(220, 2), RT::on_next(230, 3));

    assert_messages!(o2.messages(), RT::on_next(230, 3), RT::on_next(240, 4), RT::on_next(250, 5), RT::on_next(260, 6), RT::on_next(270, 7));

    assert_messages!(o3.messages(), RT::on_next(260, 6));

    assert_messages!(o4.messages(), RT::on_next(290, 9), RT::on_completed::<i32>(300));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(215, 284), RT::subscribe(285, 300));
}

#[test]
fn ref_count_delayed_disconnect_min_observers_hot_source_multiple_subscribers() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, 1),   // 0 subscribers
        RT::on_next(220, 2),   // 1 subscriber
        RT::on_next(230, 3),   // 2 subscribers
        RT::on_next(240, 4),   // 1 subscriber
        RT::on_next(250, 5),   // 1 subscriber
        RT::on_next(260, 6),   // 2 subscribers
        RT::on_next(270, 7),   // 1 subscribers
        RT::on_next(280, 8),   // 0 subscribers
        RT::on_next(290, 9),   // 1 subscribers
        RT::on_next(300, 10),  // 2 subscribers
        RT::on_completed(310),
    ]);

    let res = xs.as_observable().publish().ref_count_min_with_delay_on(2, ticks_to_duration(9), scheduler.scheduler());

    let o1 = subscribe_between(&scheduler, &res, 215, 235);
    let o2 = subscribe_between(&scheduler, &res, 225, 275);
    let o3 = subscribe_between(&scheduler, &res, 255, 265);
    let o4 = subscribe_between(&scheduler, &res, 285, 320);
    let o5 = subscribe_between(&scheduler, &res, 295, 320);

    scheduler.run();

    assert_messages!(o1.messages(), RT::on_next(230, 3));

    assert_messages!(o2.messages(), RT::on_next(230, 3), RT::on_next(240, 4), RT::on_next(250, 5), RT::on_next(260, 6), RT::on_next(270, 7));

    assert_messages!(o3.messages(), RT::on_next(260, 6));

    assert_messages!(o4.messages(), RT::on_next(300, 10), RT::on_completed::<i32>(310));

    assert_messages!(o5.messages(), RT::on_next(300, 10), RT::on_completed::<i32>(310));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(225, 284), RT::subscribe(295, 310));
}

#[test]
fn ref_count_delayed_disconnect_min_observers_subscriptions_drop_below_threshold_but_not_to_zero() {
    let _default = DefaultSchedulerGuard::new();
    let subject = ReplaySubject::<i32>::with_buffer_size(5);
    let connected = Rc::new(Cell::new(0));
    let source = {
        let (subject, connected) = (subject.clone(), connected.clone());
        Observable::defer(move || {
            connected.set(connected.get() + 1);
            subject.as_observable()
        })
        .publish()
        .ref_count_min_with_delay(2, Duration::from_secs(60))
    };

    subject.on_next(1);

    assert_eq!(0, connected.get());

    let (list1, sub1) = subscribe_list(&source);
    assert_eq!(0, connected.get());
    assert!(list1.borrow().is_empty());

    subject.on_next(2);

    let (list2, sub2) = subscribe_list(&source);

    // Since connection only occurred with the 2nd subscriber, we expect both to get everything
    // the ReplaySubject has stored.
    let expected_sub1 = vec![1, 2];

    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub1, *list2.borrow());
    assert_eq!(1, connected.get());

    subject.on_next(3);

    // Both subscribers should have received the new item.
    let (expected_sub1, expected_sub2) = (vec![1, 2, 3], vec![1, 2, 3]);
    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub2, *list2.borrow());
    assert_eq!(1, connected.get());

    let (list3, _sub3) = subscribe_list(&source);

    // Since we were already connected, the 3rd subscriber just gets added to the observers of
    // the Publish multicast output, and no new connection should occur to the underlying ReplaySubject.
    // So for this 3rd subscription, no new items should be received by any of the subscribers
    let expected_sub3: Vec<i32> = vec![];
    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub2, *list2.borrow());
    assert_eq!(expected_sub3, *list3.borrow());
    assert_eq!(1, connected.get());

    subject.on_next(4);

    // All the current subscribers should have received that latest item.
    let (expected_sub1, expected_sub2, expected_sub3) = (vec![1, 2, 3, 4], vec![1, 2, 3, 4], vec![4]);
    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub2, *list2.borrow());
    assert_eq!(expected_sub3, *list3.borrow());
    assert_eq!(1, connected.get());

    sub1.dispose();

    subject.on_next(5);

    // The two remaining subscribers should have received that new item, but the one that just
    // unsubscribed should not.
    let (expected_sub1, expected_sub2, expected_sub3) = (vec![1, 2, 3, 4], vec![1, 2, 3, 4, 5], vec![4, 5]);
    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub2, *list2.borrow());
    assert_eq!(expected_sub3, *list3.borrow());
    assert_eq!(1, connected.get());

    sub2.dispose();

    subject.on_next(6);

    // We are now below the minObservers threshold of 2, but that threshold only governs when we move
    // from a disconnected state to a connected state. We should remain connected as long as there is
    // at least one subscriber, so we expect the remaining subscriber to receive that last item.
    let (expected_sub1, expected_sub2, expected_sub3) = (vec![1, 2, 3, 4], vec![1, 2, 3, 4, 5], vec![4, 5, 6]);
    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub2, *list2.borrow());
    assert_eq!(expected_sub3, *list3.borrow());
    assert_eq!(1, connected.get());
}

#[test]
fn ref_count_delayed_disconnect_subscriptions_drop_below_threshold_and_then_back_above() {
    let scheduler = TestScheduler::new();

    let source_after_initial = Subject::<i32>::new();
    let connected = Rc::new(Cell::new(0));
    let source = {
        let (source_after_initial, connected) = (source_after_initial.clone(), connected.clone());
        Observable::defer(move || {
            connected.set(connected.get() + 1);
            Observable::range(1, 5).concat(source_after_initial.as_observable())
        })
        .publish()
        .ref_count_min_with_delay_on(2, ticks_to_duration(10), scheduler.scheduler())
    };

    assert_eq!(0, connected.get());

    let (list1, sub1) = subscribe_list(&source); // 1 subscriber

    assert_eq!(0, connected.get());
    assert!(list1.borrow().is_empty());

    let (list2, _sub2) = subscribe_list(&source); // 2 subscribers

    assert_eq!(1, connected.get());

    source_after_initial.on_next(6);

    sub1.dispose(); // 1 subscriber

    // We don't expect a disconnect, but provide enough time for one to occur, should that bug ever creep in
    scheduler.advance_by(10);
    assert_eq!(1, connected.get());

    source_after_initial.on_next(7);

    assert_eq!(1, connected.get());

    let (list3, _sub3) = subscribe_list(&source);

    // This is the distinguishing feature of this test. With that last subscription, we went from 1
    // subscriber (below minObservers) but still connected (because we already hit minObservers once
    // and never dropped to zero), and now we're passing through minObservers again. We used to have
    // a bug where we would erroneously attempt to reconnect at this point.
    assert_eq!(1, connected.get());
    source_after_initial.on_next(8);

    let expected_sub1 = vec![1, 2, 3, 4, 5, 6];
    let expected_sub2 = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let expected_sub3 = vec![8];

    assert_eq!(expected_sub1, *list1.borrow());
    assert_eq!(expected_sub2, *list2.borrow());
    assert_eq!(expected_sub3, *list3.borrow());
}

#[test]
fn ref_count_delayed_disconnect_subscriptions_drop_to_zero_then_new_subscription_arrives_before_disconnect_delay() {
    let scheduler = TestScheduler::new();
    let source = SerialSingleNotificationConnectable::<i32>::new(Notification::OnNext(1));
    let rco = connectable(&source).ref_count_with_delay_on(ticks_to_duration(10), scheduler.scheduler());

    let s1 = rco.subscribe_ignore();
    s1.dispose();

    // There are now 0 subscribers, but the time for the disconnect has not yet come.
    assert_eq!(1, source.connection_count());
    assert!(!source.connection(0).disposed());

    scheduler.advance_by(9);

    // The time has still not come,
    assert_eq!(1, source.connection_count());
    assert!(!source.connection(0).disposed());

    // Since we were still connected, this should move the connection from a 'waiting to
    // shut down' state into an active state.
    let seen = Rc::new(Cell::new(0));
    let terminated = Rc::new(Cell::new(false));
    let _s2 = subscribe_seen(&rco, &seen, &terminated);
    source.deliver_notification_for_active_connection(Notification::OnNext(2));
    assert_eq!(2, seen.get());
    assert!(!terminated.get());
    assert!(!source.connection(0).disposed());

    // This moves us past the time when `RefCount` would have shut down the connection if no new
    // subscriptions had turned up.
    scheduler.advance_by(2);

    assert!(!terminated.get());
    assert!(!source.connection(0).disposed());

    // We should be able to advance well beyond the disconnect delay because we have an active
    // subscriber.
    scheduler.advance_by(20);

    assert!(!terminated.get());
    assert!(!source.connection(0).disposed());
}

#[test]
fn ref_count_delayed_disconnect_min_observers_subscriptions_drop_to_zero_then_new_subscription_arrives_before_disconnect_delay() {
    let scheduler = TestScheduler::new();
    let source = SerialSingleNotificationConnectable::<i32>::new(Notification::OnNext(1));
    let rco = connectable(&source).ref_count_min_with_delay_on(2, ticks_to_duration(10), scheduler.scheduler());

    let s1 = rco.subscribe_ignore();
    let s2 = rco.subscribe_ignore();
    s1.dispose();
    s2.dispose();

    // There are now 0 subscribers, but the time for the disconnect has not yet come.
    assert_eq!(1, source.connection_count());
    assert!(!source.connection(0).disposed());

    scheduler.advance_by(9);

    // The time has still not come,
    assert_eq!(1, source.connection_count());
    assert!(!source.connection(0).disposed());

    // Since we were still connected, this should move the connection from a 'waiting to
    // shut down' state into an active state. (We're below the minObservers threshold, but
    // that just determines when Connect is called. RefCount has historically always waited
    // for the subscription count to reach 0 before disconnecting, so if that count goes
    // above 0 while we were waiting for the disconnect delay, it should return to an
    // active state.)
    let seen = Rc::new(Cell::new(0));
    let terminated = Rc::new(Cell::new(false));
    let _s3 = subscribe_seen(&rco, &seen, &terminated);
    source.deliver_notification_for_active_connection(Notification::OnNext(2));
    assert_eq!(2, seen.get());
    assert!(!terminated.get());
    assert!(!source.connection(0).disposed());

    // This moves us past the time when `RefCount` would have shut down the connection if
    // no new subscriptions had turned up. The arrival of a new subscriber should ensure
    // that we remain connected.
    scheduler.advance_by(2);

    assert!(!terminated.get());
    assert!(!source.connection(0).disposed());

    // We should be able to advance well beyond the disconnect delay because we have an active
    // subscriber.
    scheduler.advance_by(20);

    assert!(!terminated.get());
    assert!(!source.connection(0).disposed());
}

#[test]
fn ref_count_delayed_disconnect_values_during_and_after_subscribe() {
    let _default = DefaultSchedulerGuard::new();
    let subject = ReplaySubject::<i32>::with_buffer_size(5);
    let source = subject.as_observable().publish().ref_count_with_delay(Duration::from_secs(20));

    subject.on_next(1);

    // Although the source is a ReplaySubject, the use of Publish means there will only be
    // a single subscription to the ReplaySubject, so it will only replay one. (It will replay
    // that first value on the initial connect.) So we expect each subscriber to see fewer and
    // fewer values.
    let expected1 = vec![1];

    let (list1, _s1) = subscribe_list(&source);
    assert_eq!(expected1, *list1.borrow());

    subject.on_next(2);

    let (list2, _s2) = subscribe_list(&source);

    let expected1 = vec![1, 2];
    let expected2: Vec<i32> = vec![];

    assert_eq!(expected1, *list1.borrow());
    assert_eq!(expected2, *list2.borrow());

    subject.on_next(3);

    let (list3, _s3) = subscribe_list(&source);

    let expected1 = vec![1, 2, 3];
    let expected2 = vec![3];
    let expected3: Vec<i32> = vec![];

    assert_eq!(expected1, *list1.borrow());
    assert_eq!(expected2, *list2.borrow());
    assert_eq!(expected3, *list3.borrow());

    subject.on_next(4);
    let expected1 = vec![1, 2, 3, 4];
    let expected2 = vec![3, 4];
    let expected3 = vec![4];
    assert_eq!(expected1, *list1.borrow());
    assert_eq!(expected2, *list2.borrow());
    assert_eq!(expected3, *list3.borrow());
}

#[test]
fn ref_count_delayed_disconnect_min_observers_values_during_and_after_subscribe() {
    let _default = DefaultSchedulerGuard::new();
    let subject = ReplaySubject::<i32>::with_buffer_size(5);
    let source = subject.as_observable().publish().ref_count_min_with_delay(2, Duration::from_secs(20));

    subject.on_next(1);

    let (list1, _s1) = subscribe_list(&source);
    assert!(list1.borrow().is_empty());

    subject.on_next(2);

    let expected1and2 = vec![1, 2];

    let (list2, _s2) = subscribe_list(&source);

    assert_eq!(expected1and2, *list1.borrow());
    assert_eq!(expected1and2, *list2.borrow());

    subject.on_next(3);
    let expected1and2 = vec![1, 2, 3];
    assert_eq!(expected1and2, *list1.borrow());
    assert_eq!(expected1and2, *list2.borrow());

    let (list3, _s3) = subscribe_list(&source);

    let expected3: Vec<i32> = vec![];
    assert_eq!(expected1and2, *list1.borrow());
    assert_eq!(expected1and2, *list2.borrow());
    assert_eq!(expected3, *list3.borrow());

    subject.on_next(4);
    let expected1and2 = vec![1, 2, 3, 4];
    let expected3 = vec![4];
    assert_eq!(expected1and2, *list1.borrow());
    assert_eq!(expected1and2, *list2.borrow());
    assert_eq!(expected3, *list3.borrow());
}

/// One data row of
/// [`ref_count_delayed_disconnect_can_connect_again_if_previous_subscription_terminated_from_subscribe_by_completion`].
fn delayed_disconnect_can_connect_again_if_previous_subscription_terminated_from_subscribe_by_completion(re_subscribe_before_delayed_disconnect: bool) {
    let scheduler = TestScheduler::new();
    let seen = Rc::new(Cell::new(0));
    let terminated = Rc::new(Cell::new(false));

    // On initial subscription, the source will produce one value and will not complete.
    let connectable_source = SerialSingleNotificationConnectable::<i32>::new(Notification::OnNext(36));
    let ref_count = connectable(&connectable_source).ref_count_with_delay_on(ticks_to_duration(10), scheduler.scheduler());

    {
        let s = subscribe_seen(&ref_count, &seen, &terminated);
        assert_eq!(36, seen.get());
        assert_eq!(1, connectable_source.connection_count());
        assert!(!connectable_source.connection(0).disposed());
        s.dispose();
    }

    assert!(!connectable_source.connection(0).disposed());

    // For these initial subscriptions, we allow enough time for the delayed disconnect to occur even if
    // reSubscribeBeforeDelayedDisconnect is false, because it's the resubscription after a source-induced
    // completion that this test is interested in.
    scheduler.advance_by(11);

    assert_eq!(1, connectable_source.connection_count());
    assert!(connectable_source.connection(0).disposed());

    seen.set(0);
    terminated.set(false);

    // This time around, when Connect is called, all subscriptions after the preceding Connect will be
    // completed.
    connectable_source.set_notification_for_next_connect(Notification::OnCompleted);

    {
        let s = subscribe_seen(&ref_count, &seen, &terminated);
        assert_eq!(0, seen.get());
        assert!(terminated.get());
        assert_eq!(2, connectable_source.connection_count());
        assert!(!connectable_source.connection(1).disposed());
        s.dispose();
    }

    assert_eq!(2, connectable_source.connection_count());
    assert!(!connectable_source.connection(1).disposed());

    scheduler.advance_by(if re_subscribe_before_delayed_disconnect { 1 } else { 11 });

    assert_eq!(2, connectable_source.connection_count());
    assert_eq!(!re_subscribe_before_delayed_disconnect, connectable_source.connection(1).disposed());

    seen.set(0);
    terminated.set(false);

    // Now we go back to the initial behaviour in which the source produces one value and does not complete.
    connectable_source.set_notification_for_next_connect(Notification::OnNext(42));

    {
        let s = subscribe_seen(&ref_count, &seen, &terminated);
        assert_eq!(if re_subscribe_before_delayed_disconnect { 0 } else { 42 }, seen.get());
        assert_eq!(re_subscribe_before_delayed_disconnect, terminated.get());
        assert_eq!(if re_subscribe_before_delayed_disconnect { 2 } else { 3 }, connectable_source.connection_count());
        assert!(!connectable_source.connection(if re_subscribe_before_delayed_disconnect { 1 } else { 2 }).disposed());
        s.dispose();
    }
}

#[test]
fn ref_count_delayed_disconnect_can_connect_again_if_previous_subscription_terminated_from_subscribe_by_completion() {
    delayed_disconnect_can_connect_again_if_previous_subscription_terminated_from_subscribe_by_completion(true);
    delayed_disconnect_can_connect_again_if_previous_subscription_terminated_from_subscribe_by_completion(false);
}

#[test]
fn ref_count_delayed_disconnect_min_observers_can_connect_again_if_previous_subscription_terminated_from_subscribe_by_completion_and_enough_time_for_disconnect_has_passed() {
    let scheduler = TestScheduler::new();
    let (seen1, seen2) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let (terminated1, terminated2) = (Rc::new(Cell::new(false)), Rc::new(Cell::new(false)));

    // On initial subscription, the source will produce one value and will not complete.
    let connectable_source = SerialSingleNotificationConnectable::<i32>::new(Notification::OnNext(36));
    let ref_count = connectable(&connectable_source).ref_count_min_with_delay_on(2, ticks_to_duration(10), scheduler.scheduler());

    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        assert_eq!(0, seen1.get());
        assert_eq!(0, connectable_source.connection_count());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(36, seen1.get());
            assert_eq!(36, seen2.get());
            assert_eq!(1, connectable_source.connection_count());
            assert!(!connectable_source.connection(0).disposed());
            s2.dispose();
        }
        s1.dispose();
    }

    assert_eq!(1, connectable_source.connection_count());
    assert!(!connectable_source.connection(0).disposed());

    scheduler.advance_by(11);

    assert_eq!(1, connectable_source.connection_count());
    assert!(connectable_source.connection(0).disposed());

    seen1.set(0);
    seen2.set(0);
    terminated1.set(false);
    terminated2.set(false);

    // This time around, when Connect is called, all subscriptions after the preceding Connect will be
    // completed.
    connectable_source.set_notification_for_next_connect(Notification::OnCompleted);

    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        assert_eq!(1, connectable_source.connection_count());
        assert!(!terminated1.get());
        assert!(!terminated2.get());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(0, seen1.get());
            assert_eq!(0, seen2.get());
            assert!(terminated1.get());
            assert!(terminated2.get());
            assert_eq!(2, connectable_source.connection_count());
            assert!(!connectable_source.connection(1).disposed());
            s2.dispose();
        }
        s1.dispose();
    }

    assert_eq!(2, connectable_source.connection_count());
    assert!(!connectable_source.connection(1).disposed());

    scheduler.advance_by(11);

    assert_eq!(2, connectable_source.connection_count());
    assert!(connectable_source.connection(1).disposed());

    seen1.set(0);
    seen2.set(0);
    terminated1.set(false);
    terminated2.set(false);

    // Now we go back to the initial behaviour in which the source produces one value and does not complete.
    connectable_source.set_notification_for_next_connect(Notification::OnNext(42));

    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        assert!(!terminated1.get());
        assert_eq!(0, seen1.get());
        assert!(!terminated2.get());
        assert_eq!(2, connectable_source.connection_count());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(42, seen1.get());
            assert_eq!(42, seen2.get());
            assert!(!terminated1.get());
            assert!(!terminated2.get());
            assert_eq!(3, connectable_source.connection_count());
            assert!(!connectable_source.connection(2).disposed());
            s2.dispose();
        }
        s1.dispose();
    }
}

#[test]
fn ref_count_delayed_disconnect_min_observers_can_connect_again_if_previous_subscription_terminated_from_subscribe_by_completion_and_enough_time_for_disconnect_has_passed_with_pre_connect_notifications(
) {
    let scheduler = TestScheduler::new();
    let (seen1, seen2) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let (terminated1, terminated2) = (Rc::new(Cell::new(false)), Rc::new(Cell::new(false)));

    // On initial subscription, the source will produce one value and will not complete.
    let connectable_source = SerialConnectableIgnoringConnect::<i32>::new(BehaviorSubject::new(36).as_observable());
    let ref_count = connectable(&connectable_source).ref_count_min_with_delay_on(2, ticks_to_duration(10), scheduler.scheduler());

    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        // The SerialConnectableIgnoringConnect is unusual in that it can produce values before the
        // call to Connect. So we expect to see the value from the source, but not yet to
        // have seen a Connect call.
        assert_eq!(36, seen1.get());
        assert_eq!(0, connectable_source.connection_count());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(36, seen1.get());
            assert_eq!(36, seen2.get());
            assert_eq!(1, connectable_source.connection_count());
            assert!(!connectable_source.connection(0).disposed());
            s2.dispose();
        }
        s1.dispose();
    }

    assert_eq!(1, connectable_source.connection_count());
    assert!(!connectable_source.connection(0).disposed());

    scheduler.advance_by(11);

    assert_eq!(1, connectable_source.connection_count());
    assert!(connectable_source.connection(0).disposed());

    seen1.set(0);
    seen2.set(0);
    terminated1.set(false);
    terminated2.set(false);

    // This time around, the source will complete when subscribed to.
    connectable_source.set_source(Observable::empty::<i32>());

    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        // Again, the unusual behaviour of delivering notifications immediately from subscription
        // without waiting for the Connect means we see the initial termination immediately (and no
        // connection yet).
        assert!(terminated1.get());
        assert!(!terminated2.get());
        assert_eq!(1, connectable_source.connection_count());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(0, seen1.get());
            assert_eq!(0, seen2.get());
            assert!(terminated1.get());
            assert!(terminated2.get());

            // Since the initial subscription completed immediately, the observer count
            // never got above 1, so we do not expect a second connection
            assert_eq!(1, connectable_source.connection_count());
            assert!(connectable_source.connection(0).disposed());
            s2.dispose();
        }
        s1.dispose();
    }

    assert_eq!(1, connectable_source.connection_count());

    scheduler.advance_by(11);

    assert_eq!(1, connectable_source.connection_count());

    seen1.set(0);
    seen2.set(0);
    terminated1.set(false);
    terminated2.set(false);

    // Now we go back to the initial behaviour in which the source produces one value and does not complete.
    connectable_source.set_source(BehaviorSubject::new(42).as_observable());

    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        assert!(!terminated1.get());
        assert_eq!(42, seen1.get());
        assert!(!terminated2.get());
        assert_eq!(1, connectable_source.connection_count());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(42, seen1.get());
            assert_eq!(42, seen2.get());
            assert!(!terminated1.get());
            assert!(!terminated2.get());
            assert_eq!(2, connectable_source.connection_count());
            assert!(!connectable_source.connection(1).disposed());
            s2.dispose();
        }
        s1.dispose();
    }
}

#[test]
fn ref_count_delayed_disconnect_min_observers_does_not_connect_again_if_previous_subscription_terminated_from_subscribe_by_completion_but_not_enough_time_for_delayed_disconnect_has_passed(
) {
    let scheduler = TestScheduler::new();
    let (seen1, seen2) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let (terminated1, terminated2) = (Rc::new(Cell::new(false)), Rc::new(Cell::new(false)));

    // On initial subscription, the source will produce one value and will not complete.
    let connectable_source = SerialSingleNotificationConnectable::<i32>::new(Notification::OnNext(36));
    let ref_count = connectable(&connectable_source).ref_count_min_with_delay_on(2, ticks_to_duration(10), scheduler.scheduler());

    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        assert_eq!(0, seen1.get());
        assert_eq!(0, connectable_source.connection_count());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(36, seen1.get());
            assert_eq!(36, seen2.get());
            assert_eq!(1, connectable_source.connection_count());
            assert!(!connectable_source.connection(0).disposed());
            s2.dispose();
        }
        s1.dispose();
    }

    assert_eq!(1, connectable_source.connection_count());
    assert!(!connectable_source.connection(0).disposed());

    // For these initial subscriptions, we allow enough time for the delayed disconnect to occur, because
    // it's the resubscription after a source-induced completion that this test is interested in.
    scheduler.advance_by(11);

    assert_eq!(1, connectable_source.connection_count());
    assert!(connectable_source.connection(0).disposed());

    seen1.set(0);
    seen2.set(0);
    terminated1.set(false);
    terminated2.set(false);

    // Any further subscriptions will be completed on the next Connect.
    connectable_source.set_notification_for_next_connect(Notification::OnCompleted);

    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        assert_eq!(1, connectable_source.connection_count());
        assert!(!terminated1.get());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(0, seen1.get());
            assert_eq!(0, seen2.get());
            assert!(terminated1.get());
            assert!(terminated2.get());

            assert_eq!(2, connectable_source.connection_count());
            assert!(!connectable_source.connection(1).disposed());
            s2.dispose();
        }
        s1.dispose();
    }

    assert_eq!(2, connectable_source.connection_count());
    assert!(!connectable_source.connection(1).disposed());

    scheduler.advance_by(5);

    assert_eq!(2, connectable_source.connection_count());
    assert!(!connectable_source.connection(1).disposed());

    seen1.set(0);
    seen2.set(0);
    terminated1.set(false);
    terminated2.set(false);

    // To verify that individual subscriptions continue to be forwarded to the underlying source even
    // when no reconnect occurs, we arrange for subsequent subscriptions to get receive a single value.
    // (This is a slightly odd thing to do, but it's not RefCount's place to have opinions on how the
    // source should behave.)
    connectable_source.connection(1).replace_source(BehaviorSubject::new(42).as_observable());

    // The connection set up in the preceding section won't be torn down until the
    // specified disconnect delay has elapsed, so the expected behaviour if we try to establish
    // new subscriptions in that time is that their Subscribe will be passed through to the source,
    // and that we won't see any further connections. But now that the further subscriptions to the
    // source will result in a value (even though earlier subscriptions to the same source have been
    // completed) we expect these new subscriptions each to see the value.
    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        assert!(!terminated1.get());
        assert_eq!(42, seen1.get());
        assert!(!terminated2.get());
        assert_eq!(2, connectable_source.connection_count());
        assert!(!connectable_source.connection(1).disposed());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(42, seen1.get());
            assert_eq!(42, seen2.get());
            assert!(!terminated1.get());
            assert!(!terminated2.get());
            assert_eq!(2, connectable_source.connection_count());
            assert!(!connectable_source.connection(1).disposed());
            s2.dispose();
        }
        s1.dispose();
    }

    connectable_source.set_notification_for_next_connect(Notification::OnNext(99));

    // If we advanced by enough for the deferred disconnect to occur, it should be able to create a fresh
    // connection to the underlying source, at which point we'll see the value again.
    // We were at 5, so this takes us to 11 since the initial connection, but we don't expect that to be
    // enough, because the deferred disconnection should be relative to the most recent subscription.
    scheduler.advance_by(6);

    assert_eq!(2, connectable_source.connection_count());
    assert!(!connectable_source.connection(1).disposed());

    // Since the last subscription occurred at 5, advancing to 16 should trigger disconnection. And
    // since we're already up to 11, this should do it:
    scheduler.advance_by(5);

    assert_eq!(2, connectable_source.connection_count());
    assert!(connectable_source.connection(1).disposed());

    seen1.set(0);
    seen2.set(0);
    terminated1.set(false);
    terminated2.set(false);

    {
        let s1 = subscribe_seen(&ref_count, &seen1, &terminated1);
        assert_eq!(0, seen1.get());
        assert_eq!(2, connectable_source.connection_count());
        assert!(connectable_source.connection(1).disposed());
        {
            let s2 = subscribe_seen(&ref_count, &seen2, &terminated2);
            assert_eq!(99, seen1.get());
            assert_eq!(99, seen2.get());
            assert!(!terminated1.get());
            assert!(!terminated2.get());
            assert_eq!(3, connectable_source.connection_count());
            assert!(!connectable_source.connection(2).disposed());
            s2.dispose();
        }
        s1.dispose();
    }
}
