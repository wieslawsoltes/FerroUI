//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/CatchTest.cs` of Rx.NET.
//! Adapted: `Catch(IEnumerable<IObservable<T>>)` and `Catch(params IObservable<T>[])` are
//! `Observable::catch_all`, and `Observable::try_catch_all` when the enumeration throws
//! (`RogueEnumerable`, an iterator that throws); the typed handlers `Catch((ArgumentException e)
//! => ...)` are `catch_of::<ArgumentException>`, with the exception types of the tests as error
//! types; a handler that throws is the `try_` form returning the error; `MockEnumerable` (of the
//! upstream tests) is ported below, its enumerator recording its disposal when it is dropped; the
//! `DefaultScheduler` tests run without the event they wait on (the sequences are synchronous).

use ferroui_reactive::{IObservable, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{
    assert_messages, assert_recorded_eq, assert_subscriptions, new_error, Expected, ReactiveTest as RT, Recorded, Subscription, TestScheduler,
};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

type Source = Rc<dyn IObservable<i32>>;
type TrySource = Result<Source, ObservableError>;

/// `ArgumentException`.
#[derive(Debug)]
struct ArgumentException(String);

impl fmt::Display for ArgumentException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ArgumentException {}

/// `InvalidOperationException`.
#[derive(Debug)]
struct InvalidOperationException(String);

impl fmt::Display for InvalidOperationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for InvalidOperationException {}

fn argument_exception(message: &str) -> ObservableError {
    Rc::new(ArgumentException(message.to_owned()))
}

fn invalid_operation_exception(message: &str) -> ObservableError {
    Rc::new(InvalidOperationException(message.to_owned()))
}

fn same_error(a: &ObservableError, b: &ObservableError) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// `MockEnumerable<T>`: a sequence whose enumerators record when they are made and disposed (an
/// enumerator is disposed when it is dropped).
struct MockEnumerable<T> {
    scheduler: TestScheduler,
    subscriptions: Rc<RefCell<Vec<Subscription>>>,
    underlying: Rc<dyn Fn() -> Box<dyn Iterator<Item = T>>>,
}

impl<T> Clone for MockEnumerable<T> {
    fn clone(&self) -> Self {
        Self { scheduler: self.scheduler.clone(), subscriptions: self.subscriptions.clone(), underlying: self.underlying.clone() }
    }
}

impl<T: 'static> MockEnumerable<T> {
    fn new(scheduler: &TestScheduler, underlying: impl Fn() -> Box<dyn Iterator<Item = T>> + 'static) -> Self {
        Self { scheduler: scheduler.clone(), subscriptions: Rc::new(RefCell::new(Vec::new())), underlying: Rc::new(underlying) }
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions.borrow().clone()
    }
}

impl<T: 'static> IntoIterator for MockEnumerable<T> {
    type Item = T;
    type IntoIter = MockEnumerator<T>;

    fn into_iter(self) -> MockEnumerator<T> {
        let index = {
            let mut subscriptions = self.subscriptions.borrow_mut();
            subscriptions.push(Subscription::starting(self.scheduler.clock()));
            subscriptions.len() - 1
        };
        MockEnumerator { scheduler: self.scheduler.clone(), subscriptions: self.subscriptions.clone(), index, enumerator: (self.underlying)() }
    }
}

/// `MockEnumerable<T>.MockEnumerator`.
struct MockEnumerator<T> {
    scheduler: TestScheduler,
    subscriptions: Rc<RefCell<Vec<Subscription>>>,
    index: usize,
    enumerator: Box<dyn Iterator<Item = T>>,
}

impl<T> Iterator for MockEnumerator<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.enumerator.next()
    }
}

impl<T> Drop for MockEnumerator<T> {
    fn drop(&mut self) {
        let mut subscriptions = self.subscriptions.borrow_mut();
        let subscribe = subscriptions[self.index].subscribe;
        subscriptions[self.index] = Subscription::new(subscribe, self.scheduler.clock());
    }
}

