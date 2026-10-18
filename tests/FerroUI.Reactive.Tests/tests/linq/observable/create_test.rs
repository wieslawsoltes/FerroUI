//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/CreateTest.cs` of Rx.NET.
//! Adapted: `Create(Func<IObserver<T>, Action>)` is `Observable::create_with_action` and
//! `Create(Func<IObserver<T>, IDisposable>)` is `Observable::create`; an exception thrown by the
//! subscription or by the observer is a panic, checked with `assert_throws`; the `stopped` flag of
//! `Create_Dispose` is a shared `Cell`. `ObservableEx.Create` of an iterator is
//! `Observable::create_iterator` (`create_iterator_void` without an observer, the `try_` forms
//! for an iterator that throws); a C# iterator method is a hand-written iterator whose steps run
//! at the same pulls, its yielded `IObservable<object>` sequences are sequences of `()`, and
//! `AssertEqual` of two sequences compares their notifications, collected during the subscription
//! (the sequences run on the current thread).

use ferroui_reactive::concurrency::SchedulerExt;
use ferroui_reactive::disposables::{BooleanDisposable, Disposable, ICancelable};
use ferroui_reactive::linq::ListObservable;
use ferroui_reactive::{IDisposable, IObservable, IObserver, Notification, NotificationKind, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, assert_throws, new_error, ticks_to_duration, ReactiveTest as RT, TestScheduler};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

#[test]
fn create_next() {
    let scheduler = TestScheduler::new();

    let res = scheduler.start(|| {
        Observable::create_with_action(|o: Rc<dyn IObserver<i32>>| {
            o.on_next(1);
            o.on_next(2);
            || {}
        })
    });

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(200, 2));
}

#[test]
fn create_completed() {
    let scheduler = TestScheduler::new();

    let res = scheduler.start(|| {
        Observable::create_with_action(|o: Rc<dyn IObserver<i32>>| {
            o.on_completed();
            o.on_next(100);
            o.on_error(new_error("ex"));
            o.on_completed();
            || {}
        })
    });

    assert_messages!(res.messages(), RT::on_completed::<i32>(200));
}

#[test]
fn create_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let error = ex.clone();
    let res = scheduler.start(move || {
        Observable::create_with_action(move |o: Rc<dyn IObserver<i32>>| {
            o.on_error(error.clone());
            o.on_next(100);
            o.on_error(new_error("other"));
            o.on_completed();
            || {}
        })
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex));
}

#[test]
fn create_exception() {
    assert_throws(|| {
        Observable::create_with_action(|_o: Rc<dyn IObserver<i32>>| -> fn() { panic!("InvalidOperationException") }).subscribe_ignore();
    });
}

#[test]
fn create_dispose() {
    let scheduler = TestScheduler::new();

    let s = scheduler.clone();
    let res = scheduler.start(move || {
        Observable::create_with_action(move |o: Rc<dyn IObserver<i32>>| {
            let stopped = Rc::new(Cell::new(false));

            o.on_next(1);
            o.on_next(2);
            for (ticks, value) in [(600, 3), (700, 4), (900, 5), (1100, 6)] {
                let (o, stopped) = (o.clone(), stopped.clone());
                s.scheduler().schedule_action_after(ticks_to_duration(ticks), move || {
                    if !stopped.get() {
                        o.on_next(value);
                    }
                });
            }

            move || stopped.set(true)
        })
    });

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(200, 2), RT::on_next(800, 3), RT::on_next(900, 4));
}

#[test]
fn create_observer_throws() {
    assert_throws(|| {
        Observable::create_with_action(|o: Rc<dyn IObserver<i32>>| {
            o.on_next(1);
            || {}
        })
        .subscribe_next(|_| panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::create_with_action(|o: Rc<dyn IObserver<i32>>| {
            o.on_error(new_error("ex"));
            || {}
        })
        .subscribe_next_error(|_| {}, |_| panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::create_with_action(|o: Rc<dyn IObserver<i32>>| {
            o.on_completed();
            || {}
        })
        .subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));
    });
}

