//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SelectManyTest.cs` of Rx.NET: the forms
//! on sequences (the forms on enumerables are in `select_many_iter_test`).
//! Adapted: `SelectMany(IObservable<TOther>)` is `select_many_to`; the query operator form
//! (`from x in xs from y in ... select ...`) and `SelectMany(collectionSelector, resultSelector)`
//! are `select_many_with` (`select_many_with_index_with` with indexes); the forms with three
//! selectors are `select_many_notifications`; a selector that throws is the `try_` form returning
//! the error. The upstream tests of the forms without and with an index (`SelectMany_X` and
//! `SelectManyWithIndex_X`) share their scenarios: each scenario is a function run by both tests,
//! with the form chosen by `indexed`. The anonymous values are tuples; the inner sequences of a
//! sequence of sequences are `Rc<dyn IObservable<i32>>` handles of the test observables.
//! The forms on tasks are `select_many_async` (`_with_index`, `_with_token`, `_with` for a result
//! selector, `try_` for selectors that throw). `Task.Factory.StartNew` is `tasks::run`, whose
//! token argument the port does not have; `ToEnumerable()` is `block_on` of
//! `to_array().to_task()`. A task that throws `ex` fails with `ex` once its work has run, so that
//! the error is the same value; `ManualResetEvent`s that only wait for the end are the flags of
//! the subscription (the continuations of the tasks run when the tasks end, on the test thread);
//! `token.Register` is `register_local`.

use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{IDisposable, IObservable, IObserver, Notification, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{
    assert_messages, assert_subscriptions, new_error, ticks_to_duration, ColdObservable, HotObservable, ReactiveTest as RT, Recorded, TestScheduler,
};
use ferroui_system::threading::tasks::{block_on, run, Task, TaskCanceledError, TaskCompletionSource};
use ferroui_system::threading::CancellationToken;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;

type Inner = Rc<dyn IObservable<i32>>;

/// `SelectMany(selector)`, or `SelectMany((x, _) => selector(x))` when `indexed`.
fn many<T: 'static, U: 'static>(xs: Rc<dyn IObservable<T>>, indexed: bool, selector: impl Fn(T) -> Rc<dyn IObservable<U>> + 'static) -> Rc<dyn IObservable<U>> {
    if indexed {
        xs.select_many_with_index(move |x, _| selector(x))
    } else {
        xs.select_many(selector)
    }
}

/// [`many`] with a selector that may fail.
fn try_many<T: 'static, U: 'static>(
    xs: Rc<dyn IObservable<T>>,
    indexed: bool,
    selector: impl Fn(T) -> Result<Rc<dyn IObservable<U>>, ObservableError> + 'static,
) -> Rc<dyn IObservable<U>> {
    if indexed {
        xs.try_select_many_with_index(move |x, _| selector(x))
    } else {
        xs.try_select_many(selector)
    }
}

/// `SelectMany(collectionSelector, resultSelector)` (the query operator form), or
/// `SelectMany((x, _) => collectionSelector(x), (x, _, y, __) => resultSelector(x, y))` when
/// `indexed`.
fn query<C: 'static>(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    collection_selector: impl Fn(i32) -> Rc<dyn IObservable<C>> + 'static,
    result_selector: impl Fn(i32, C) -> i32 + 'static,
) -> Rc<dyn IObservable<i32>> {
    if indexed {
        xs.select_many_with_index_with(move |x, _| collection_selector(x), move |x, _, y, _| result_selector(x, y))
    } else {
        xs.select_many_with(collection_selector, result_selector)
    }
}

/// [`query`] with selectors that may fail.
fn try_query<C: 'static>(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    collection_selector: impl Fn(i32) -> Result<Rc<dyn IObservable<C>>, ObservableError> + 'static,
    result_selector: impl Fn(i32, C) -> Result<i32, ObservableError> + 'static,
) -> Rc<dyn IObservable<i32>> {
    if indexed {
        xs.try_select_many_with_index_with(move |x, _| collection_selector(x), move |x, _, y, _| result_selector(x, y))
    } else {
        xs.try_select_many_with(collection_selector, result_selector)
    }
}

/// `SelectMany(onNext, onError, onCompleted)`, or `SelectMany((x, _) => onNext(x), onError,
/// onCompleted)` when `indexed`.
fn triple(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    on_next: impl Fn(i32) -> Inner + 'static,
    on_error: impl Fn(ObservableError) -> Inner + 'static,
    on_completed: impl Fn() -> Inner + 'static,
) -> Inner {
    if indexed {
        xs.select_many_notifications_with_index(move |x, _| on_next(x), on_error, on_completed)
    } else {
        xs.select_many_notifications(on_next, on_error, on_completed)
    }
}

/// [`triple`] with selectors that may fail.
fn try_triple(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    on_next: impl Fn(i32) -> Result<Inner, ObservableError> + 'static,
    on_error: impl Fn(ObservableError) -> Result<Inner, ObservableError> + 'static,
    on_completed: impl Fn() -> Result<Inner, ObservableError> + 'static,
) -> Inner {
    if indexed {
        xs.try_select_many_notifications_with_index(move |x, _| on_next(x), on_error, on_completed)
    } else {
        xs.try_select_many_notifications(on_next, on_error, on_completed)
    }
}

#[test]
fn select_many_then_complete_complete() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 4), RT::on_next(200, 2), RT::on_next(300, 3), RT::on_next(400, 1), RT::on_completed(500)]);

    let ys = scheduler.create_cold_observable(vec![
        RT::on_next(50, "foo"),
        RT::on_next(100, "bar"),
        RT::on_next(150, "baz"),
        RT::on_next(200, "qux"),
        RT::on_completed(250),
    ]);

    let (source, other) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.select_many_to(other));

    assert_messages!(
        res.messages(),
        RT::on_next(350, "foo"),
        RT::on_next(400, "bar"),
        RT::on_next(450, "baz"),
        RT::on_next(450, "foo"),
        RT::on_next(500, "qux"),
        RT::on_next(500, "bar"),
        RT::on_next(550, "baz"),
        RT::on_next(550, "foo"),
        RT::on_next(600, "qux"),
        RT::on_next(600, "bar"),
        RT::on_next(650, "baz"),
        RT::on_next(650, "foo"),
        RT::on_next(700, "qux"),
        RT::on_next(700, "bar"),
        RT::on_next(750, "baz"),
        RT::on_next(800, "qux"),
        RT::on_completed(850)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 700));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(300, 550), RT::subscribe(400, 650), RT::subscribe(500, 750), RT::subscribe(600, 850));
}

#[test]
fn select_many_then_complete_complete_2() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 4), RT::on_next(200, 2), RT::on_next(300, 3), RT::on_next(400, 1), RT::on_completed(700)]);

    let ys = scheduler.create_cold_observable(vec![
        RT::on_next(50, "foo"),
        RT::on_next(100, "bar"),
        RT::on_next(150, "baz"),
        RT::on_next(200, "qux"),
        RT::on_completed(250),
    ]);

    let (source, other) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.select_many_to(other));

    assert_messages!(
        res.messages(),
        RT::on_next(350, "foo"),
        RT::on_next(400, "bar"),
        RT::on_next(450, "baz"),
        RT::on_next(450, "foo"),
        RT::on_next(500, "qux"),
        RT::on_next(500, "bar"),
        RT::on_next(550, "baz"),
        RT::on_next(550, "foo"),
        RT::on_next(600, "qux"),
        RT::on_next(600, "bar"),
        RT::on_next(650, "baz"),
        RT::on_next(650, "foo"),
        RT::on_next(700, "qux"),
        RT::on_next(700, "bar"),
        RT::on_next(750, "baz"),
        RT::on_next(800, "qux"),
        RT::on_completed(900)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 900));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(300, 550), RT::subscribe(400, 650), RT::subscribe(500, 750), RT::subscribe(600, 850));
}

#[test]
fn select_many_then_never_complete() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![
        RT::on_next(100, 4),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 1),
        RT::on_next(500, 5),
        RT::on_next(700, 0),
    ]);

    let ys = scheduler.create_cold_observable(vec![
        RT::on_next(50, "foo"),
        RT::on_next(100, "bar"),
        RT::on_next(150, "baz"),
        RT::on_next(200, "qux"),
        RT::on_completed(250),
    ]);

    let (source, other) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.select_many_to(other));

    assert_messages!(
        res.messages(),
        RT::on_next(350, "foo"),
        RT::on_next(400, "bar"),
        RT::on_next(450, "baz"),
        RT::on_next(450, "foo"),
        RT::on_next(500, "qux"),
        RT::on_next(500, "bar"),
        RT::on_next(550, "baz"),
        RT::on_next(550, "foo"),
        RT::on_next(600, "qux"),
        RT::on_next(600, "bar"),
        RT::on_next(650, "baz"),
        RT::on_next(650, "foo"),
        RT::on_next(700, "qux"),
        RT::on_next(700, "bar"),
        RT::on_next(750, "baz"),
        RT::on_next(750, "foo"),
        RT::on_next(800, "qux"),
        RT::on_next(800, "bar"),
        RT::on_next(850, "baz"),
        RT::on_next(900, "qux"),
        RT::on_next(950, "foo")
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(
        ys.subscriptions(),
        RT::subscribe(300, 550),
        RT::subscribe(400, 650),
        RT::subscribe(500, 750),
        RT::subscribe(600, 850),
        RT::subscribe(700, 950),
        RT::subscribe(900, 1000)
    );
}

#[test]
fn select_many_then_complete_never() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 4), RT::on_next(200, 2), RT::on_next(300, 3), RT::on_next(400, 1), RT::on_completed(500)]);

    let ys = scheduler.create_cold_observable(vec![RT::on_next(50, "foo"), RT::on_next(100, "bar"), RT::on_next(150, "baz"), RT::on_next(200, "qux")]);

    let (source, other) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.select_many_to(other));

    assert_messages!(
        res.messages(),
        RT::on_next(350, "foo"),
        RT::on_next(400, "bar"),
        RT::on_next(450, "baz"),
        RT::on_next(450, "foo"),
        RT::on_next(500, "qux"),
        RT::on_next(500, "bar"),
        RT::on_next(550, "baz"),
        RT::on_next(550, "foo"),
        RT::on_next(600, "qux"),
        RT::on_next(600, "bar"),
        RT::on_next(650, "baz"),
        RT::on_next(650, "foo"),
        RT::on_next(700, "qux"),
        RT::on_next(700, "bar"),
        RT::on_next(750, "baz"),
        RT::on_next(800, "qux")
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 700));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(300, 1000), RT::subscribe(400, 1000), RT::subscribe(500, 1000), RT::subscribe(600, 1000));
}

#[test]
fn select_many_then_complete_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 4), RT::on_next(200, 2), RT::on_next(300, 3), RT::on_next(400, 1), RT::on_completed(500)]);

    let ys = scheduler.create_cold_observable(vec![
        RT::on_next(50, "foo"),
        RT::on_next(100, "bar"),
        RT::on_next(150, "baz"),
        RT::on_next(200, "qux"),
        RT::on_error(300, ex.clone()),
    ]);

    let (source, other) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.select_many_to(other));

    assert_messages!(
        res.messages(),
        RT::on_next(350, "foo"),
        RT::on_next(400, "bar"),
        RT::on_next(450, "baz"),
        RT::on_next(450, "foo"),
        RT::on_next(500, "qux"),
        RT::on_next(500, "bar"),
        RT::on_next(550, "baz"),
        RT::on_next(550, "foo"),
        RT::on_error(600, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(300, 600), RT::subscribe(400, 600), RT::subscribe(500, 600), RT::subscribe(600, 600));
}