#[test]
fn catch_ie_of_io_get_enumerator_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    // `RogueEnumerable`: `GetEnumerator` throws.
    let res = scheduler.start({
        let ex = ex.clone();
        move || Observable::try_catch_all(move || Err::<Vec<TrySource>, _>(ex.clone()))
    });

    assert_messages!(res.messages(), RT::on_error(200, ex.clone()));
}

#[test]
fn catch_ie_of_io() {
    let scheduler = TestScheduler::new();

    let xs1 = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_error(40, new_error("ex"))]);

    let xs2 = scheduler.create_cold_observable(vec![RT::on_next(10, 4), RT::on_next(20, 5), RT::on_error(30, new_error("ex"))]);

    let xs3 = scheduler.create_cold_observable(vec![
        RT::on_next(10, 6),
        RT::on_next(20, 7),
        RT::on_next(30, 8),
        RT::on_next(40, 9),
        RT::on_completed(50),
    ]);

    let res = scheduler.start({
        let sources: Vec<Source> = vec![xs1.as_observable(), xs2.as_observable(), xs3.as_observable()];
        move || Observable::catch_all(sources)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 1),
        RT::on_next(220, 2),
        RT::on_next(230, 3),
        RT::on_next(250, 4),
        RT::on_next(260, 5),
        RT::on_next(280, 6),
        RT::on_next(290, 7),
        RT::on_next(300, 8),
        RT::on_next(310, 9),
        RT::on_completed(320)
    );

    assert_subscriptions!(xs1.subscriptions(), RT::subscribe(200, 240));

    assert_subscriptions!(xs2.subscriptions(), RT::subscribe(240, 270));

    assert_subscriptions!(xs3.subscriptions(), RT::subscribe(270, 320));
}

#[test]
fn catch_no_errors() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_completed(250)]);

    let res = scheduler.start({
        let (o1, o2) = (o1.clone(), o2.clone());
        move || o1.as_observable().catch_with(o2.as_observable())
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_completed(230));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions());
}

#[test]
fn catch_never() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_completed(250)]);

    let res = scheduler.start({
        let (o1, o2) = (o1.clone(), o2.clone());
        move || o1.as_observable().catch_with(o2.as_observable())
    });

    assert_messages!(res.messages());

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(o2.subscriptions());
}

#[test]
fn catch_empty() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_completed(250)]);

    let res = scheduler.start({
        let (o1, o2) = (o1.clone(), o2.clone());
        move || o1.as_observable().catch_with(o2.as_observable())
    });

    assert_messages!(res.messages(), RT::on_completed(230));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions());
}

#[test]
fn catch_return() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_completed(250)]);

    let res = scheduler.start({
        let (o1, o2) = (o1.clone(), o2.clone());
        move || o1.as_observable().catch_with(o2.as_observable())
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_completed(230));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions());
}

#[test]
fn catch_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_error(230, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_completed(250)]);

    let res = scheduler.start({
        let (o1, o2) = (o1.clone(), o2.clone());
        move || o1.as_observable().catch_with(o2.as_observable())
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_next(240, 4), RT::on_completed(250));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 250));
}

#[test]
fn catch_error_never() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_error(230, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let res = scheduler.start({
        let (o1, o2) = (o1.clone(), o2.clone());
        move || o1.as_observable().catch_with(o2.as_observable())
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 1000));
}

#[test]
fn catch_error_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_error(230, new_error("ex1"))]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_error(250, ex.clone())]);

    let res = scheduler.start({
        let (o1, o2) = (o1.clone(), o2.clone());
        move || o1.as_observable().catch_with(o2.as_observable())
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_next(240, 4), RT::on_error(250, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 250));
}

#[test]
fn catch_multiple() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_error(215, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(220, 3), RT::on_error(225, ex.clone())]);

    let o3 = scheduler.create_hot_observable(vec![RT::on_next(230, 4), RT::on_completed(235)]);

    let res = scheduler.start({
        let sources: Vec<Source> = vec![o1.as_observable(), o2.as_observable(), o3.as_observable()];
        move || Observable::catch_all(sources)
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_next(230, 4), RT::on_completed(235));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 215));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(215, 225));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(225, 235));
}