#[test]
fn create_with_disposable_next() {
    let scheduler = TestScheduler::new();

    let res = scheduler.start(|| {
        Observable::create(|o: Rc<dyn IObserver<i32>>| {
            o.on_next(1);
            o.on_next(2);
            Disposable::empty()
        })
    });

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(200, 2));
}

#[test]
fn create_with_disposable_completed() {
    let scheduler = TestScheduler::new();

    let res = scheduler.start(|| {
        Observable::create(|o: Rc<dyn IObserver<i32>>| {
            o.on_completed();
            o.on_next(100);
            o.on_error(new_error("ex"));
            o.on_completed();
            Disposable::empty()
        })
    });

    assert_messages!(res.messages(), RT::on_completed::<i32>(200));
}

#[test]
fn create_with_disposable_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let error = ex.clone();
    let res = scheduler.start(move || {
        Observable::create(move |o: Rc<dyn IObserver<i32>>| {
            o.on_error(error.clone());
            o.on_next(100);
            o.on_error(new_error("other"));
            o.on_completed();
            Disposable::empty()
        })
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex));
}

#[test]
fn create_with_disposable_exception() {
    assert_throws(|| {
        Observable::create(|_o: Rc<dyn IObserver<i32>>| -> Rc<dyn IDisposable> { panic!("InvalidOperationException") }).subscribe_ignore();
    });
}

#[test]
fn create_with_disposable_dispose() {
    let scheduler = TestScheduler::new();

    let s = scheduler.clone();
    let res = scheduler.start(move || {
        Observable::create(move |o: Rc<dyn IObserver<i32>>| {
            let d = BooleanDisposable::new();

            o.on_next(1);
            o.on_next(2);
            for (ticks, value) in [(600, 3), (700, 4), (900, 5), (1100, 6)] {
                let (o, d) = (o.clone(), d.clone());
                s.scheduler().schedule_action_after(ticks_to_duration(ticks), move || {
                    if !d.is_disposed() {
                        o.on_next(value);
                    }
                });
            }

            let d: Rc<dyn IDisposable> = d;
            d
        })
    });

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(200, 2), RT::on_next(800, 3), RT::on_next(900, 4));
}

#[test]
fn create_with_disposable_observer_throws() {
    assert_throws(|| {
        Observable::create(|o: Rc<dyn IObserver<i32>>| {
            o.on_next(1);
            Disposable::empty()
        })
        .subscribe_next(|_| panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::create(|o: Rc<dyn IObserver<i32>>| {
            o.on_error(new_error("ex"));
            Disposable::empty()
        })
        .subscribe_next_error(|_| {}, |_| panic!("InvalidOperationException"));
    });
    assert_throws(|| {
        Observable::create(|o: Rc<dyn IObserver<i32>>| {
            o.on_completed();
            Disposable::empty()
        })
        .subscribe_all(|_| {}, |_| {}, || panic!("InvalidOperationException"));
    });
}

/// The yielded sequence of `xs.Select(x => new object())`: the values of `xs` as `()`.
fn objects(xs: &Rc<dyn IObservable<i32>>) -> Rc<dyn IObservable<()>> {
    xs.select(|_| ())
}

/// The notifications of `source`, collected during the subscription (the sequences of the
/// scenarios run on the current thread), for `AssertEqual` of two sequences.
fn notifications<T: 'static>(source: &Rc<dyn IObservable<T>>) -> Vec<Notification<T>> {
    let list = Rc::new(RefCell::new(Vec::new()));
    {
        let list = list.clone();
        source.materialize().subscribe_next(move |n| list.borrow_mut().push(n));
    }
    let result = list.take();
    result
}

/// `InvalidOperationException`, thrown by the iterator method of `Iterate_Void_Func_Throw`.
#[derive(Debug)]
struct InvalidOperationException;

impl fmt::Display for InvalidOperationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Operation is not valid due to the current state of the object.")
    }
}

impl std::error::Error for InvalidOperationException {}