#[test]
fn select_many_then_error_complete() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![RT::on_next(100, 4), RT::on_next(200, 2), RT::on_next(300, 3), RT::on_next(400, 1), RT::on_error(500, ex.clone())]);

    let ys = scheduler.create_cold_observable(vec![
        RT::on_next(50, "foo"),
        RT::on_next(100, "bar"),
        RT::on_next(150, "baz"),
        RT::on_next(200, "qux"),
        RT::on_completed(250),
    ]);

    let (source, other) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.select_many_to(other));

    assert_messages!(
        res.messages(),
        RT::on_next(350, "foo"),
        RT::on_next(400, "bar"),
        RT::on_next(450, "baz"),
        RT::on_next(450, "foo"),
        RT::on_next(500, "qux"),
        RT::on_next(500, "bar"),
        RT::on_next(550, "baz"),
        RT::on_next(550, "foo"),
        RT::on_next(600, "qux"),
        RT::on_next(600, "bar"),
        RT::on_next(650, "baz"),
        RT::on_next(650, "foo"),
        RT::on_error(700, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 700));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(300, 550), RT::subscribe(400, 650), RT::subscribe(500, 700), RT::subscribe(600, 700));
}

#[test]
fn select_many_then_error_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_cold_observable(vec![
        RT::on_next(100, 4),
        RT::on_next(200, 2),
        RT::on_next(300, 3),
        RT::on_next(400, 1),
        RT::on_error(500, new_error("other")),
    ]);

    let ys = scheduler.create_cold_observable(vec![
        RT::on_next(50, "foo"),
        RT::on_next(100, "bar"),
        RT::on_next(150, "baz"),
        RT::on_next(200, "qux"),
        RT::on_error(250, ex.clone()),
    ]);

    let (source, other) = (xs.as_observable(), ys.as_observable());
    let res = scheduler.start(move || source.select_many_to(other));

    assert_messages!(
        res.messages(),
        RT::on_next(350, "foo"),
        RT::on_next(400, "bar"),
        RT::on_next(450, "baz"),
        RT::on_next(450, "foo"),
        RT::on_next(500, "qux"),
        RT::on_next(500, "bar"),
        RT::on_error(550, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));

    assert_subscriptions!(ys.subscriptions(), RT::subscribe(300, 550), RT::subscribe(400, 550), RT::subscribe(500, 550));
}

/// The inner sequences of the `Complete`, `Error` and `Dispose` scenarios: the third ends with
/// `third_end`; the fourth completes when `fourth_completes`.
fn inners(scheduler: &TestScheduler, third_end: Recorded<Notification<i32>>, fourth_completes: bool) -> Vec<Rc<ColdObservable<i32>>> {
    let mut fourth = vec![RT::on_next(180, 202), RT::on_next(190, 203)];
    if fourth_completes {
        fourth.push(RT::on_completed(205));
    }
    vec![
        scheduler.create_cold_observable(vec![RT::on_error(1, new_error("invalid operation"))]),
        scheduler.create_cold_observable(vec![RT::on_error(1, new_error("invalid operation"))]),
        scheduler.create_cold_observable(vec![
            RT::on_next(10, 102),
            RT::on_next(90, 103),
            RT::on_next(110, 104),
            RT::on_next(190, 105),
            RT::on_next(440, 106),
            third_end,
        ]),
        scheduler.create_cold_observable(fourth),
        scheduler.create_cold_observable(vec![
            RT::on_next(10, 301),
            RT::on_next(50, 302),
            RT::on_next(70, 303),
            RT::on_next(260, 304),
            RT::on_next(310, 305),
            RT::on_completed(410),
        ]),
        scheduler.create_cold_observable(vec![RT::on_completed(40)]),
        scheduler.create_cold_observable(vec![RT::on_next(80, 401), RT::on_next(90, 402), RT::on_completed(100)]),
    ]
}

/// The sequence of the inner sequences `ys`, at 5, 105, 300, 400, 550, 750 and 850, ending with
/// `end`.
fn outer(scheduler: &TestScheduler, ys: &[Rc<ColdObservable<i32>>], end: Option<Recorded<Notification<Inner>>>) -> Rc<HotObservable<Inner>> {
    let times = [5, 105, 300, 400, 550, 750, 850];
    let mut messages: Vec<Recorded<Notification<Inner>>> = times.iter().zip(ys).map(|(time, y)| RT::on_next::<Inner>(*time, y.as_observable())).collect();
    messages.extend(end);
    scheduler.create_hot_observable(messages)
}

/// `SelectMany_Complete` and `SelectManyWithIndex_Complete`.
fn select_many_complete_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let ys = inners(&scheduler, RT::on_completed(460), true);
    let xs = outer(&scheduler, &ys, Some(RT::on_completed(900)));

    let source = xs.as_observable();
    let res = scheduler.start(move || many(source, indexed, |x: Inner| x));

    assert_messages!(
        res.messages(),
        RT::on_next(310, 102),
        RT::on_next(390, 103),
        RT::on_next(410, 104),
        RT::on_next(490, 105),
        RT::on_next(560, 301),
        RT::on_next(580, 202),
        RT::on_next(590, 203),
        RT::on_next(600, 302),
        RT::on_next(620, 303),
        RT::on_next(740, 106),
        RT::on_next(810, 304),
        RT::on_next(860, 305),
        RT::on_next(930, 401),
        RT::on_next(940, 402),
        RT::on_completed(960)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 900));

    assert_subscriptions!(ys[2].subscriptions(), RT::subscribe(300, 760));

    assert_subscriptions!(ys[3].subscriptions(), RT::subscribe(400, 605));

    assert_subscriptions!(ys[4].subscriptions(), RT::subscribe(550, 960));

    assert_subscriptions!(ys[5].subscriptions(), RT::subscribe(750, 790));

    assert_subscriptions!(ys[6].subscriptions(), RT::subscribe(850, 950));
}

/// `SelectMany_Complete_InnerNotComplete` and `SelectManyWithIndex_Complete_InnerNotComplete`.
fn select_many_complete_inner_not_complete_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let ys = inners(&scheduler, RT::on_completed(460), false);
    let xs = outer(&scheduler, &ys, Some(RT::on_completed(900)));

    let source = xs.as_observable();
    let res = scheduler.start(move || many(source, indexed, |x: Inner| x));

    assert_messages!(
        res.messages(),
        RT::on_next(310, 102),
        RT::on_next(390, 103),
        RT::on_next(410, 104),
        RT::on_next(490, 105),
        RT::on_next(560, 301),
        RT::on_next(580, 202),
        RT::on_next(590, 203),
        RT::on_next(600, 302),
        RT::on_next(620, 303),
        RT::on_next(740, 106),
        RT::on_next(810, 304),
        RT::on_next(860, 305),
        RT::on_next(930, 401),
        RT::on_next(940, 402)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 900));

    assert_subscriptions!(ys[2].subscriptions(), RT::subscribe(300, 760));

    assert_subscriptions!(ys[3].subscriptions(), RT::subscribe(400, 1000));

    assert_subscriptions!(ys[4].subscriptions(), RT::subscribe(550, 960));

    assert_subscriptions!(ys[5].subscriptions(), RT::subscribe(750, 790));

    assert_subscriptions!(ys[6].subscriptions(), RT::subscribe(850, 950));
}

/// `SelectMany_Complete_OuterNotComplete` and `SelectManyWithIndex_Complete_OuterNotComplete`.
fn select_many_complete_outer_not_complete_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let ys = inners(&scheduler, RT::on_completed(460), true);
    let xs = outer(&scheduler, &ys, None);

    let source = xs.as_observable();
    let res = scheduler.start(move || many(source, indexed, |x: Inner| x));

    assert_messages!(
        res.messages(),
        RT::on_next(310, 102),
        RT::on_next(390, 103),
        RT::on_next(410, 104),
        RT::on_next(490, 105),
        RT::on_next(560, 301),
        RT::on_next(580, 202),
        RT::on_next(590, 203),
        RT::on_next(600, 302),
        RT::on_next(620, 303),
        RT::on_next(740, 106),
        RT::on_next(810, 304),
        RT::on_next(860, 305),
        RT::on_next(930, 401),
        RT::on_next(940, 402)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(ys[2].subscriptions(), RT::subscribe(300, 760));

    assert_subscriptions!(ys[3].subscriptions(), RT::subscribe(400, 605));

    assert_subscriptions!(ys[4].subscriptions(), RT::subscribe(550, 960));

    assert_subscriptions!(ys[5].subscriptions(), RT::subscribe(750, 790));

    assert_subscriptions!(ys[6].subscriptions(), RT::subscribe(850, 950));
}

/// `SelectMany_Error_Outer` and `SelectManyWithIndex_Error_Outer`.
fn select_many_error_outer_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let ys = inners(&scheduler, RT::on_completed(460), true);
    let xs = outer(&scheduler, &ys, Some(RT::on_error(900, ex.clone())));

    let source = xs.as_observable();
    let res = scheduler.start(move || many(source, indexed, |x: Inner| x));

    assert_messages!(
        res.messages(),
        RT::on_next(310, 102),
        RT::on_next(390, 103),
        RT::on_next(410, 104),
        RT::on_next(490, 105),
        RT::on_next(560, 301),
        RT::on_next(580, 202),
        RT::on_next(590, 203),
        RT::on_next(600, 302),
        RT::on_next(620, 303),
        RT::on_next(740, 106),
        RT::on_next(810, 304),
        RT::on_next(860, 305),
        RT::on_error(900, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 900));

    assert_subscriptions!(ys[2].subscriptions(), RT::subscribe(300, 760));

    assert_subscriptions!(ys[3].subscriptions(), RT::subscribe(400, 605));

    assert_subscriptions!(ys[4].subscriptions(), RT::subscribe(550, 900));

    assert_subscriptions!(ys[5].subscriptions(), RT::subscribe(750, 790));

    assert_subscriptions!(ys[6].subscriptions(), RT::subscribe(850, 900));
}

/// `SelectMany_Error_Inner` and `SelectManyWithIndex_Error_Inner`.
fn select_many_error_inner_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let ys = inners(&scheduler, RT::on_error(460, ex.clone()), true);
    let xs = outer(&scheduler, &ys, Some(RT::on_completed(900)));

    let source = xs.as_observable();
    let res = scheduler.start(move || many(source, indexed, |x: Inner| x));

    assert_messages!(
        res.messages(),
        RT::on_next(310, 102),
        RT::on_next(390, 103),
        RT::on_next(410, 104),
        RT::on_next(490, 105),
        RT::on_next(560, 301),
        RT::on_next(580, 202),
        RT::on_next(590, 203),
        RT::on_next(600, 302),
        RT::on_next(620, 303),
        RT::on_next(740, 106),
        RT::on_error(760, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 760));

    assert_subscriptions!(ys[2].subscriptions(), RT::subscribe(300, 760));

    assert_subscriptions!(ys[3].subscriptions(), RT::subscribe(400, 605));

    assert_subscriptions!(ys[4].subscriptions(), RT::subscribe(550, 760));

    assert_subscriptions!(ys[5].subscriptions(), RT::subscribe(750, 760));

    assert_subscriptions!(ys[6].subscriptions());
}