#[test]
fn catch_error_specific_caught() {
    let scheduler = TestScheduler::new();

    let ex = argument_exception("x");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_error(230, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_completed(250)]);

    let handler_called = Rc::new(Cell::new(None::<i64>));

    let res = scheduler.start({
        let (o1, o2, clock, handler_called) = (o1.clone(), o2.clone(), scheduler.clone(), handler_called.clone());
        move || {
            o1.as_observable().catch_of::<ArgumentException>(move |_| {
                handler_called.set(Some(clock.clock()));
                o2.as_observable()
            })
        }
    });

    assert_eq!(Some(230), handler_called.get());

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_next(240, 4), RT::on_completed(250));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 250));
}

#[test]
fn catch_error_specific_uncaught() {
    let scheduler = TestScheduler::new();

    let ex = invalid_operation_exception("x");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_error(230, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_completed(250)]);

    let handler_called = Rc::new(Cell::new(None::<i64>));

    let res = scheduler.start({
        let (o1, o2, clock, handler_called) = (o1.clone(), o2.clone(), scheduler.clone(), handler_called.clone());
        move || {
            o1.as_observable().catch_of::<ArgumentException>(move |_| {
                handler_called.set(Some(clock.clock()));
                o2.as_observable()
            })
        }
    });

    assert_eq!(None, handler_called.get());

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_error(230, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions());
}

#[test]
fn catch_handler_throws() {
    let scheduler = TestScheduler::new();

    let ex1 = argument_exception("x");
    let ex2 = new_error("ex2");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_error(230, ex1.clone())]);

    let handler_called = Rc::new(Cell::new(None::<i64>));

    let res = scheduler.start({
        let (o1, ex2, clock, handler_called) = (o1.clone(), ex2.clone(), scheduler.clone(), handler_called.clone());
        move || {
            o1.as_observable().try_catch_of::<ArgumentException>(move |_| {
                handler_called.set(Some(clock.clock()));
                Err(ex2.clone())
            })
        }
    });

    assert_eq!(Some(230), handler_called.get());

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_error(230, ex2.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn catch_nested_outer_catches() {
    let scheduler = TestScheduler::new();

    let ex = argument_exception("x");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_error(215, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(220, 3), RT::on_completed(225)]);

    let o3 = scheduler.create_hot_observable(vec![
        RT::on_next(220, 4), // (!)
        RT::on_completed(225),
    ]);

    let first_handler_called = Rc::new(Cell::new(None::<i64>));
    let second_handler_called = Rc::new(Cell::new(None::<i64>));

    let res = scheduler.start({
        let (o1, o2, o3) = (o1.clone(), o2.clone(), o3.clone());
        let (clock, first, second) = (scheduler.clone(), first_handler_called.clone(), second_handler_called.clone());
        move || {
            let clock2 = clock.clone();
            o1.as_observable()
                .catch_of::<InvalidOperationException>(move |_| {
                    first.set(Some(clock.clock()));
                    o2.as_observable()
                })
                .catch_of::<ArgumentException>(move |_| {
                    second.set(Some(clock2.clock()));
                    o3.as_observable()
                })
        }
    });

    assert_eq!(None, first_handler_called.get());
    assert_eq!(Some(215), second_handler_called.get());

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 4), RT::on_completed(225));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 215));

    assert_subscriptions!(o2.subscriptions());

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(215, 225));
}

#[test]
fn catch_nested_inner_catches() {
    let scheduler = TestScheduler::new();

    let ex = argument_exception("x");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_error(215, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![
        RT::on_next(220, 3), // (!)
        RT::on_completed(225),
    ]);

    let o3 = scheduler.create_hot_observable(vec![RT::on_next(220, 4), RT::on_completed(225)]);

    let first_handler_called = Rc::new(Cell::new(None::<i64>));
    let second_handler_called = Rc::new(Cell::new(None::<i64>));

    let res = scheduler.start({
        let (o1, o2, o3) = (o1.clone(), o2.clone(), o3.clone());
        let (clock, first, second) = (scheduler.clone(), first_handler_called.clone(), second_handler_called.clone());
        move || {
            let clock2 = clock.clone();
            o1.as_observable()
                .catch_of::<ArgumentException>(move |_| {
                    first.set(Some(clock.clock()));
                    o2.as_observable()
                })
                .catch_of::<InvalidOperationException>(move |_| {
                    second.set(Some(clock2.clock()));
                    o3.as_observable()
                })
        }
    });

    assert_eq!(Some(215), first_handler_called.get());
    assert_eq!(None, second_handler_called.get());

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_completed(225));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 215));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(215, 225));

    assert_subscriptions!(o3.subscriptions());
}

