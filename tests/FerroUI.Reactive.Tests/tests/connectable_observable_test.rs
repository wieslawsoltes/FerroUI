//! Ported from `Tests.System.Reactive/Tests/ConnectableObservableTest.cs` of Rx.NET.
//! Adapted: the test helpers `MySubject.cs` and `ConnectableObservable.cs` are ported below; the
//! `Multicast(subject)` they and the last test use is the crate's `ConnectableObservable::new`
//! (what upstream's `Multicast` returns), as the `Multicast` operator itself is not ported yet.

use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{ConnectableObservable as MulticastObservable, IConnectableObservable, ISubjectHandle, Subject};
use ferroui_reactive::{IDisposable, IObservable, IObserver, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, ReactiveTest as RT, TestScheduler};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// `source.Multicast(subject)`.
fn multicast<T: 'static>(source: Rc<dyn IObservable<T>>, subject: Rc<dyn ISubjectHandle<T, T>>) -> Rc<MulticastObservable<T, T>> {
    MulticastObservable::new(source, subject)
}

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

    fn dispose_on(&self, value: i32, disposable: Rc<dyn IDisposable>) {
        self.dispose_on.borrow_mut().insert(value, disposable);
    }

    /// `SubscribeCount`.
    #[allow(dead_code)]
    fn subscribe_count(&self) -> i32 {
        self.subscribe_count.get()
    }

    /// `Disposed`.
    #[allow(dead_code)]
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
    o: Rc<MulticastObservable<T, T>>,
}

impl<T: 'static> ConnectableObservable<T> {
    fn new(o: Rc<dyn IObservable<T>>, s: Rc<dyn ISubjectHandle<T, T>>) -> Rc<Self> {
        Rc::new(Self { o: multicast(o, s) })
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

#[test]
fn connectable_observable_creation() {
    let y = Rc::new(Cell::new(0));

    let s2 = Subject::<i32>::new();
    let co2 = ConnectableObservable::<i32>::new(Observable::return_(1), s2);

    {
        let y = y.clone();
        let source: Rc<dyn IObservable<i32>> = co2.clone();
        source.subscribe_next(move |x| y.set(x));
    }
    assert_ne!(1, y.get());

    co2.connect();
    assert_eq!(1, y.get());
}

#[test]
fn connectable_observable_connected() {
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
    let _disconnect = conn.connect();

    let source: Rc<dyn IObservable<i32>> = conn.clone();
    let res = scheduler.start(move || source);

    assert_messages!(
        res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(240, 4),
        RT::on_completed::<i32>(250),
    );
}

#[test]
fn connectable_observable_not_connected() {
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

    let source: Rc<dyn IObservable<i32>> = conn.clone();
    let res = scheduler.start(move || source);

    assert_messages!(res.messages());
}

#[test]
fn connectable_observable_disconnected() {
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
    let disconnect = conn.connect();
    disconnect.dispose();

    let source: Rc<dyn IObservable<i32>> = conn.clone();
    let res = scheduler.start(move || source);

    assert_messages!(res.messages());
}

#[test]
fn connectable_observable_disconnect_future() {
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
    subject.dispose_on(3, conn.connect());

    let source: Rc<dyn IObservable<i32>> = conn.clone();
    let res = scheduler.start(move || source);

    assert_messages!(res.messages(), RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3));
}

#[test]
fn connectable_observable_multiple_non_overlapped_connections() {
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

    let subject = Subject::<i32>::new();

    let conn = multicast::<i32>(xs.as_observable(), subject);

    let connect_at = |time: i64, c: &Rc<RefCell<Option<Rc<dyn IDisposable>>>>| {
        let (conn, c) = (conn.clone(), c.clone());
        scheduler.schedule_absolute_action(time, move || *c.borrow_mut() = Some(conn.connect()));
    };
    let dispose_at = |time: i64, c: &Rc<RefCell<Option<Rc<dyn IDisposable>>>>| {
        let c = c.clone();
        scheduler.schedule_absolute_action(time, move || {
            let d = c.borrow().clone().expect("the connection is assigned");
            d.dispose();
        });
    };

    let c1: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
    connect_at(225, &c1);
    dispose_at(241, &c1);
    dispose_at(245, &c1); // idempotency test
    dispose_at(251, &c1); // idempotency test
    dispose_at(260, &c1); // idempotency test

    let c2: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
    connect_at(249, &c2);
    dispose_at(255, &c2);
    dispose_at(265, &c2); // idempotency test
    dispose_at(280, &c2); // idempotency test

    let c3: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
    connect_at(275, &c3);
    dispose_at(295, &c3);

    let source: Rc<dyn IObservable<i32>> = conn.clone();
    let res = scheduler.start(move || source);

    assert_messages!(res.messages(), RT::on_next(230, 3), RT::on_next(240, 4), RT::on_next(250, 5), RT::on_next(280, 8), RT::on_next(290, 9));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(225, 241), RT::subscribe(249, 255), RT::subscribe(275, 295));
}

// Not ported: ConnectableObservable_ArgumentChecking - argument validation