/// `SelectMany_Dispose` and `SelectManyWithIndex_Dispose`.
fn select_many_dispose_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let ys = inners(&scheduler, RT::on_completed(460), true);
    let xs = outer(&scheduler, &ys, Some(RT::on_completed(900)));

    let source = xs.as_observable();
    let res = scheduler.start_disposed(move || many(source, indexed, |x: Inner| x), 700);

    assert_messages!(
        res.messages(),
        RT::on_next(310, 102),
        RT::on_next(390, 103),
        RT::on_next(410, 104),
        RT::on_next(490, 105),
        RT::on_next(560, 301),
        RT::on_next(580, 202),
        RT::on_next(590, 203),
        RT::on_next(600, 302),
        RT::on_next(620, 303)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 700));

    assert_subscriptions!(ys[2].subscriptions(), RT::subscribe(300, 700));

    assert_subscriptions!(ys[3].subscriptions(), RT::subscribe(400, 605));

    assert_subscriptions!(ys[4].subscriptions(), RT::subscribe(550, 700));

    assert_subscriptions!(ys[5].subscriptions());

    assert_subscriptions!(ys[6].subscriptions());
}

/// `SelectMany_Throw` and `SelectManyWithIndex_Throw`.
fn select_many_throw_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let ys = inners(&scheduler, RT::on_completed(460), true);
    let xs = outer(&scheduler, &ys, Some(RT::on_completed(900)));

    let invoked = Rc::new(Cell::new(0));

    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, invoked, ex) = (xs.as_observable(), invoked.clone(), ex.clone());
        move || {
            try_many(source, indexed, move |x: Inner| {
                invoked.set(invoked.get() + 1);
                if invoked.get() == 3 {
                    return Err(ex.clone());
                }

                Ok(x)
            })
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(310, 102),
        RT::on_next(390, 103),
        RT::on_next(410, 104),
        RT::on_next(490, 105),
        RT::on_error(550, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 550));

    assert_subscriptions!(ys[2].subscriptions(), RT::subscribe(300, 550));

    assert_subscriptions!(ys[3].subscriptions(), RT::subscribe(400, 550));

    assert_subscriptions!(ys[4].subscriptions());

    assert_subscriptions!(ys[5].subscriptions());

    assert_subscriptions!(ys[6].subscriptions());

    assert_eq!(3, invoked.get());
}

/// `SelectMany_UseFunction` and `SelectManyWithIndex_UseFunction`.
fn select_many_use_function_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 4), RT::on_next(220, 3), RT::on_next(250, 5), RT::on_next(270, 1), RT::on_completed(290)]);

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || {
        many(source, indexed, move |x: i32| Observable::interval_on(ticks_to_duration(10), s.clone()).select(move |_| x).take(x as usize))
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220, 4),
        RT::on_next(230, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 3),
        RT::on_next(240, 4),
        RT::on_next(250, 3),
        RT::on_next(250, 4),
        RT::on_next(260, 5),
        RT::on_next(270, 5),
        RT::on_next(280, 1),
        RT::on_next(280, 5),
        RT::on_next(290, 5),
        RT::on_next(300, 5),
        RT::on_completed(300)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 290));
}

#[test]
fn select_many_complete() {
    select_many_complete_case(false);
}

#[test]
fn select_many_complete_inner_not_complete() {
    select_many_complete_inner_not_complete_case(false);
}

#[test]
fn select_many_complete_outer_not_complete() {
    select_many_complete_outer_not_complete_case(false);
}

#[test]
fn select_many_error_outer() {
    select_many_error_outer_case(false);
}

#[test]
fn select_many_error_inner() {
    select_many_error_inner_case(false);
}

#[test]
fn select_many_dispose() {
    select_many_dispose_case(false);
}

#[test]
fn select_many_throw() {
    select_many_throw_case(false);
}

#[test]
fn select_many_use_function() {
    select_many_use_function_case(false);
}

#[test]
fn select_many_with_index_index() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 4), RT::on_next(220, 3), RT::on_next(250, 5), RT::on_next(270, 1), RT::on_completed(290)]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.select_many_with_index(|x, i| Observable::return_((x, i))));

    assert_messages!(
        res.messages(),
        RT::on_next(210, (4, 0usize)),
        RT::on_next(220, (3, 1usize)),
        RT::on_next(250, (5, 2usize)),
        RT::on_next(270, (1, 3usize)),
        RT::on_completed(290)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 290));
}

#[test]
fn select_many_with_index_complete() {
    select_many_complete_case(true);
}

#[test]
fn select_many_with_index_complete_inner_not_complete() {
    select_many_complete_inner_not_complete_case(true);
}

#[test]
fn select_many_with_index_complete_outer_not_complete() {
    select_many_complete_outer_not_complete_case(true);
}

#[test]
fn select_many_with_index_error_outer() {
    select_many_error_outer_case(true);
}

#[test]
fn select_many_with_index_error_inner() {
    select_many_error_inner_case(true);
}

#[test]
fn select_many_with_index_dispose() {
    select_many_dispose_case(true);
}

#[test]
fn select_many_with_index_throw() {
    select_many_throw_case(true);
}

#[test]
fn select_many_with_index_use_function() {
    select_many_use_function_case(true);
}

/// The source of the query operator scenarios, ending with `end`.
fn query_source(scheduler: &TestScheduler, end: Recorded<Notification<i32>>) -> Rc<HotObservable<i32>> {
    scheduler.create_hot_observable(vec![RT::on_next(220, 4), RT::on_next(221, 3), RT::on_next(222, 2), RT::on_next(223, 5), end])
}

/// `Observable.Interval(TimeSpan.FromTicks(1), scheduler).Take(x)`.
fn interval_take(scheduler: &Rc<dyn IScheduler>, x: i32) -> Rc<dyn IObservable<i64>> {
    Observable::interval_on(ticks_to_duration(1), scheduler.clone()).take(x as usize)
}

/// `x * 10 + (int)y`.
fn tens(x: i32, y: i64) -> i32 {
    x * 10 + y as i32
}

/// `SelectMany_QueryOperator_CompleteOuterFirst` and `SelectManyWithIndex_QueryOperator_CompleteOuterFirst`.
fn select_many_query_operator_complete_outer_first_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = query_source(&scheduler, RT::on_completed(224));

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || query(source, indexed, move |x| interval_take(&s, x), tens));

    assert_messages!(
        res.messages(),
        RT::on_next(221, 40),
        RT::on_next(222, 30),
        RT::on_next(222, 41),
        RT::on_next(223, 20),
        RT::on_next(223, 31),
        RT::on_next(223, 42),
        RT::on_next(224, 50),
        RT::on_next(224, 21),
        RT::on_next(224, 32),
        RT::on_next(224, 43),
        RT::on_next(225, 51),
        RT::on_next(226, 52),
        RT::on_next(227, 53),
        RT::on_next(228, 54),
        RT::on_completed(228)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 224));
}

/// `SelectMany_QueryOperator_CompleteInnerFirst` and `SelectManyWithIndex_QueryOperator_CompleteInnerFirst`.
fn select_many_query_operator_complete_inner_first_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = query_source(&scheduler, RT::on_completed(300));

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || query(source, indexed, move |x| interval_take(&s, x), tens));

    assert_messages!(
        res.messages(),
        RT::on_next(221, 40),
        RT::on_next(222, 30),
        RT::on_next(222, 41),
        RT::on_next(223, 20),
        RT::on_next(223, 31),
        RT::on_next(223, 42),
        RT::on_next(224, 50),
        RT::on_next(224, 21),
        RT::on_next(224, 32),
        RT::on_next(224, 43),
        RT::on_next(225, 51),
        RT::on_next(226, 52),
        RT::on_next(227, 53),
        RT::on_next(228, 54),
        RT::on_completed(300)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

/// `SelectMany_QueryOperator_ErrorOuter` and `SelectManyWithIndex_QueryOperator_ErrorOuter`.
fn select_many_query_operator_error_outer_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = query_source(&scheduler, RT::on_error(224, ex.clone()));

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start(move || query(source, indexed, move |x| interval_take(&s, x), tens));

    assert_messages!(
        res.messages(),
        RT::on_next(221, 40),
        RT::on_next(222, 30),
        RT::on_next(222, 41),
        RT::on_next(223, 20),
        RT::on_next(223, 31),
        RT::on_next(223, 42),
        RT::on_error(224, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 224));
}

/// `SelectMany_QueryOperator_ErrorInner` and `SelectManyWithIndex_QueryOperator_ErrorInner`.
fn select_many_query_operator_error_inner_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = query_source(&scheduler, RT::on_completed(224));

    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, s, ex) = (xs.as_observable(), scheduler.scheduler(), ex.clone());
        move || query(source, indexed, move |x| if x == 2 { Observable::throw_on::<i64>(ex.clone(), s.clone()) } else { interval_take(&s, x) }, tens)
    });

    assert_messages!(res.messages(), RT::on_next(221, 40), RT::on_next(222, 30), RT::on_next(222, 41), RT::on_error(223, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 223));
}

/// `SelectMany_QueryOperator_Dispose` and `SelectManyWithIndex_QueryOperator_Dispose`.
fn select_many_query_operator_dispose_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = query_source(&scheduler, RT::on_completed(224));

    let (source, s) = (xs.as_observable(), scheduler.scheduler());
    let res = scheduler.start_disposed(move || query(source, indexed, move |x| interval_take(&s, x), tens), 223);

    assert_messages!(res.messages(), RT::on_next(221, 40), RT::on_next(222, 30), RT::on_next(222, 41));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 223));
}

/// `SelectMany_QueryOperator_ThrowSelector` and `SelectManyWithIndex_QueryOperator_ThrowSelector`.
fn select_many_query_operator_throw_selector_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = query_source(&scheduler, RT::on_completed(224));

    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, ex) = (xs.as_observable(), ex.clone());
        move || try_query(source, indexed, move |_| Err::<Rc<dyn IObservable<i64>>, _>(ex.clone()), |x, y| Ok(tens(x, y)))
    });

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220));
}

/// `SelectMany_QueryOperator_ThrowResult` and `SelectManyWithIndex_QueryOperator_ThrowResult`.
fn select_many_query_operator_throw_result_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = query_source(&scheduler, RT::on_completed(224));

    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, s, ex) = (xs.as_observable(), scheduler.scheduler(), ex.clone());
        move || try_query(source, indexed, move |x| Ok(interval_take(&s, x)), move |_, _: i64| Err(ex.clone()))
    });

    assert_messages!(res.messages(), RT::on_error(221, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 221));
}

#[test]
fn select_many_query_operator_complete_outer_first() {
    select_many_query_operator_complete_outer_first_case(false);
}

#[test]
fn select_many_query_operator_complete_inner_first() {
    select_many_query_operator_complete_inner_first_case(false);
}

#[test]
fn select_many_query_operator_error_outer() {
    select_many_query_operator_error_outer_case(false);
}

#[test]
fn select_many_query_operator_error_inner() {
    select_many_query_operator_error_inner_case(false);
}

#[test]
fn select_many_query_operator_dispose() {
    select_many_query_operator_dispose_case(false);
}

#[test]
fn select_many_query_operator_throw_selector() {
    select_many_query_operator_throw_selector_case(false);
}

#[test]
fn select_many_query_operator_throw_result() {
    select_many_query_operator_throw_result_case(false);
}

#[test]
fn select_many_with_index_query_operator_index() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 4), RT::on_next(220, 3), RT::on_next(250, 5), RT::on_next(270, 1), RT::on_completed(290)]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.select_many_with_index_with(|_, i| Observable::range(10, i + 1), |x, i, y, j| (x, i, y, j)));

    assert_messages!(
        res.messages(),
        RT::on_next(210, (4, 0usize, 10, 0usize)),
        RT::on_next(220, (3, 1usize, 10, 0usize)),
        RT::on_next(220, (3, 1usize, 11, 1usize)),
        RT::on_next(250, (5, 2usize, 10, 0usize)),
        RT::on_next(250, (5, 2usize, 11, 1usize)),
        RT::on_next(250, (5, 2usize, 12, 2usize)),
        RT::on_next(270, (1, 3usize, 10, 0usize)),
        RT::on_next(270, (1, 3usize, 11, 1usize)),
        RT::on_next(270, (1, 3usize, 12, 2usize)),
        RT::on_next(270, (1, 3usize, 13, 3usize)),
        RT::on_completed(290)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 290));
}