#[test]
fn catch_throw_from_nested_catch() {
    let scheduler = TestScheduler::new();

    let ex1 = argument_exception("x1");
    let ex2 = argument_exception("x2");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_error(215, ex1.clone())]);

    let o2 = scheduler.create_hot_observable(vec![
        RT::on_next(220, 3), // (!)
        RT::on_error(225, ex2.clone()),
    ]);

    let o3 = scheduler.create_hot_observable(vec![RT::on_next(230, 4), RT::on_completed(235)]);

    let first_handler_called = Rc::new(Cell::new(None::<i64>));
    let second_handler_called = Rc::new(Cell::new(None::<i64>));

    let res = scheduler.start({
        let (o1, o2, o3, ex1, ex2) = (o1.clone(), o2.clone(), o3.clone(), ex1.clone(), ex2.clone());
        let (clock, first, second) = (scheduler.clone(), first_handler_called.clone(), second_handler_called.clone());
        move || {
            let clock2 = clock.clone();
            o1.as_observable()
                .catch_of::<ArgumentException>(move |e| {
                    first.set(Some(clock.clock()));
                    assert!(same_error(&ex1, &e), "Expected ex1");
                    o2.as_observable()
                })
                .catch_of::<ArgumentException>(move |e| {
                    second.set(Some(clock2.clock()));
                    assert!(same_error(&ex2, &e), "Expected ex2");
                    o3.as_observable()
                })
        }
    });

    assert_eq!(Some(215), first_handler_called.get());
    assert_eq!(Some(225), second_handler_called.get());

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_next(230, 4), RT::on_completed(235));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 215));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(215, 225));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(225, 235));
}

#[test]
fn catch_default_scheduler_binary() {
    let res = Rc::new(Cell::new(0));
    let _d = Observable::return_(1).catch_with(Observable::return_(2)).subscribe_next({
        let res = res.clone();
        move |x| res.set(x)
    });

    assert_eq!(1, res.get());
}

#[test]
fn catch_default_scheduler_nary() {
    let res = Rc::new(Cell::new(0));
    let _d = Observable::catch_all(vec![Observable::return_(1), Observable::return_(2), Observable::return_(3)]).subscribe_next({
        let res = res.clone();
        move |x| res.set(x)
    });

    assert_eq!(1, res.get());
}

#[test]
fn catch_default_scheduler_nary_enumerable() {
    let sources: Vec<Source> = vec![Observable::return_(1), Observable::return_(2), Observable::return_(3)];

    let res = Rc::new(Cell::new(0));
    let _d = Observable::catch_all(sources).subscribe_next({
        let res = res.clone();
        move |x| res.set(x)
    });

    assert_eq!(1, res.get());
}

#[test]
fn catch_empty_iterator() {
    let scheduler = TestScheduler::new();

    let res = scheduler.start(|| Observable::catch_all(Vec::<Source>::new()));

    assert_messages!(res.messages(), RT::on_completed(200));
}

#[test]
fn catch_iterator_throws() {
    let scheduler = TestScheduler::new();
    let ex = new_error("ex");

    // `Catch_IteratorThrows_Source(ex, true)`: the first `MoveNext` throws.
    let res = scheduler.start({
        let ex = ex.clone();
        move || Observable::try_catch_all(move || Ok::<_, ObservableError>(std::iter::once(Err::<Source, _>(ex.clone()))))
    });

    assert_messages!(res.messages(), RT::on_error(200, ex.clone()));
}