/// `ToIterate_Complete`.
fn to_iterate_complete(
    xs: Rc<dyn IObservable<i32>>,
    ys: Rc<dyn IObservable<i32>>,
    zs: Rc<dyn IObservable<i32>>,
    observer: Rc<dyn IObserver<i32>>,
) -> impl Iterator<Item = Rc<dyn IObservable<()>>> {
    let mut step = 0;
    std::iter::from_fn(move || {
        step += 1;
        match step {
            1 => {
                observer.on_next(1);
                Some(objects(&xs))
            }
            2 => {
                observer.on_next(2);
                Some(objects(&ys))
            }
            3 => {
                observer.on_next(3);
                observer.on_completed();
                Some(objects(&zs))
            }
            4 => {
                observer.on_next(4);
                None
            }
            _ => None,
        }
    })
}

#[test]
fn iterate_complete() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_next(40, 4), RT::on_completed(50)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_completed(30)]);

    let zs = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(40, 4),
        RT::on_next(50, 5),
        RT::on_completed(60),
    ]);

    let (a, b, c) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || {
        Observable::create_iterator(move |observer: Rc<dyn IObserver<i32>>| to_iterate_complete(a.clone(), b.clone(), c.clone(), observer))
    });

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(250, 2), RT::on_next(280, 3), RT::on_completed(280));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(250, 280));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(280, 280));
}

/// `ToIterate_Complete_Implicit`.
fn to_iterate_complete_implicit(
    xs: Rc<dyn IObservable<i32>>,
    ys: Rc<dyn IObservable<i32>>,
    zs: Rc<dyn IObservable<i32>>,
    observer: Rc<dyn IObserver<i32>>,
) -> impl Iterator<Item = Rc<dyn IObservable<()>>> {
    let mut step = 0;
    std::iter::from_fn(move || {
        step += 1;
        match step {
            1 => {
                observer.on_next(1);
                Some(objects(&xs))
            }
            2 => {
                observer.on_next(2);
                Some(objects(&ys))
            }
            3 => {
                observer.on_next(3);
                Some(objects(&zs))
            }
            4 => {
                observer.on_next(4);
                None
            }
            _ => None,
        }
    })
}

#[test]
fn iterate_complete_implicit() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_next(40, 4), RT::on_completed(50)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_completed(30)]);

    let zs = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(40, 4),
        RT::on_next(50, 5),
        RT::on_completed(60),
    ]);

    let (a, b, c) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || {
        Observable::create_iterator(move |observer: Rc<dyn IObserver<i32>>| to_iterate_complete_implicit(a.clone(), b.clone(), c.clone(), observer))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(200, 1),
        RT::on_next(250, 2),
        RT::on_next(280, 3),
        RT::on_next(340, 4),
        RT::on_completed(340)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(250, 280));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(280, 340));
}

/// `ToIterate_Throw`: the iterator throws `ex` after the second sequence (upstream's
/// `if (xs != null)` always holds), so its yield of `zs` and the steps after it never run.
fn to_iterate_throw(
    xs: Rc<dyn IObservable<i32>>,
    ys: Rc<dyn IObservable<i32>>,
    _zs: Rc<dyn IObservable<i32>>,
    observer: Rc<dyn IObserver<i32>>,
    ex: ObservableError,
) -> impl Iterator<Item = Result<Rc<dyn IObservable<()>>, ObservableError>> {
    let mut step = 0;
    std::iter::from_fn(move || {
        step += 1;
        match step {
            1 => {
                observer.on_next(1);
                Some(Ok(objects(&xs)))
            }
            2 => {
                observer.on_next(2);
                Some(Ok(objects(&ys)))
            }
            3 => {
                observer.on_next(3);
                Some(Err(ex.clone()))
            }
            // An iterator that threw has ended.
            _ => None,
        }
    })
}

#[test]
fn iterate_iterator_throw() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_next(40, 4), RT::on_completed(50)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_completed(30)]);

    let zs = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(40, 4),
        RT::on_next(50, 5),
        RT::on_completed(60),
    ]);

    let ex = new_error("ex");

    let (a, b, c, e) = (xs.as_observable(), ys.as_observable(), zs.as_observable(), ex.clone());
    let res = scheduler.start(move || {
        Observable::try_create_iterator(move |observer: Rc<dyn IObserver<i32>>| {
            Ok::<_, ObservableError>(to_iterate_throw(a.clone(), b.clone(), c.clone(), observer, e.clone()))
        })
    });

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(250, 2), RT::on_next(280, 3), RT::on_error::<i32>(280, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(250, 280));

    assert_subscriptions!(zs.subscriptions());
}