#[test]
fn select_many_with_index_query_operator_complete_outer_first() {
    select_many_query_operator_complete_outer_first_case(true);
}

#[test]
fn select_many_with_index_query_operator_complete_inner_first() {
    select_many_query_operator_complete_inner_first_case(true);
}

#[test]
fn select_many_with_index_query_operator_error_outer() {
    select_many_query_operator_error_outer_case(true);
}

#[test]
fn select_many_with_index_query_operator_error_inner() {
    select_many_query_operator_error_inner_case(true);
}

#[test]
fn select_many_with_index_query_operator_dispose() {
    select_many_query_operator_dispose_case(true);
}

#[test]
fn select_many_with_index_query_operator_throw_selector() {
    select_many_query_operator_throw_selector_case(true);
}

#[test]
fn select_many_with_index_query_operator_throw_result() {
    select_many_query_operator_throw_result_case(true);
}

/// The source of the scenarios with three selectors, ending with `end`.
fn triple_source(scheduler: &TestScheduler, end: Recorded<Notification<i32>>) -> Rc<HotObservable<i32>> {
    scheduler.create_hot_observable(vec![RT::on_next(300, 0), RT::on_next(301, 1), RT::on_next(302, 2), RT::on_next(303, 3), RT::on_next(304, 4), end])
}

/// `SelectMany_Triple_Identity` and `SelectManyWithIndex_Triple_Identity`.
fn select_many_triple_identity_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_completed(305));

    let res = scheduler.start({
        let (source, s1, s2, s3) = (xs.as_observable(), scheduler.scheduler(), scheduler.scheduler(), scheduler.scheduler());
        move || {
            triple(
                source,
                indexed,
                move |x| Observable::return_on(x, s1.clone()),
                move |ex| Observable::throw_on(ex, s2.clone()),
                move || Observable::empty_on(s3.clone()),
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(301, 0),
        RT::on_next(302, 1),
        RT::on_next(303, 2),
        RT::on_next(304, 3),
        RT::on_next(305, 4),
        RT::on_completed(306)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));
}

/// The scenarios `InnersWithTiming1`, `InnersWithTiming2` and `InnersWithTiming3` (with and without
/// an index): the source `xs`, the projection of each value `ysn`, of the error `yse` and of the
/// completion `ysc`.
fn triple_inners(scheduler: &TestScheduler, xs: &Rc<HotObservable<i32>>, ysc_end: i64, indexed: bool) -> (Rc<ColdObservable<i32>>, Rc<ColdObservable<i32>>, Rc<ColdObservable<i32>>, Vec<Recorded<Notification<i32>>>) {
    let ysn = scheduler.create_cold_observable(vec![RT::on_next(10, 10), RT::on_next(20, 11), RT::on_next(30, 12), RT::on_completed(40)]);

    let yse = scheduler.create_cold_observable(vec![RT::on_next(0, 99), RT::on_completed(10)]);

    let ysc = scheduler.create_cold_observable(vec![RT::on_next(10, 42), RT::on_completed(ysc_end)]);

    let res = scheduler.start({
        let (source, n, e, c) = (xs.as_observable(), ysn.as_observable(), yse.as_observable(), ysc.as_observable());
        move || triple(source, indexed, move |_| n.clone(), move |_| e.clone(), move || c.clone())
    });

    (ysn, yse, ysc, res.messages())
}

/// `SelectMany_Triple_InnersWithTiming1` and `SelectManyWithIndex_Triple_InnersWithTiming1`.
fn select_many_triple_inners_with_timing1_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_completed(305));

    let (ysn, yse, ysc, messages) = triple_inners(&scheduler, &xs, 20, indexed);

    assert_messages!(
        messages,
        RT::on_next(310, 10),
        RT::on_next(311, 10),
        RT::on_next(312, 10),
        RT::on_next(313, 10),
        RT::on_next(314, 10),
        RT::on_next(315, 42),
        RT::on_next(320, 11),
        RT::on_next(321, 11),
        RT::on_next(322, 11),
        RT::on_next(323, 11),
        RT::on_next(324, 11),
        RT::on_next(330, 12),
        RT::on_next(331, 12),
        RT::on_next(332, 12),
        RT::on_next(333, 12),
        RT::on_next(334, 12),
        RT::on_completed(344)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));

    assert_subscriptions!(
        ysn.subscriptions(),
        RT::subscribe(300, 340),
        RT::subscribe(301, 341),
        RT::subscribe(302, 342),
        RT::subscribe(303, 343),
        RT::subscribe(304, 344)
    );

    assert_subscriptions!(yse.subscriptions());

    assert_subscriptions!(ysc.subscriptions(), RT::subscribe(305, 325));
}

/// `SelectMany_Triple_InnersWithTiming2` and `SelectManyWithIndex_Triple_InnersWithTiming2`.
fn select_many_triple_inners_with_timing2_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_completed(305));

    let (ysn, yse, ysc, messages) = triple_inners(&scheduler, &xs, 50, indexed);

    assert_messages!(
        messages,
        RT::on_next(310, 10),
        RT::on_next(311, 10),
        RT::on_next(312, 10),
        RT::on_next(313, 10),
        RT::on_next(314, 10),
        RT::on_next(315, 42),
        RT::on_next(320, 11),
        RT::on_next(321, 11),
        RT::on_next(322, 11),
        RT::on_next(323, 11),
        RT::on_next(324, 11),
        RT::on_next(330, 12),
        RT::on_next(331, 12),
        RT::on_next(332, 12),
        RT::on_next(333, 12),
        RT::on_next(334, 12),
        RT::on_completed(355)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));

    assert_subscriptions!(
        ysn.subscriptions(),
        RT::subscribe(300, 340),
        RT::subscribe(301, 341),
        RT::subscribe(302, 342),
        RT::subscribe(303, 343),
        RT::subscribe(304, 344)
    );

    assert_subscriptions!(yse.subscriptions());

    assert_subscriptions!(ysc.subscriptions(), RT::subscribe(305, 355));
}

/// `SelectMany_Triple_InnersWithTiming3` and `SelectManyWithIndex_Triple_InnersWithTiming3`.
fn select_many_triple_inners_with_timing3_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(300, 0),
        RT::on_next(400, 1),
        RT::on_next(500, 2),
        RT::on_next(600, 3),
        RT::on_next(700, 4),
        RT::on_completed(800),
    ]);

    let (ysn, yse, ysc, messages) = triple_inners(&scheduler, &xs, 100, indexed);

    assert_messages!(
        messages,
        RT::on_next(310, 10),
        RT::on_next(320, 11),
        RT::on_next(330, 12),
        RT::on_next(410, 10),
        RT::on_next(420, 11),
        RT::on_next(430, 12),
        RT::on_next(510, 10),
        RT::on_next(520, 11),
        RT::on_next(530, 12),
        RT::on_next(610, 10),
        RT::on_next(620, 11),
        RT::on_next(630, 12),
        RT::on_next(710, 10),
        RT::on_next(720, 11),
        RT::on_next(730, 12),
        RT::on_next(810, 42),
        RT::on_completed(900)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 800));

    assert_subscriptions!(
        ysn.subscriptions(),
        RT::subscribe(300, 340),
        RT::subscribe(400, 440),
        RT::subscribe(500, 540),
        RT::subscribe(600, 640),
        RT::subscribe(700, 740)
    );

    assert_subscriptions!(yse.subscriptions());

    assert_subscriptions!(ysc.subscriptions(), RT::subscribe(800, 900));
}

/// `SelectMany_Triple_Error_Identity` and `SelectManyWithIndex_Triple_Error_Identity`.
fn select_many_triple_error_identity_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = triple_source(&scheduler, RT::on_error(305, ex.clone()));

    let res = scheduler.start({
        let (source, s1, s2, s3) = (xs.as_observable(), scheduler.scheduler(), scheduler.scheduler(), scheduler.scheduler());
        move || {
            triple(
                source,
                indexed,
                move |x| Observable::return_on(x, s1.clone()),
                move |ex1| Observable::throw_on(ex1, s2.clone()),
                move || Observable::empty_on(s3.clone()),
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(301, 0),
        RT::on_next(302, 1),
        RT::on_next(303, 2),
        RT::on_next(304, 3),
        RT::on_next(305, 4),
        RT::on_error(306, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));
}

/// `SelectMany_Triple_SelectMany` and `SelectManyWithIndex_Triple_SelectMany`.
fn select_many_triple_select_many_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_completed(305));

    let res = scheduler.start({
        let (source, s1, s2, s3) = (xs.as_observable(), scheduler.scheduler(), scheduler.scheduler(), scheduler.scheduler());
        move || {
            triple(
                source,
                indexed,
                move |x| Observable::repeat_count_on(x, x as usize, s1.clone()),
                move |ex| Observable::throw_on(ex, s2.clone()),
                move || Observable::empty_on(s3.clone()),
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(302, 1),
        RT::on_next(303, 2),
        RT::on_next(304, 3),
        RT::on_next(304, 2),
        RT::on_next(305, 4),
        RT::on_next(305, 3),
        RT::on_next(306, 4),
        RT::on_next(306, 3),
        RT::on_next(307, 4),
        RT::on_next(308, 4),
        RT::on_completed(308)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));
}

/// `SelectMany_Triple_Concat` and `SelectManyWithIndex_Triple_Concat`.
fn select_many_triple_concat_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_completed(305));

    let res = scheduler.start({
        let (source, s1, s2, s3) = (xs.as_observable(), scheduler.scheduler(), scheduler.scheduler(), scheduler.scheduler());
        move || {
            triple(
                source,
                indexed,
                move |x| Observable::return_on(x, s1.clone()),
                move |ex| Observable::throw_on(ex, s2.clone()),
                move || Observable::range_on(1, 3, s3.clone()),
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(301, 0),
        RT::on_next(302, 1),
        RT::on_next(303, 2),
        RT::on_next(304, 3),
        RT::on_next(305, 4),
        RT::on_next(306, 1),
        RT::on_next(307, 2),
        RT::on_next(308, 3),
        RT::on_completed(309)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));
}

/// `SelectMany_Triple_Catch` and `SelectManyWithIndex_Triple_Catch`.
fn select_many_triple_catch_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_completed(305));

    let res = scheduler.start({
        let (source, s1, s2, s3) = (xs.as_observable(), scheduler.scheduler(), scheduler.scheduler(), scheduler.scheduler());
        move || {
            triple(
                source,
                indexed,
                move |x| Observable::return_on(x, s1.clone()),
                move |_| Observable::range_on(1, 3, s2.clone()),
                move || Observable::empty_on(s3.clone()),
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(301, 0),
        RT::on_next(302, 1),
        RT::on_next(303, 2),
        RT::on_next(304, 3),
        RT::on_next(305, 4),
        RT::on_completed(306)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));
}

/// `SelectMany_Triple_Error_Catch` and `SelectManyWithIndex_Triple_Error_Catch`.
fn select_many_triple_error_catch_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_error(305, new_error("ex")));

    let res = scheduler.start({
        let (source, s1, s2, s3) = (xs.as_observable(), scheduler.scheduler(), scheduler.scheduler(), scheduler.scheduler());
        move || {
            triple(
                source,
                indexed,
                move |x| Observable::return_on(x, s1.clone()),
                move |_| Observable::range_on(1, 3, s2.clone()),
                move || Observable::empty_on(s3.clone()),
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(301, 0),
        RT::on_next(302, 1),
        RT::on_next(303, 2),
        RT::on_next(304, 3),
        RT::on_next(305, 4),
        RT::on_next(306, 1),
        RT::on_next(307, 2),
        RT::on_next(308, 3),
        RT::on_completed(309)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));
}

/// The projection of the `All` scenarios: `x => Observable.Repeat(x, x, scheduler)`,
/// `ex => Observable.Repeat(0, 2, scheduler)`, `() => Observable.Repeat(-1, 2, scheduler)`.
fn triple_all(scheduler: &TestScheduler, xs: &Rc<HotObservable<i32>>, indexed: bool) -> impl FnOnce() -> Inner + 'static {
    let (source, s1, s2, s3) = (xs.as_observable(), scheduler.scheduler(), scheduler.scheduler(), scheduler.scheduler());
    move || {
        triple(
            source,
            indexed,
            move |x| Observable::repeat_count_on(x, x as usize, s1.clone()),
            move |_| Observable::repeat_count_on(0, 2, s2.clone()),
            move || Observable::repeat_count_on(-1, 2, s3.clone()),
        )
    }
}

/// `SelectMany_Triple_All` and `SelectManyWithIndex_Triple_All`.
fn select_many_triple_all_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_completed(305));

    let res = scheduler.start(triple_all(&scheduler, &xs, indexed));

    assert_messages!(
        res.messages(),
        RT::on_next(302, 1),
        RT::on_next(303, 2),
        RT::on_next(304, 3),
        RT::on_next(304, 2),
        RT::on_next(305, 4),
        RT::on_next(305, 3),
        RT::on_next(306, -1),
        RT::on_next(306, 4),
        RT::on_next(306, 3),
        RT::on_next(307, -1),
        RT::on_next(307, 4),
        RT::on_next(308, 4),
        RT::on_completed(308)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));
}