#[test]
fn catch_enumerable_throws() {
    let scheduler = TestScheduler::new();

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_completed(225)]);

    let ex = new_error("ex");
    // `GetObservablesForCatchThrow(o, ex)`: yields `o`, then throws.
    let xss = MockEnumerable::new(&scheduler, {
        let (o, ex) = (o.as_observable(), ex.clone());
        move || Box::new(vec![Ok(o.clone()), Err(ex.clone())].into_iter()) as Box<dyn Iterator<Item = TrySource>>
    });

    let res = scheduler.start({
        let xss = xss.clone();
        move || Observable::try_catch_all(move || Ok::<_, ObservableError>(xss.clone()))
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_completed(225));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 225));

    assert_subscriptions!(xss.subscriptions(), RT::subscribe(200, 225));
}

#[test]
fn catch_enumerable_timing() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2), // !
        RT::on_next(220, 3), // !
        RT::on_error(230, new_error("ex")),
    ]);

    let o2 = scheduler.create_cold_observable(vec![
        RT::on_next(50, 4), // !
        RT::on_next(60, 5), // !
        RT::on_next(70, 6), // !
        RT::on_error(80, new_error("ex")),
    ]);

    let o3 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(200, 2),
        RT::on_next(210, 3),
        RT::on_next(220, 4),
        RT::on_next(230, 5),
        RT::on_next(270, 6),
        RT::on_next(320, 7), // !
        RT::on_next(330, 8), // !
        RT::on_completed(340),
    ]);

    let xss = MockEnumerable::new(&scheduler, {
        let sources: Vec<Source> = vec![o1.as_observable(), o2.as_observable(), o3.as_observable(), o2.as_observable()];
        move || Box::new(sources.clone().into_iter()) as Box<dyn Iterator<Item = Source>>
    });

    let res = scheduler.start({
        let xss = xss.clone();
        move || Observable::catch_all(xss)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(280, 4),
        RT::on_next(290, 5),
        RT::on_next(300, 6),
        RT::on_next(320, 7),
        RT::on_next(330, 8),
        RT::on_completed(340)
    );

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 310));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(310, 340));

    assert_subscriptions!(xss.subscriptions(), RT::subscribe(200, 340));
}

#[test]
fn catch_enumerable_dispose() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_error(230, new_error("ex"))]);

    let o2 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(200, 2),
        RT::on_next(210, 3),
        RT::on_next(240, 4),
        RT::on_next(270, 5),
        RT::on_next(320, 6),
        RT::on_next(330, 7),
        RT::on_completed(340),
    ]);

    let xss = MockEnumerable::new(&scheduler, {
        let sources: Vec<Source> = vec![o1.as_observable(), o2.as_observable()];
        move || Box::new(sources.clone().into_iter()) as Box<dyn Iterator<Item = Source>>
    });

    let res = scheduler.start_disposed(
        {
            let xss = xss.clone();
            move || Observable::catch_all(xss)
        },
        300,
    );

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_next(240, 4), RT::on_next(270, 5));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 300));

    assert_subscriptions!(xss.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn catch_tail_recursive1() {
    let create = 0;
    let start = 200;
    let end = 1000;

    let scheduler = TestScheduler::new();

    let o = scheduler
        .create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_error(40, new_error("ex"))])
        .as_observable();

    fn f(o: Source) -> Source {
        Observable::defer(move || o.catch_with(f(o.clone())))
    }
    let res = scheduler.start_with(move || f(o), create, start, end);

    let mut expected: Vec<Recorded<Expected<i32>>> = Vec::new();

    let mut t = start;
    while t <= end {
        let n = (t - start) / 10;
        if n % 4 != 0 {
            expected.push(RT::on_next(t, (n % 4) as i32).into());
        }

        t += 10;
    }

    assert_recorded_eq(&res.messages(), &expected);
}

// Not ported: Catch_ArgumentChecking - argument validation
// Not ported: Catch_TailRecursive2 - real threads (ThreadPoolScheduler, stack traces); blocking operator ForEach (waived: one thread)
// Not ported: Catch_TailRecursive3 - blocking operator Wait (waived: one thread)