/// `ToIterate_Error`.
fn to_iterate_error(
    xs: Rc<dyn IObservable<i32>>,
    ys: Rc<dyn IObservable<i32>>,
    zs: Rc<dyn IObservable<i32>>,
    observer: Rc<dyn IObserver<i32>>,
    ex: ObservableError,
) -> impl Iterator<Item = Rc<dyn IObservable<()>>> {
    let mut step = 0;
    std::iter::from_fn(move || {
        step += 1;
        match step {
            1 => {
                observer.on_next(1);
                Some(objects(&xs))
            }
            2 => {
                observer.on_next(2);
                observer.on_error(ex.clone());
                Some(objects(&ys))
            }
            3 => {
                observer.on_next(3);
                Some(objects(&zs))
            }
            4 => {
                observer.on_next(4);
                observer.on_completed();
                None
            }
            _ => None,
        }
    })
}

#[test]
fn iterate_iterator_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_next(40, 4), RT::on_completed(50)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_completed(30)]);

    let zs = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(40, 4),
        RT::on_next(50, 5),
        RT::on_completed(60),
    ]);

    let (a, b, c, e) = (xs.as_observable(), ys.as_observable(), zs.as_observable(), ex.clone());
    let res = scheduler.start(move || {
        Observable::create_iterator(move |observer: Rc<dyn IObserver<i32>>| to_iterate_error(a.clone(), b.clone(), c.clone(), observer, e.clone()))
    });

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(250, 2), RT::on_error::<i32>(250, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(250, 250));

    assert_subscriptions!(zs.subscriptions());
}

/// `ToIterate_Complete_Dispose`.
fn to_iterate_complete_dispose(
    xs: Rc<dyn IObservable<i32>>,
    ys: Rc<dyn IObservable<i32>>,
    zs: Rc<dyn IObservable<i32>>,
    observer: Rc<dyn IObserver<i32>>,
) -> impl Iterator<Item = Rc<dyn IObservable<()>>> {
    let mut step = 0;
    std::iter::from_fn(move || {
        step += 1;
        match step {
            1 => {
                observer.on_next(1);
                Some(objects(&xs))
            }
            2 => {
                observer.on_next(2);
                Some(objects(&ys))
            }
            3 => {
                observer.on_next(3);
                Some(objects(&zs))
            }
            4 => {
                observer.on_next(4);
                None
            }
            _ => None,
        }
    })
}

#[test]
fn iterate_complete_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_next(40, 4), RT::on_completed(50)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_completed(30)]);

    let zs = scheduler.create_cold_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 5),
        RT::on_next(600, 6),
        RT::on_next(700, 7),
        RT::on_next(800, 8),
        RT::on_next(900, 9),
        RT::on_next(1000, 10),
    ]);

    let (a, b, c) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || {
        Observable::create_iterator(move |observer: Rc<dyn IObserver<i32>>| to_iterate_complete_dispose(a.clone(), b.clone(), c.clone(), observer))
    });

    assert_messages!(res.messages(), RT::on_next(200, 1), RT::on_next(250, 2), RT::on_next(280, 3));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(250, 280));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(280, 1000));
}

#[test]
fn iterator_scenario() {
    let xs = Observable::create_iterator(|o: Rc<dyn IObserver<i32>>| IteratorScenario::new(100, 1000, o));

    assert_eq!(notifications(&Observable::to_observable(vec![100, 1000])), notifications(&xs));
}

/// `_IteratorScenario`: the lists of two ranges, and their last values sent to `results`.
struct IteratorScenario {
    x: usize,
    y: usize,
    results: Rc<dyn IObserver<i32>>,
    step: usize,
    xs: Option<Rc<ListObservable<i32>>>,
    ys: Option<Rc<ListObservable<i32>>>,
}

impl IteratorScenario {
    fn new(x: usize, y: usize, results: Rc<dyn IObserver<i32>>) -> Self {
        Self { x, y, results, step: 0, xs: None, ys: None }
    }
}