/// `SelectMany_Triple_Error_All` and `SelectManyWithIndex_Triple_Error_All`.
fn select_many_triple_error_all_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_error(305, new_error("ex")));

    let res = scheduler.start(triple_all(&scheduler, &xs, indexed));

    assert_messages!(
        res.messages(),
        RT::on_next(302, 1),
        RT::on_next(303, 2),
        RT::on_next(304, 3),
        RT::on_next(304, 2),
        RT::on_next(305, 4),
        RT::on_next(305, 3),
        RT::on_next(306, 0),
        RT::on_next(306, 4),
        RT::on_next(306, 3),
        RT::on_next(307, 0),
        RT::on_next(307, 4),
        RT::on_next(308, 4),
        RT::on_completed(308)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));
}

/// `SelectMany_Triple_All_Dispose` and `SelectManyWithIndex_Triple_All_Dispose`.
fn select_many_triple_all_dispose_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_completed(305));

    let res = scheduler.start_disposed(triple_all(&scheduler, &xs, indexed), 307);

    assert_messages!(
        res.messages(),
        RT::on_next(302, 1),
        RT::on_next(303, 2),
        RT::on_next(304, 3),
        RT::on_next(304, 2),
        RT::on_next(305, 4),
        RT::on_next(305, 3),
        RT::on_next(306, -1),
        RT::on_next(306, 4),
        RT::on_next(306, 3)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));
}

/// `SelectMany_Triple_All_Dispose_Before_First` and `SelectManyWithIndex_Triple_All_Dispose_Before_First`.
fn select_many_triple_all_dispose_before_first_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_completed(305));

    let res = scheduler.start_disposed(triple_all(&scheduler, &xs, indexed), 304);

    assert_messages!(res.messages(), RT::on_next(302, 1), RT::on_next(303, 2));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 304));
}