impl Iterator for IteratorScenario {
    type Item = Rc<dyn IObservable<()>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.step += 1;
        match self.step {
            1 => {
                let xs = Observable::range(1, self.x).to_list_observable();
                self.xs = Some(xs.clone());
                let xs: Rc<dyn IObservable<()>> = xs;
                Some(xs)
            }
            2 => {
                let xs = self.xs.as_ref().expect("the first list was yielded");
                self.results.on_next(xs.value().expect("the first list has ended with a value"));

                let ys = Observable::range(1, self.y).to_list_observable();
                self.ys = Some(ys.clone());
                let ys: Rc<dyn IObservable<()>> = ys;
                Some(ys)
            }
            3 => {
                let ys = self.ys.as_ref().expect("the second list was yielded");
                self.results.on_next(ys.value().expect("the second list has ended with a value"));
                None
            }
            _ => None,
        }
    }
}

/// `ToIterate_Void_Complete`.
fn to_iterate_void_complete(
    xs: Rc<dyn IObservable<i32>>,
    ys: Rc<dyn IObservable<i32>>,
    zs: Rc<dyn IObservable<i32>>,
) -> impl Iterator<Item = Rc<dyn IObservable<()>>> {
    let mut step = 0;
    std::iter::from_fn(move || {
        step += 1;
        match step {
            1 => Some(objects(&xs)),
            2 => Some(objects(&ys)),
            3 => Some(objects(&zs)),
            _ => None,
        }
    })
}

#[test]
fn iterate_void_complete() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_next(40, 4), RT::on_completed(50)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_completed(30)]);

    let zs = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(40, 4),
        RT::on_next(50, 5),
        RT::on_completed(60),
    ]);

    let (a, b, c) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || Observable::create_iterator_void(move || to_iterate_void_complete(a.clone(), b.clone(), c.clone())));

    assert_messages!(res.messages(), RT::on_completed::<()>(340));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(250, 280));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(280, 340));
}

/// `ToIterate_Void_Complete_Implicit`.
fn to_iterate_void_complete_implicit(
    xs: Rc<dyn IObservable<i32>>,
    ys: Rc<dyn IObservable<i32>>,
    zs: Rc<dyn IObservable<i32>>,
) -> impl Iterator<Item = Rc<dyn IObservable<()>>> {
    let mut step = 0;
    std::iter::from_fn(move || {
        step += 1;
        match step {
            1 => Some(objects(&xs)),
            2 => Some(objects(&ys)),
            3 => Some(objects(&zs)),
            _ => None,
        }
    })
}

#[test]
fn iterate_void_complete_implicit() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_next(40, 4), RT::on_completed(50)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_completed(30)]);

    let zs = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(40, 4),
        RT::on_next(50, 5),
        RT::on_completed(60),
    ]);

    let (a, b, c) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || Observable::create_iterator_void(move || to_iterate_void_complete_implicit(a.clone(), b.clone(), c.clone())));

    assert_messages!(res.messages(), RT::on_completed::<()>(340));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(250, 280));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(280, 340));
}

/// `ToIterate_Void_Throw`: the iterator throws `ex` after the second sequence (upstream's
/// `if (xs != null)` always holds), so its yield of `zs` never runs.
fn to_iterate_void_throw(
    xs: Rc<dyn IObservable<i32>>,
    ys: Rc<dyn IObservable<i32>>,
    _zs: Rc<dyn IObservable<i32>>,
    ex: ObservableError,
) -> impl Iterator<Item = Result<Rc<dyn IObservable<()>>, ObservableError>> {
    let mut step = 0;
    std::iter::from_fn(move || {
        step += 1;
        match step {
            1 => Some(Ok(objects(&xs))),
            2 => Some(Ok(objects(&ys))),
            3 => Some(Err(ex.clone())),
            // An iterator that threw has ended.
            _ => None,
        }
    })
}

#[test]
fn iterate_void_iterator_throw() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_next(40, 4), RT::on_completed(50)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_completed(30)]);

    let zs = scheduler.create_cold_observable(vec![
        RT::on_next(10, 1),
        RT::on_next(20, 2),
        RT::on_next(30, 3),
        RT::on_next(40, 4),
        RT::on_next(50, 5),
        RT::on_completed(60),
    ]);

    let ex = new_error("ex");

    let (a, b, c, e) = (xs.as_observable(), ys.as_observable(), zs.as_observable(), ex.clone());
    let res = scheduler.start(move || {
        Observable::try_create_iterator_void(move || Ok::<_, ObservableError>(to_iterate_void_throw(a.clone(), b.clone(), c.clone(), e.clone())))
    });

    assert_messages!(res.messages(), RT::on_error::<()>(280, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(250, 280));

    assert_subscriptions!(zs.subscriptions());
}

/// `ToIterate_Void_Complete_Dispose`.
fn to_iterate_void_complete_dispose(
    xs: Rc<dyn IObservable<i32>>,
    ys: Rc<dyn IObservable<i32>>,
    zs: Rc<dyn IObservable<i32>>,
) -> impl Iterator<Item = Rc<dyn IObservable<()>>> {
    let mut step = 0;
    std::iter::from_fn(move || {
        step += 1;
        match step {
            1 => Some(objects(&xs)),
            2 => Some(objects(&ys)),
            3 => Some(objects(&zs)),
            _ => None,
        }
    })
}

#[test]
fn iterate_void_complete_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_next(40, 4), RT::on_completed(50)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_completed(30)]);

    let zs = scheduler.create_cold_observable(vec![
        RT::on_next(100, 1),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 4),
        RT::on_next(500, 5),
        RT::on_next(600, 6),
        RT::on_next(700, 7),
        RT::on_next(800, 8),
        RT::on_next(900, 9),
        RT::on_next(1000, 10),
    ]);

    let (a, b, c) = (xs.as_observable(), ys.as_observable(), zs.as_observable());
    let res = scheduler.start(move || Observable::create_iterator_void(move || to_iterate_void_complete_dispose(a.clone(), b.clone(), c.clone())));

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(250, 280));

    assert_subscriptions!(zs.subscriptions(), RT::subscribe(280, 1000));
}

#[test]
fn iterate_void_func_throw() {
    let scheduler = TestScheduler::new();

    let obs = scheduler.start(|| {
        Observable::try_create_iterator_void(|| -> Result<Vec<Result<Rc<dyn IObservable<()>>, ObservableError>>, ObservableError> {
            Err(Rc::new(InvalidOperationException) as ObservableError)
        })
    });

    let messages = obs.messages();
    assert_eq!(1, messages.len());

    let notification = &messages[0].value;
    assert_eq!(NotificationKind::OnError, notification.kind());
    assert!(notification.exception().is_some_and(|error| error.downcast_ref::<InvalidOperationException>().is_some()));
}

/// `_IteratorScenario_Void`: the lists of two ranges.
fn iterator_scenario_void_sources(x: usize, y: usize) -> impl Iterator<Item = Rc<dyn IObservable<()>>> {
    let mut step = 0;
    std::iter::from_fn(move || {
        step += 1;
        match step {
            1 => {
                let xs: Rc<dyn IObservable<()>> = Observable::range(1, x).to_list_observable();
                Some(xs)
            }
            2 => {
                let ys: Rc<dyn IObservable<()>> = Observable::range(1, y).to_list_observable();
                Some(ys)
            }
            _ => None,
        }
    })
}

#[test]
fn iterator_scenario_void() {
    let xs = Observable::create_iterator_void(|| iterator_scenario_void_sources(100, 1000));

    assert_eq!(notifications(&Observable::to_observable(Vec::<()>::new())), notifications(&xs));
}

// Not ported: Create_ArgumentChecking - argument validation
// Not ported: Create_NullCoalescingAction - a null action returned by the subscription (the closure of the port returns an action, which cannot be null)
// Not ported: CreateWithDisposable_ArgumentChecking - argument validation
// Not ported: CreateWithDisposable_NullCoalescingAction - a null disposable returned by the subscription (the closure of the port returns an `Rc`, which cannot be null)
// Not ported: Iterate_ArgumentChecking - argument validation
// Not ported: Iterate_Void_ArgumentChecking - argument validation