/// `SelectMany_Triple_OnNextThrow` and `SelectManyWithIndex_Triple_OnNextThrow`.
fn select_many_triple_on_next_throw_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_completed(305));

    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, s2, s3, ex) = (xs.as_observable(), scheduler.scheduler(), scheduler.scheduler(), ex.clone());
        move || {
            try_triple(
                source,
                indexed,
                move |_| Err(ex.clone()),
                move |_| Ok(Observable::repeat_count_on(0, 2, s2.clone())),
                move || Ok(Observable::repeat_count_on(-1, 2, s3.clone())),
            )
        }
    });

    assert_messages!(res.messages(), RT::on_error(300, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

/// `SelectMany_Triple_OnErrorThrow` and `SelectManyWithIndex_Triple_OnErrorThrow`.
fn select_many_triple_on_error_throw_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = triple_source(&scheduler, RT::on_error(305, new_error("other")));

    let res = scheduler.start({
        let (source, s1, s3, ex) = (xs.as_observable(), scheduler.scheduler(), scheduler.scheduler(), ex.clone());
        move || {
            try_triple(
                source,
                indexed,
                move |x| Ok(Observable::repeat_count_on(x, x as usize, s1.clone())),
                move |_| Err(ex.clone()),
                move || Ok(Observable::repeat_count_on(-1, 2, s3.clone())),
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(302, 1),
        RT::on_next(303, 2),
        RT::on_next(304, 3),
        RT::on_next(304, 2),
        RT::on_error(305, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));
}

/// `SelectMany_Triple_OnCompletedThrow` and `SelectManyWithIndex_Triple_OnCompletedThrow`.
fn select_many_triple_on_completed_throw_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_completed(305));

    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, s1, s2, ex) = (xs.as_observable(), scheduler.scheduler(), scheduler.scheduler(), ex.clone());
        move || {
            try_triple(
                source,
                indexed,
                move |x| Ok(Observable::repeat_count_on(x, x as usize, s1.clone())),
                move |_| Ok(Observable::repeat_count_on(0, 2, s2.clone())),
                move || Err(ex.clone()),
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(302, 1),
        RT::on_next(303, 2),
        RT::on_next(304, 3),
        RT::on_next(304, 2),
        RT::on_error(305, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));
}

#[test]
fn select_many_triple_identity() {
    select_many_triple_identity_case(false);
}

#[test]
fn select_many_triple_inners_with_timing1() {
    select_many_triple_inners_with_timing1_case(false);
}

#[test]
fn select_many_triple_inners_with_timing2() {
    select_many_triple_inners_with_timing2_case(false);
}

#[test]
fn select_many_triple_inners_with_timing3() {
    select_many_triple_inners_with_timing3_case(false);
}

#[test]
fn select_many_triple_error_identity() {
    select_many_triple_error_identity_case(false);
}

#[test]
fn select_many_triple_select_many() {
    select_many_triple_select_many_case(false);
}

#[test]
fn select_many_triple_concat() {
    select_many_triple_concat_case(false);
}

#[test]
fn select_many_triple_catch() {
    select_many_triple_catch_case(false);
}

#[test]
fn select_many_triple_error_catch() {
    select_many_triple_error_catch_case(false);
}

#[test]
fn select_many_triple_all() {
    select_many_triple_all_case(false);
}

#[test]
fn select_many_triple_error_all() {
    select_many_triple_error_all_case(false);
}

#[test]
fn select_many_triple_all_dispose() {
    select_many_triple_all_dispose_case(false);
}

#[test]
fn select_many_triple_all_dispose_before_first() {
    select_many_triple_all_dispose_before_first_case(false);
}

#[test]
fn select_many_triple_on_next_throw() {
    select_many_triple_on_next_throw_case(false);
}

#[test]
fn select_many_triple_on_error_throw() {
    select_many_triple_on_error_throw_case(false);
}

#[test]
fn select_many_triple_on_completed_throw() {
    select_many_triple_on_completed_throw_case(false);
}

#[test]
fn select_many_with_index_triple_index() {
    let scheduler = TestScheduler::new();

    let xs = triple_source(&scheduler, RT::on_completed(305));

    let res = scheduler.start({
        let (source, s1, s2, s3) = (xs.as_observable(), scheduler.scheduler(), scheduler.scheduler(), scheduler.scheduler());
        move || {
            source.select_many_notifications_with_index(
                move |x, i| Observable::return_on((x, i), s1.clone()),
                move |ex| Observable::throw_on(ex, s2.clone()),
                move || Observable::empty_on(s3.clone()),
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(301, (0, 0usize)),
        RT::on_next(302, (1, 1usize)),
        RT::on_next(303, (2, 2usize)),
        RT::on_next(304, (3, 3usize)),
        RT::on_next(305, (4, 4usize)),
        RT::on_completed(306)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 305));
}

#[test]
fn select_many_with_index_triple_identity() {
    select_many_triple_identity_case(true);
}

#[test]
fn select_many_with_index_triple_inners_with_timing1() {
    select_many_triple_inners_with_timing1_case(true);
}

#[test]
fn select_many_with_index_triple_inners_with_timing2() {
    select_many_triple_inners_with_timing2_case(true);
}

#[test]
fn select_many_with_index_triple_inners_with_timing3() {
    select_many_triple_inners_with_timing3_case(true);
}

#[test]
fn select_many_with_index_triple_error_identity() {
    select_many_triple_error_identity_case(true);
}

#[test]
fn select_many_with_index_triple_select_many() {
    select_many_triple_select_many_case(true);
}

#[test]
fn select_many_with_index_triple_concat() {
    select_many_triple_concat_case(true);
}

#[test]
fn select_many_with_index_triple_catch() {
    select_many_triple_catch_case(true);
}

#[test]
fn select_many_with_index_triple_error_catch() {
    select_many_triple_error_catch_case(true);
}

#[test]
fn select_many_with_index_triple_all() {
    select_many_triple_all_case(true);
}

#[test]
fn select_many_with_index_triple_error_all() {
    select_many_triple_error_all_case(true);
}

#[test]
fn select_many_with_index_triple_all_dispose() {
    select_many_triple_all_dispose_case(true);
}

#[test]
fn select_many_with_index_triple_all_dispose_before_first() {
    select_many_triple_all_dispose_before_first_case(true);
}

#[test]
fn select_many_with_index_triple_on_next_throw() {
    select_many_triple_on_next_throw_case(true);
}

#[test]
fn select_many_with_index_triple_on_error_throw() {
    select_many_triple_on_error_throw_case(true);
}

#[test]
fn select_many_with_index_triple_on_completed_throw() {
    select_many_triple_on_completed_throw_case(true);
}
// The forms on tasks.

/// `SelectMany(selector)` to tasks, or `SelectMany((x, _) => selector(x))` when `indexed`.
fn many_async<U: Clone + 'static, Fut: Future<Output = Result<U, ObservableError>> + 'static>(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    selector: impl Fn(i32) -> Fut + 'static,
) -> Rc<dyn IObservable<U>> {
    if indexed {
        xs.select_many_async_with_index(move |x, _| selector(x))
    } else {
        xs.select_many_async(selector)
    }
}

/// [`many_async`] with a selector that may fail.
fn try_many_async<U: Clone + 'static, Fut: Future<Output = Result<U, ObservableError>> + 'static>(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    selector: impl Fn(i32) -> Result<Fut, ObservableError> + 'static,
) -> Rc<dyn IObservable<U>> {
    if indexed {
        xs.try_select_many_async_with_index(move |x, _| selector(x))
    } else {
        xs.try_select_many_async(selector)
    }
}

/// `SelectMany((x, token) => selector(x, token))` to tasks, or
/// `SelectMany((x, _, token) => selector(x, token))` when `indexed`.
fn many_async_token<U: Clone + 'static, Fut: Future<Output = Result<U, ObservableError>> + 'static>(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    selector: impl Fn(i32, CancellationToken) -> Fut + 'static,
) -> Rc<dyn IObservable<U>> {
    if indexed {
        xs.select_many_async_with_index_with_token(move |x, _, token| selector(x, token))
    } else {
        xs.select_many_async_with_token(selector)
    }
}

/// [`many_async_token`] with a selector that may fail.
fn try_many_async_token<U: Clone + 'static, Fut: Future<Output = Result<U, ObservableError>> + 'static>(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    selector: impl Fn(i32, CancellationToken) -> Result<Fut, ObservableError> + 'static,
) -> Rc<dyn IObservable<U>> {
    if indexed {
        xs.try_select_many_async_with_index_with_token(move |x, _, token| selector(x, token))
    } else {
        xs.try_select_many_async_with_token(selector)
    }
}

/// `SelectMany(taskSelector, resultSelector)`, or
/// `SelectMany((x, _) => taskSelector(x), (x, _, y) => resultSelector(x, y))` when `indexed`.
fn many_async_with<C: Clone + 'static, Fut: Future<Output = Result<C, ObservableError>> + 'static>(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    task_selector: impl Fn(i32) -> Fut + 'static,
    result_selector: impl Fn(i32, C) -> i32 + 'static,
) -> Rc<dyn IObservable<i32>> {
    if indexed {
        xs.select_many_async_with_index_with(move |x, _| task_selector(x), move |x, _, y| result_selector(x, y))
    } else {
        xs.select_many_async_with(task_selector, result_selector)
    }
}

/// [`many_async_with`] with selectors that may fail.
fn try_many_async_with<C: Clone + 'static, Fut: Future<Output = Result<C, ObservableError>> + 'static>(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    task_selector: impl Fn(i32) -> Result<Fut, ObservableError> + 'static,
    result_selector: impl Fn(i32, C) -> Result<i32, ObservableError> + 'static,
) -> Rc<dyn IObservable<i32>> {
    if indexed {
        xs.try_select_many_async_with_index_with(move |x, _| task_selector(x), move |x, _, y| result_selector(x, y))
    } else {
        xs.try_select_many_async_with(task_selector, result_selector)
    }
}

/// `SelectMany((x, token) => taskSelector(x, token), resultSelector)`, or
/// `SelectMany((x, _, token) => taskSelector(x, token), (x, _, y) => resultSelector(x, y))` when
/// `indexed`.
fn many_async_token_with<C: Clone + 'static, Fut: Future<Output = Result<C, ObservableError>> + 'static>(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    task_selector: impl Fn(i32, CancellationToken) -> Fut + 'static,
    result_selector: impl Fn(i32, C) -> i32 + 'static,
) -> Rc<dyn IObservable<i32>> {
    if indexed {
        xs.select_many_async_with_index_with_token_with(move |x, _, token| task_selector(x, token), move |x, _, y| result_selector(x, y))
    } else {
        xs.select_many_async_with_token_with(task_selector, result_selector)
    }
}

/// [`many_async_token_with`] with selectors that may fail.
fn try_many_async_token_with<C: Clone + 'static, Fut: Future<Output = Result<C, ObservableError>> + 'static>(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    task_selector: impl Fn(i32, CancellationToken) -> Result<Fut, ObservableError> + 'static,
    result_selector: impl Fn(i32, C) -> Result<i32, ObservableError> + 'static,
) -> Rc<dyn IObservable<i32>> {
    if indexed {
        xs.try_select_many_async_with_index_with_token_with(move |x, _, token| task_selector(x, token), move |x, _, y| result_selector(x, y))
    } else {
        xs.try_select_many_async_with_token_with(task_selector, result_selector)
    }
}

/// `n` completion sources (`new TaskCompletionSource<int>[n]`).
fn completion_sources(n: usize) -> Vec<TaskCompletionSource<i32>> {
    (0..n).map(|_| TaskCompletionSource::new()).collect()
}

/// A selector of the task of a completion source (`x => tcss[x].Task`).
fn task_of(tcss: &[TaskCompletionSource<i32>]) -> impl Fn(i32) -> Task<i32> + 'static {
    let tcss = tcss.to_vec();
    move |x| tcss[x as usize].task()
}

/// What a subscription received: the values (`lst`), the error (`err`) and whether it ended
/// (`done`, set by an error too where upstream's test sets it on both).
#[derive(Default)]
struct Received {
    values: RefCell<Vec<i32>>,
    error: RefCell<Option<ObservableError>>,
    completed: Cell<bool>,
}

impl Received {
    fn sorted(&self) -> Vec<i32> {
        let mut values = self.values.borrow().clone();
        values.sort();
        values
    }

    fn error(&self) -> Option<ObservableError> {
        self.error.borrow().clone()
    }
}

/// `res.Subscribe(lst.Add, ex_ => err = ex_, () => done = true)`.
fn receive(res: &Rc<dyn IObservable<i32>>) -> (Rc<Received>, Rc<dyn IDisposable>) {
    let received = Rc::new(Received::default());
    let (on_next, on_error, on_completed) = (received.clone(), received.clone(), received.clone());
    let subscription = res.subscribe_all(
        move |x| on_next.values.borrow_mut().push(x),
        move |ex| *on_error.error.borrow_mut() = Some(ex),
        move || on_completed.completed.set(true),
    );
    (received, subscription)
}

fn same_error(a: &ObservableError, b: &ObservableError) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// `err is TaskCanceledException tcException && tcException.Task == task`.
fn is_cancellation_of(error: &ObservableError, task: &Task<i32>) -> bool {
    error.downcast_ref::<TaskCanceledError>().is_some_and(|canceled| canceled.task_id == Some(task.id()))
}

/// The values of a sequence whose tasks run on worker threads (`ToEnumerable()`), or its error.
fn to_enumerable<T: Clone + 'static>(res: Rc<dyn IObservable<T>>) -> Result<Vec<T>, ObservableError> {
    block_on(res.to_array().to_task())
}

fn select_many_task1_case(indexed: bool) {
    let res = many_async(Observable::range(0, 10), indexed, |x| run(move || Ok(x + 1)));
    let mut res = to_enumerable(res).unwrap();
    res.sort();
    assert_eq!((0..10).map(|x| x + 1).collect::<Vec<i32>>(), res);
}

fn select_many_task2_case(indexed: bool) {
    let res = many_async_token(Observable::range(0, 10), indexed, |x, _ct| run(move || Ok(x + 1)));
    let mut res = to_enumerable(res).unwrap();
    res.sort();
    assert_eq!((0..10).map(|x| x + 1).collect::<Vec<i32>>(), res);
}

fn select_many_task_task_throws_case(indexed: bool) {
    let ex = new_error("ex");

    let e = ex.clone();
    let res = many_async(Observable::range(0, 10), indexed, move |x| {
        let ex = e.clone();
        async move {
            let value = run(move || Ok(x + 1)).await;
            if x > 5 {
                Err(ex)
            } else {
                value
            }
        }
    });

    let error = to_enumerable(res).unwrap_err();
    assert!(same_error(&ex, &error));
}

fn select_many_task_selector_throws_case(indexed: bool) {
    let ex = new_error("ex");

    let e = ex.clone();
    let res = try_many_async(Observable::range(0, 10), indexed, move |x| if x > 5 { Err(e.clone()) } else { Ok(run(move || Ok(x + 1))) });

    let error = to_enumerable(res).unwrap_err();
    assert!(same_error(&ex, &error));
}

fn select_many_task_result_selector1_case(indexed: bool) {
    let res = many_async_with(Observable::range(0, 10), indexed, |x| run(move || Ok(x + 1)), |x, y| x + y);
    let mut res = to_enumerable(res).unwrap();
    res.sort();
    assert_eq!((0..10).map(|x| 2 * x + 1).collect::<Vec<i32>>(), res);
}

fn select_many_task_result_selector2_case(indexed: bool) {
    let res = many_async_token_with(Observable::range(0, 10), indexed, |x, _ct| run(move || Ok(x + 1)), |x, y| x + y);
    let mut res = to_enumerable(res).unwrap();
    res.sort();
    assert_eq!((0..10).map(|x| 2 * x + 1).collect::<Vec<i32>>(), res);
}

fn select_many_task_result_selector_throws_case(indexed: bool) {
    let ex = new_error("ex");

    let e = ex.clone();
    let res = try_many_async_with(
        Observable::range(0, 10),
        indexed,
        |x| Ok(run(move || Ok(x + 1))),
        move |x, y| if x > 5 { Err(e.clone()) } else { Ok(x + y) },
    );

    let error = to_enumerable(res).unwrap_err();
    assert!(same_error(&ex, &error));
}

fn simple_ran_to_completion_async_case(indexed: bool) {
    let tcss = completion_sources(2);

    let res = many_async(Observable::range(0, 2), indexed, task_of(&tcss));

    let (received, _) = receive(&res);

    tcss[0].set_result(42);
    tcss[1].set_result(43);

    assert!(received.completed.get());

    assert_eq!(vec![42, 43], received.sorted());
}

fn simple_ran_to_completion_sync_case(indexed: bool) {
    let tcss = completion_sources(2);

    tcss[0].set_result(42);
    tcss[1].set_result(43);

    let res = many_async(Observable::range(0, 2), indexed, task_of(&tcss));

    let (received, _) = receive(&res);

    assert!(received.completed.get());

    assert_eq!(vec![42, 43], received.sorted());
}

fn simple_faulted_async_case(indexed: bool) {
    let tcss = completion_sources(3);

    let res = many_async(Observable::range(0, 3), indexed, task_of(&tcss));

    let (received, _) = receive(&res);

    let ex = new_error("ex");
    tcss[1].set_exception(ex.clone());

    assert!(received.error().is_some() || received.completed.get());

    assert!(received.values.borrow().is_empty());
    assert!(same_error(&ex, &received.error().unwrap()));
}

fn simple_faulted_sync_case(indexed: bool) {
    let tcss = completion_sources(3);

    let ex = new_error("ex");
    tcss[1].set_exception(ex.clone());

    let res = many_async(Observable::range(0, 3), indexed, task_of(&tcss));

    let (received, _) = receive(&res);

    assert!(received.error().is_some() || received.completed.get());

    assert!(received.values.borrow().is_empty());
    assert!(same_error(&ex, &received.error().unwrap()));
}

fn simple_canceled_async_case(indexed: bool) {
    let tcss = completion_sources(3);

    let res = many_async(Observable::range(0, 3), indexed, task_of(&tcss));

    let (received, _) = receive(&res);

    tcss[1].set_canceled();

    assert!(received.error().is_some() || received.completed.get());

    assert!(received.values.borrow().is_empty());
    assert!(is_cancellation_of(&received.error().unwrap(), &tcss[1].task()));
}

fn simple_canceled_sync_case(indexed: bool) {
    let tcss = completion_sources(3);

    tcss[1].set_canceled();

    let res = many_async(Observable::range(0, 3), indexed, task_of(&tcss));

    let (received, _) = receive(&res);

    assert!(received.error().is_some() || received.completed.get());

    assert!(received.values.borrow().is_empty());
    assert!(is_cancellation_of(&received.error().unwrap(), &tcss[1].task()));
}

fn simple_inner_complete_before_outer_case(indexed: bool) {
    let xs = Subject::new();

    let tcss = completion_sources(3);

    let res = many_async(xs.as_observable(), indexed, task_of(&tcss));

    let (received, _) = receive(&res);

    tcss[1].set_result(42);

    xs.on_next(0);
    xs.on_next(1);
    xs.on_next(2);

    tcss[0].set_result(43);
    tcss[2].set_result(44);

    xs.on_completed();

    assert!(received.completed.get());

    assert_eq!(vec![42, 43, 44], received.sorted());
}

fn simple_outer_complete_before_inner_case(indexed: bool) {
    let xs = Subject::new();

    let tcss = completion_sources(3);

    let res = many_async(xs.as_observable(), indexed, task_of(&tcss));

    let (received, _) = receive(&res);

    tcss[1].set_result(42);

    xs.on_next(0);
    xs.on_next(1);
    xs.on_next(2);
    xs.on_completed();

    tcss[0].set_result(43);
    tcss[2].set_result(44);

    assert!(received.completed.get());

    assert_eq!(vec![42, 43, 44], received.sorted());
}

/// A selector that registers `on_cancel(tcs)` with the token and returns the task of `tcs`
/// (`(x, token) => { var tcs = tcss[x]; token.Register(...); return tcs.Task; }`); `throw_at`
/// is the value for which the selector throws `ex`.
fn registering_selector(
    tcss: &[TaskCompletionSource<i32>],
    throw_at: Option<(i32, ObservableError)>,
    on_cancel: impl Fn(&TaskCompletionSource<i32>) + 'static,
) -> impl Fn(i32, CancellationToken) -> Result<Task<i32>, ObservableError> + 'static {
    let tcss = tcss.to_vec();
    let on_cancel = Rc::new(on_cancel);
    move |x, token| {
        if let Some((at, ex)) = &throw_at {
            if x == *at {
                return Err(ex.clone());
            }
        }

        let tcs = tcss[x as usize].clone();

        let canceled = tcs.clone();
        let on_cancel = on_cancel.clone();
        let _ = token.register_local(move || on_cancel(&canceled));

        Ok(tcs.task())
    }
}

/// [`registering_selector`] as a selector that does not fail (the form of upstream's tests whose
/// selector does not throw).
fn plain(selector: impl Fn(i32, CancellationToken) -> Result<Task<i32>, ObservableError> + 'static) -> impl Fn(i32, CancellationToken) -> Task<i32> + 'static {
    move |x, token| selector(x, token).expect("the selector does not fail")
}

/// The `token.Register(() => { n++; m += tcs.TrySetCanceled() ? 1 : 0; })` of the tests of the
/// cancellation.
fn counting_cancellation(n: &Rc<Cell<i32>>, m: &Rc<Cell<i32>>) -> impl Fn(&TaskCompletionSource<i32>) + 'static {
    let (n, m) = (n.clone(), m.clone());
    move |tcs| {
        n.set(n.get() + 1);
        m.set(m.get() + if tcs.try_set_canceled() { 1 } else { 0 });
    }
}

fn simple_cancellation_never_invoked_case(indexed: bool) {
    let xs = Subject::new();

    let tcss = completion_sources(3);

    // `tcs.SetCanceled()` of upstream runs when the subscription is disposed after the end, and
    // throws on the completed task in a continuation of the thread pool, which swallows it; the
    // continuations here run on the thread that ends the task, so the callback uses
    // `try_set_canceled`.
    let res = many_async_token(
        xs.as_observable(),
        indexed,
        plain(registering_selector(&tcss, None, |tcs| {
            tcs.try_set_canceled();
        })),
    );

    let (received, _d) = receive(&res);

    tcss[1].set_result(42);

    xs.on_next(0);
    xs.on_next(1);
    xs.on_next(2);
    xs.on_completed();

    tcss[0].set_result(43);
    tcss[2].set_result(44);

    assert!(received.completed.get());

    assert_eq!(vec![42, 43, 44], received.sorted());
}

fn simple_cancellation_invoked_case(indexed: bool) {
    let xs = Subject::new();

    let tcss = completion_sources(3);

    let n = Rc::new(Cell::new(0));
    let m = Rc::new(Cell::new(0));

    let res = many_async_token(xs.as_observable(), indexed, plain(registering_selector(&tcss, None, counting_cancellation(&n, &m))));

    let (received, d) = receive(&res);

    tcss[1].set_result(42);

    xs.on_next(0);
    xs.on_next(1);

    d.dispose();

    xs.on_next(2);
    xs.on_completed();

    assert!(!tcss[0].try_set_result(43));
    tcss[2].set_result(44); // never observed because xs.OnNext(2) happened after dispose

    assert_eq!(vec![42], *received.values.borrow());
    assert!(!received.completed.get());
    assert_eq!(2, n.get());
    assert_eq!(1, m.get()); // tcss[1] was already finished
}

fn simple_cancellation_after_outer_error_case(indexed: bool) {
    let xs = Subject::new();

    let tcss = completion_sources(3);

    let n = Rc::new(Cell::new(0));
    let m = Rc::new(Cell::new(0));

    let res = many_async_token(xs.as_observable(), indexed, plain(registering_selector(&tcss, None, counting_cancellation(&n, &m))));

    let (received, _) = receive(&res);

    tcss[1].set_result(42);

    xs.on_next(0);
    xs.on_next(1);

    let ex = new_error("ex");
    xs.on_error(ex.clone());

    assert!(!tcss[0].try_set_result(43));
    tcss[2].set_result(44); // no-op

    assert_eq!(vec![42], *received.values.borrow());
    assert!(same_error(&ex, &received.error().unwrap()));
    assert!(!received.completed.get());
    assert_eq!(2, n.get());
    assert_eq!(1, m.get()); // tcss[1] was already finished
}

fn simple_cancellation_after_selector_throws_case(indexed: bool) {
    let xs = Subject::new();

    let tcss = completion_sources(4);

    let n = Rc::new(Cell::new(0));
    let m = Rc::new(Cell::new(0));

    let ex = new_error("ex");

    let res = try_many_async_token(xs.as_observable(), indexed, registering_selector(&tcss, Some((2, ex.clone())), counting_cancellation(&n, &m)));

    let (received, _) = receive(&res);

    tcss[1].set_result(43);

    xs.on_next(0);
    xs.on_next(1);

    tcss[0].set_result(42);

    xs.on_next(2); // causes error
    xs.on_completed();

    assert!(received.error().is_some() || received.completed.get());

    assert!(!received.completed.get());
    assert!(same_error(&ex, &received.error().unwrap()));
    assert_eq!(2, n.get());
    assert_eq!(0, m.get());
}

fn with_result_selector_ran_to_completion_async_case(indexed: bool) {
    let tcss = completion_sources(2);

    let res = many_async_with(Observable::range(0, 2), indexed, task_of(&tcss), |x, y| x + y);

    let (received, _) = receive(&res);

    tcss[0].set_result(42);
    tcss[1].set_result(43);

    assert!(received.completed.get());

    assert_eq!(vec![42, 43 + 1], received.sorted());
}

fn with_result_selector_ran_to_completion_sync_case(indexed: bool) {
    let tcss = completion_sources(2);

    tcss[0].set_result(42);
    tcss[1].set_result(43);

    let res = many_async_with(Observable::range(0, 2), indexed, task_of(&tcss), |x, y| x + y);

    let (received, _) = receive(&res);

    assert!(received.completed.get());

    assert_eq!(vec![42, 43 + 1], received.sorted());
}

fn with_result_selector_faulted_async_case(indexed: bool) {
    let tcss = completion_sources(3);

    let res = many_async_with(Observable::range(0, 3), indexed, task_of(&tcss), |x, y| x + y);

    let (received, _) = receive(&res);

    let ex = new_error("ex");
    tcss[1].set_exception(ex.clone());

    assert!(received.error().is_some() || received.completed.get());

    assert!(received.values.borrow().is_empty());
    assert!(same_error(&ex, &received.error().unwrap()));
}

fn with_result_selector_faulted_sync_case(indexed: bool) {
    let tcss = completion_sources(3);

    let ex = new_error("ex");
    tcss[1].set_exception(ex.clone());

    let res = many_async_with(Observable::range(0, 3), indexed, task_of(&tcss), |x, y| x + y);

    let (received, _) = receive(&res);

    assert!(received.error().is_some() || received.completed.get());

    assert!(received.values.borrow().is_empty());
    assert!(same_error(&ex, &received.error().unwrap()));
}

fn with_result_selector_canceled_async_case(indexed: bool) {
    let tcss = completion_sources(3);

    let res = many_async_with(Observable::range(0, 3), indexed, task_of(&tcss), |x, y| x + y);

    let (received, _) = receive(&res);

    tcss[1].set_canceled();

    assert!(received.error().is_some() || received.completed.get());

    assert!(received.values.borrow().is_empty());
    assert!(is_cancellation_of(&received.error().unwrap(), &tcss[1].task()));
}

fn with_result_selector_canceled_sync_case(indexed: bool) {
    let tcss = completion_sources(3);

    tcss[1].set_canceled();

    let res = many_async_with(Observable::range(0, 3), indexed, task_of(&tcss), |x, y| x + y);

    let (received, _) = receive(&res);

    assert!(received.error().is_some() || received.completed.get());

    assert!(received.values.borrow().is_empty());
    assert!(is_cancellation_of(&received.error().unwrap(), &tcss[1].task()));
}

fn with_result_selector_inner_complete_before_outer_case(indexed: bool) {
    let xs = Subject::new();

    let tcss = completion_sources(3);

    let res = many_async_with(xs.as_observable(), indexed, task_of(&tcss), |x, y| x + y);

    let (received, _) = receive(&res);

    tcss[1].set_result(42);

    xs.on_next(0);
    xs.on_next(1);
    xs.on_next(2);

    tcss[0].set_result(43);
    tcss[2].set_result(44);

    xs.on_completed();

    assert!(received.completed.get());

    let mut expected = vec![42 + 1, 43, 44 + 2];
    expected.sort();
    assert_eq!(expected, received.sorted());
}

fn with_result_selector_outer_complete_before_inner_case(indexed: bool) {
    let xs = Subject::new();

    let tcss = completion_sources(3);

    let res = many_async_with(xs.as_observable(), indexed, task_of(&tcss), |x, y| x + y);

    let (received, _) = receive(&res);

    tcss[1].set_result(42);

    xs.on_next(0);
    xs.on_next(1);
    xs.on_next(2);
    xs.on_completed();

    tcss[0].set_result(43);
    tcss[2].set_result(44);

    assert!(received.completed.get());

    let mut expected = vec![42 + 1, 43, 44 + 2];
    expected.sort();
    assert_eq!(expected, received.sorted());
}

fn with_result_selector_cancellation_never_invoked_case(indexed: bool) {
    let xs = Subject::new();

    let tcss = completion_sources(3);

    // `try_set_canceled` for upstream's `SetCanceled`: see `simple_cancellation_never_invoked_case`.
    let res = many_async_token_with(
        xs.as_observable(),
        indexed,
        plain(registering_selector(&tcss, None, |tcs| {
            tcs.try_set_canceled();
        })),
        |x, y| x + y,
    );

    let (received, _d) = receive(&res);

    tcss[1].set_result(42);

    xs.on_next(0);
    xs.on_next(1);
    xs.on_next(2);
    xs.on_completed();

    tcss[0].set_result(43);
    tcss[2].set_result(44);

    assert!(received.completed.get());

    let mut expected = vec![42 + 1, 43, 44 + 2];
    expected.sort();
    assert_eq!(expected, received.sorted());
}

fn with_result_selector_cancellation_invoked_case(indexed: bool) {
    let xs = Subject::new();

    let tcss = completion_sources(3);

    let n = Rc::new(Cell::new(0));
    let m = Rc::new(Cell::new(0));

    let res = many_async_token_with(xs.as_observable(), indexed, plain(registering_selector(&tcss, None, counting_cancellation(&n, &m))), |x, y| x + y);

    let (received, d) = receive(&res);

    tcss[1].set_result(42);

    xs.on_next(0);
    xs.on_next(1);

    d.dispose();

    xs.on_next(2);
    xs.on_completed();

    assert!(!tcss[0].try_set_result(43));
    tcss[2].set_result(44); // never observed because xs.OnNext(2) happened after dispose

    assert_eq!(vec![42 + 1], *received.values.borrow());
    assert!(!received.completed.get());
    assert_eq!(2, n.get());
    assert_eq!(1, m.get()); // tcss[1] was already finished
}

fn with_result_selector_cancellation_after_outer_error_case(indexed: bool) {
    let xs = Subject::new();

    let tcss = completion_sources(3);

    let n = Rc::new(Cell::new(0));
    let m = Rc::new(Cell::new(0));

    let res = many_async_token_with(xs.as_observable(), indexed, plain(registering_selector(&tcss, None, counting_cancellation(&n, &m))), |x, y| x + y);

    let (received, _) = receive(&res);

    tcss[1].set_result(42);

    xs.on_next(0);
    xs.on_next(1);

    let ex = new_error("ex");
    xs.on_error(ex.clone());

    assert!(!tcss[0].try_set_result(43));
    tcss[2].set_result(44); // no-op

    assert_eq!(vec![42 + 1], *received.values.borrow());
    assert!(same_error(&ex, &received.error().unwrap()));
    assert!(!received.completed.get());
    assert_eq!(2, n.get());
    assert_eq!(1, m.get()); // tcss[1] was already finished
}

fn with_result_selector_cancellation_after_selector_throws_case(indexed: bool) {
    let xs = Subject::new();

    let tcss = completion_sources(4);

    let n = Rc::new(Cell::new(0));
    let m = Rc::new(Cell::new(0));

    let ex = new_error("ex");

    let res = try_many_async_token_with(
        xs.as_observable(),
        indexed,
        registering_selector(&tcss, Some((2, ex.clone())), counting_cancellation(&n, &m)),
        |x, y| Ok(x + y),
    );

    let (received, _) = receive(&res);

    tcss[1].set_result(43);

    xs.on_next(0);
    xs.on_next(1);

    tcss[0].set_result(42);

    xs.on_next(2); // causes error
    xs.on_completed();

    assert!(received.error().is_some() || received.completed.get());

    assert!(!received.completed.get());
    assert!(same_error(&ex, &received.error().unwrap()));
    assert_eq!(2, n.get());
    assert_eq!(0, m.get());
}

#[test]
fn select_many_task1() {
    select_many_task1_case(false);
}

#[test]
fn select_many_task2() {
    select_many_task2_case(false);
}

#[test]
fn select_many_task_task_throws() {
    select_many_task_task_throws_case(false);
}

#[test]
fn select_many_task_selector_throws() {
    select_many_task_selector_throws_case(false);
}

#[test]
fn select_many_task_result_selector1() {
    select_many_task_result_selector1_case(false);
}

#[test]
fn select_many_task_result_selector2() {
    select_many_task_result_selector2_case(false);
}

#[test]
fn select_many_task_result_selector_throws() {
    select_many_task_result_selector_throws_case(false);
}

#[test]
fn select_many_task_with_completion_source_simple_ran_to_completion_async() {
    simple_ran_to_completion_async_case(false);
}

#[test]
fn select_many_task_with_completion_source_simple_ran_to_completion_sync() {
    simple_ran_to_completion_sync_case(false);
}

#[test]
fn select_many_task_with_completion_source_simple_faulted_async() {
    simple_faulted_async_case(false);
}

#[test]
fn select_many_task_with_completion_source_simple_faulted_sync() {
    simple_faulted_sync_case(false);
}

#[test]
fn select_many_task_with_completion_source_simple_canceled_async() {
    simple_canceled_async_case(false);
}

#[test]
fn select_many_task_with_completion_source_simple_canceled_sync() {
    simple_canceled_sync_case(false);
}

#[test]
fn select_many_task_with_completion_source_simple_inner_complete_before_outer() {
    simple_inner_complete_before_outer_case(false);
}

#[test]
fn select_many_task_with_completion_source_simple_outer_complete_before_inner() {
    simple_outer_complete_before_inner_case(false);
}

#[test]
fn select_many_task_with_completion_source_simple_cancellation_never_invoked() {
    simple_cancellation_never_invoked_case(false);
}

#[test]
fn select_many_task_with_completion_source_simple_cancellation_invoked() {
    simple_cancellation_invoked_case(false);
}

#[test]
fn select_many_task_with_completion_source_simple_cancellation_after_outer_error() {
    simple_cancellation_after_outer_error_case(false);
}

#[test]
fn select_many_task_with_completion_source_simple_cancellation_after_selector_throws() {
    simple_cancellation_after_selector_throws_case(false);
}

#[test]
fn select_many_task_with_completion_source_with_result_selector_ran_to_completion_async() {
    with_result_selector_ran_to_completion_async_case(false);
}

#[test]
fn select_many_task_with_completion_source_with_result_selector_ran_to_completion_sync() {
    with_result_selector_ran_to_completion_sync_case(false);
}

#[test]
fn select_many_task_with_completion_source_with_result_selector_faulted_async() {
    with_result_selector_faulted_async_case(false);
}

#[test]
fn select_many_task_with_completion_source_with_result_selector_faulted_sync() {
    with_result_selector_faulted_sync_case(false);
}

#[test]
fn select_many_task_with_completion_source_with_result_selector_canceled_async() {
    with_result_selector_canceled_async_case(false);
}

#[test]
fn select_many_task_with_completion_source_with_result_selector_canceled_sync() {
    with_result_selector_canceled_sync_case(false);
}

#[test]
fn select_many_task_with_completion_source_with_result_selector_inner_complete_before_outer() {
    with_result_selector_inner_complete_before_outer_case(false);
}

#[test]
fn select_many_task_with_completion_source_with_result_selector_outer_complete_before_inner() {
    with_result_selector_outer_complete_before_inner_case(false);
}

#[test]
fn select_many_task_with_completion_source_with_result_selector_cancellation_never_invoked() {
    with_result_selector_cancellation_never_invoked_case(false);
}

#[test]
fn select_many_task_with_completion_source_with_result_selector_cancellation_invoked() {
    with_result_selector_cancellation_invoked_case(false);
}

#[test]
fn select_many_task_with_completion_source_with_result_selector_cancellation_after_outer_error() {
    with_result_selector_cancellation_after_outer_error_case(false);
}

#[test]
fn select_many_task_with_completion_source_with_result_selector_cancellation_after_selector_throws() {
    with_result_selector_cancellation_after_selector_throws_case(false);
}

#[test]
fn select_many_with_index_task_index() {
    let res = Observable::range(0, 10).select_many_async_with_index(|x, i| run(move || Ok((x, i))));
    let mut res = to_enumerable(res).unwrap();
    res.sort_by_key(|v| v.1);
    assert_eq!((0..10).map(|x| (x, x as usize)).collect::<Vec<(i32, usize)>>(), res);
}

#[test]
fn select_many_with_index_task_cancellation_index() {
    let res = Observable::range(0, 10).select_many_async_with_index_with_token(|x, i, _ctx| run(move || Ok((x, i))));
    let mut res = to_enumerable(res).unwrap();
    res.sort_by_key(|v| v.1);
    assert_eq!((0..10).map(|x| (x, x as usize)).collect::<Vec<(i32, usize)>>(), res);
}

#[test]
fn select_many_with_index_task_result_selector_index() {
    let res = Observable::range(0, 10).select_many_async_with_index_with(|x, i| run(move || Ok((x, i))), |_, _, r| r);
    let mut res = to_enumerable(res).unwrap();
    res.sort_by_key(|v| v.1);
    assert_eq!((0..10).map(|x| (x, x as usize)).collect::<Vec<(i32, usize)>>(), res);
}

#[test]
fn select_many_with_index_task_result_selector_cancellation_index() {
    let res = Observable::range(0, 10).select_many_async_with_index_with_token_with(|x, i, _ctx| run(move || Ok((x, i))), |_, _, r| r);
    let mut res = to_enumerable(res).unwrap();
    res.sort_by_key(|v| v.1);
    assert_eq!((0..10).map(|x| (x, x as usize)).collect::<Vec<(i32, usize)>>(), res);
}

#[test]
fn select_many_with_index_task1() {
    select_many_task1_case(true);
}

#[test]
fn select_many_with_index_task2() {
    select_many_task2_case(true);
}

#[test]
fn select_many_with_index_task_task_throws() {
    select_many_task_task_throws_case(true);
}

#[test]
fn select_many_with_index_task_selector_throws() {
    select_many_task_selector_throws_case(true);
}

#[test]
fn select_many_with_index_task_result_selector1() {
    select_many_task_result_selector1_case(true);
}

#[test]
fn select_many_with_index_task_result_selector2() {
    select_many_task_result_selector2_case(true);
}

#[test]
fn select_many_with_index_task_result_selector_throws() {
    select_many_task_result_selector_throws_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_simple_ran_to_completion_async() {
    simple_ran_to_completion_async_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_simple_ran_to_completion_sync() {
    simple_ran_to_completion_sync_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_simple_faulted_async() {
    simple_faulted_async_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_simple_faulted_sync() {
    simple_faulted_sync_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_simple_canceled_async() {
    simple_canceled_async_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_simple_canceled_sync() {
    simple_canceled_sync_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_simple_inner_complete_before_outer() {
    simple_inner_complete_before_outer_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_simple_outer_complete_before_inner() {
    simple_outer_complete_before_inner_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_simple_cancellation_never_invoked() {
    simple_cancellation_never_invoked_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_simple_cancellation_invoked() {
    simple_cancellation_invoked_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_simple_cancellation_after_outer_error() {
    simple_cancellation_after_outer_error_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_simple_cancellation_after_selector_throws() {
    simple_cancellation_after_selector_throws_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_with_result_selector_ran_to_completion_async() {
    with_result_selector_ran_to_completion_async_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_with_result_selector_ran_to_completion_sync() {
    with_result_selector_ran_to_completion_sync_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_with_result_selector_faulted_async() {
    with_result_selector_faulted_async_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_with_result_selector_faulted_sync() {
    with_result_selector_faulted_sync_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_with_result_selector_canceled_async() {
    with_result_selector_canceled_async_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_with_result_selector_canceled_sync() {
    with_result_selector_canceled_sync_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_with_result_selector_inner_complete_before_outer() {
    with_result_selector_inner_complete_before_outer_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_with_result_selector_outer_complete_before_inner() {
    with_result_selector_outer_complete_before_inner_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_with_result_selector_cancellation_never_invoked() {
    with_result_selector_cancellation_never_invoked_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_with_result_selector_cancellation_invoked() {
    with_result_selector_cancellation_invoked_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_with_result_selector_cancellation_after_outer_error() {
    with_result_selector_cancellation_after_outer_error_case(true);
}

#[test]
fn select_many_with_index_task_with_completion_source_with_result_selector_cancellation_after_selector_throws() {
    with_result_selector_cancellation_after_selector_throws_case(true);
}

// Not ported: SelectMany_Then_ArgumentChecking - argument validation
// Not ported: SelectMany_ArgumentChecking - argument validation
// Not ported: SelectManyWithIndex_ArgumentChecking - argument validation
// Not ported: SelectMany_QueryOperator_ArgumentChecking - argument validation
// Not ported: SelectManyWithIndex_QueryOperator_ArgumentChecking - argument validation
// Not ported: SelectMany_Triple_ArgumentChecking - argument validation
// Not ported: SelectManyWithIndex_Triple_ArgumentChecking - argument validation
// Not ported: SelectMany_Task_ArgumentChecking - argument validation
// Not ported: SelectManyWithIndex_Task_ArgumentChecking - argument validation
