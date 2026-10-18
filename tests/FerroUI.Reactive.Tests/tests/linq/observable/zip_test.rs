//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ZipTest.cs` of Rx.NET: the tests of the
//! binary form, of the form with an enumerable and of the form of an enumerable of sources, in the
//! order of the upstream file; the tests of the forms of two to sixteen sources (the regions
//! `Never/Never`, `Empty/Empty`, `SymmetricReturn`, `SelectorThrows` and `AllCompleted`) are in
//! `zip_arity_test.rs`.
//! Adapted: a selector that throws is the selector of `try_zip` or `try_zip_iter` returning the
//! error. `MockEnumerable<T>` (of the test helpers upstream) is defined here, as an iterable whose
//! iterator records the virtual time it was made at (`GetEnumerator`) and dropped at (`Dispose`). An
//! enumerable whose `GetEnumerator`, `MoveNext` or `Current` throws is the enumeration of
//! `try_zip_iter`: a function that fails, or an iterable of `Result`s (a throwing `MoveNext` and a
//! throwing `Current` are both an `Err` item). `EnumerableNever` (whose `MoveNext` waits for an event
//! the test sets at its end) is an iterable whose iterator panics when it moves, which these tests
//! never do. An enumeration of the sources of `Observable.Zip` that throws, which escapes `Subscribe`
//! upstream, is an iteration that panics, checked with `assert_throws`.

use ferroui_reactive::disposables::SingleAssignmentDisposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{IDisposable, IObservable, IObserver, NotificationKind, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, assert_throws, new_error, ReactiveTest as RT, Subscription, TestScheduler};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An iterable that records the virtual times of its iterations (`MockEnumerable<T>`).
#[derive(Clone)]
struct MockEnumerable<I> {
    scheduler: TestScheduler,
    subscriptions: Rc<RefCell<Vec<Subscription>>>,
    underlying: I,
}

impl<I> MockEnumerable<I> {
    fn new(scheduler: &TestScheduler, underlying: I) -> Self {
        Self { scheduler: scheduler.clone(), subscriptions: Rc::new(RefCell::new(Vec::new())), underlying }
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions.borrow().clone()
    }
}

/// The iterator of a [`MockEnumerable`] (`MockEnumerator`).
struct MockEnumerator<E> {
    scheduler: TestScheduler,
    subscriptions: Rc<RefCell<Vec<Subscription>>>,
    index: usize,
    inner: E,
}

impl<I: IntoIterator> IntoIterator for MockEnumerable<I> {
    type Item = I::Item;
    type IntoIter = MockEnumerator<I::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        let index = self.subscriptions.borrow().len();
        self.subscriptions.borrow_mut().push(Subscription::starting(self.scheduler.clock()));
        MockEnumerator { scheduler: self.scheduler, subscriptions: self.subscriptions, index, inner: self.underlying.into_iter() }
    }
}

impl<E: Iterator> Iterator for MockEnumerator<E> {
    type Item = E::Item;

    fn next(&mut self) -> Option<E::Item> {
        self.inner.next()
    }
}

impl<E> Drop for MockEnumerator<E> {
    fn drop(&mut self) {
        let mut subscriptions = self.subscriptions.borrow_mut();
        let subscribe = subscriptions[self.index].subscribe;
        subscriptions[self.index] = Subscription::new(subscribe, self.scheduler.clock());
    }
}

#[test]
fn zip_never_empty() {
    let scheduler = TestScheduler::new();

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let (a, b) = (n.as_observable(), e.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages());

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn zip_empty_never() {
    let scheduler = TestScheduler::new();

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let (a, b) = (e.as_observable(), n.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages());

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn zip_empty_empty() {
    let scheduler = TestScheduler::new();

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let (a, b) = (e1.as_observable(), e2.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_completed(210));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn zip_empty_non_empty() {
    let scheduler = TestScheduler::new();

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    // Intended behavior - will only know at 215 there was no error and we can complete gracefully.
    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(220)]);

    let (a, b) = (e.as_observable(), o.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_completed(215));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 215));
}

#[test]
fn zip_non_empty_empty() {
    let scheduler = TestScheduler::new();

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(220)]);

    let (a, b) = (o.as_observable(), e.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_completed(215));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 215));
}

#[test]
fn zip_never_non_empty() {
    let scheduler = TestScheduler::new();

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(220)]);

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (a, b) = (n.as_observable(), o.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages());

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn zip_non_empty_never() {
    let scheduler = TestScheduler::new();

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(220)]);

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (a, b) = (o.as_observable(), n.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages());

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn zip_non_empty_non_empty() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(230)]);

    // Intended behavior - will only know at 240 there was no error and we can complete gracefully.
    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 3), RT::on_completed(240)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_next(220, 2 + 3), RT::on_completed(240));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 240));
}

#[test]
fn zip_empty_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let f = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (a, b) = (e.as_observable(), f.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(f.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn zip_error_empty() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let f = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (a, b) = (f.as_observable(), e.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(f.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn zip_never_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let f = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (a, b) = (n.as_observable(), f.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(f.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn zip_error_never() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let f = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (a, b) = (f.as_observable(), n.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(f.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn zip_error_error() {
    let scheduler = TestScheduler::new();

    let ex1 = new_error("ex1");
    let ex2 = new_error("ex2");

    let f1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(230, ex1.clone())]);

    let f2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex2.clone())]);

    let (a, b) = (f1.as_observable(), f2.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex2.clone()));

    assert_subscriptions!(f1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(f2.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn zip_some_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(230)]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (a, b) = (o.as_observable(), e.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn zip_error_some() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(230)]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (a, b) = (e.as_observable(), o.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn zip_left_completes_first() {
    let scheduler = TestScheduler::new();

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(220)]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 4), RT::on_completed(225)]);

    let (a, b) = (o.as_observable(), e.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_next(215, 6), RT::on_completed(225));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 225));
}

#[test]
fn zip_right_completes_first() {
    let scheduler = TestScheduler::new();

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 4), RT::on_completed(225)]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(220)]);

    let (a, b) = (o.as_observable(), e.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_next(215, 6), RT::on_completed(225));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 225));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn zip_left_triggers_selector_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2)]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 4)]);

    let (a, b, ex1) = (o.as_observable(), e.as_observable(), ex.clone());
    let res = scheduler.start(move || a.try_zip(b, move |x, y| if x == y { Ok(42) } else { Err(ex1.clone()) }));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn zip_right_triggers_selector_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2)]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 4)]);

    let (a, b, ex1) = (o.as_observable(), e.as_observable(), ex.clone());
    let res = scheduler.start(move || a.try_zip(b, move |x, y| if x == y { Ok(42) } else { Err(ex1.clone()) }));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220));
}

/// The checks of `Zip_SomeData*`: one sum for each pair of values, at the later of their times.
fn assert_some_data(msgs1: &[ferroui_reactive_testing::Recorded<ferroui_reactive::Notification<i32>>], msgs2: &[ferroui_reactive_testing::Recorded<ferroui_reactive::Notification<i32>>], len1: usize, len2: usize) {
    let scheduler = TestScheduler::new();

    let len = len1.min(len2);

    let o1 = scheduler.create_hot_observable(msgs1.to_vec());
    let o2 = scheduler.create_hot_observable(msgs2.to_vec());

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.zip(b, |x, y| x + y));

    let messages = res.messages();
    assert!(len == messages.len(), "length");
    for i in 0..len {
        let sum = msgs1[i].value.value().unwrap() + msgs2[i].value.value().unwrap();
        let time = msgs1[i].time.max(msgs2[i].time);

        assert!(messages[i].time == time);
        assert!(messages[i].value.kind() == NotificationKind::OnNext);
        assert!(*messages[i].value.value().unwrap() == sum, "{i}");
    }
}

#[test]
fn zip_some_data_asymmetric1() {
    let msgs1: Vec<_> = (0..5).map(|i: i64| RT::on_next(205 + i * 5, i as i32)).collect();
    let msgs2: Vec<_> = (0..10).map(|i: i64| RT::on_next(202 + i * 8, i as i32)).collect();

    assert_some_data(&msgs1, &msgs2, msgs1.len(), msgs2.len());
}

#[test]
fn zip_some_data_asymmetric2() {
    let msgs1: Vec<_> = (0..10).map(|i: i64| RT::on_next(205 + i * 5, i as i32)).collect();
    let msgs2: Vec<_> = (0..5).map(|i: i64| RT::on_next(202 + i * 8, i as i32)).collect();

    assert_some_data(&msgs1, &msgs2, msgs1.len(), msgs2.len());
}

#[test]
fn zip_some_data_symmetric() {
    let msgs1: Vec<_> = (0..10).map(|i: i64| RT::on_next(205 + i * 5, i as i32)).collect();
    let msgs2: Vec<_> = (0..10).map(|i: i64| RT::on_next(202 + i * 8, i as i32)).collect();

    assert_some_data(&msgs1, &msgs2, msgs1.len(), msgs2.len());
}

#[test]
fn zip_selector_throws() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_next(225, 4), RT::on_completed(240)]);

    let o2 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 3),
        RT::on_next(230, 5), // !
        RT::on_completed(250),
    ]);

    let ex = new_error("ex");

    let (a, b, ex1) = (o1.as_observable(), o2.as_observable(), ex.clone());
    let res = scheduler.start(move || a.try_zip(b, move |x, y| if y == 5 { Err(ex1.clone()) } else { Ok(x + y) }));

    assert_messages!(res.messages(), RT::on_next(220, 2 + 3), RT::on_error(230, ex.clone()));
}

#[test]
fn zip_get_enumerator_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 42), RT::on_next(220, 43), RT::on_completed(230)]);

    // `RogueEnumerable<int>`: an enumerable whose `GetEnumerator` throws.
    let (a, ex1) = (xs.as_observable(), ex.clone());
    let res = scheduler.start(move || a.try_zip_iter(move || Err::<Vec<Result<i32, ObservableError>>, _>(ex1.clone()), |x: i32, y: i32| Ok(x + y)));

    assert_messages!(res.messages(), RT::on_error(200, ex.clone()));

    assert_subscriptions!(xs.subscriptions());
}

/// The iterator of `EnumerableNever(evt)`, whose `MoveNext` waits for an event the test sets at its
/// end: the tests never move it.
fn never_moved() -> Option<i32> {
    panic!("the enumerable that never ends is never moved by these tests")
}

/// `EnumerableNever(evt)`.
fn enumerable_never() -> std::iter::FromFn<fn() -> Option<i32>> {
    std::iter::from_fn(never_moved as fn() -> Option<i32>)
}

/// `ThrowEnumerable(false, ex)`: an enumerable whose first `MoveNext` throws.
fn throw_enumerable(ex: &ObservableError) -> Vec<Result<i32, ObservableError>> {
    vec![Err(ex.clone())]
}

/// `MyEnumerable(throwInCurrent, ex)`: an enumerator whose `MoveNext` throws (or, with
/// `throw_in_current`, whose `MoveNext` succeeds and whose `Current` throws); both are an `Err` item.
fn my_enumerable(_throw_in_current: bool, ex: &ObservableError) -> std::iter::Repeat<Result<i32, ObservableError>> {
    std::iter::repeat(Err(ex.clone()))
}

#[test]
fn zip_with_enumerable_never_never() {
    let scheduler = TestScheduler::new();

    let n1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let n2 = MockEnumerable::new(&scheduler, enumerable_never());

    let (a, b) = (n1.as_observable(), n2.clone());
    let res = scheduler.start(move || a.zip_iter(b, |x, y| x + y));

    assert_messages!(res.messages());

    assert_subscriptions!(n1.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(n2.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn zip_with_enumerable_never_empty() {
    let scheduler = TestScheduler::new();

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let e = MockEnumerable::new(&scheduler, Vec::<i32>::new());

    let (a, b) = (n.as_observable(), e.clone());
    let res = scheduler.start(move || a.zip_iter(b, |x, y| x + y));

    assert_messages!(res.messages());

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn zip_with_enumerable_empty_never() {
    let scheduler = TestScheduler::new();

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let n = MockEnumerable::new(&scheduler, enumerable_never());

    let (a, b) = (e.as_observable(), n.clone());
    let res = scheduler.start(move || a.zip_iter(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_completed(210));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn zip_with_enumerable_empty_empty() {
    let scheduler = TestScheduler::new();

    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let e2 = MockEnumerable::new(&scheduler, Vec::<i32>::new());

    let (a, b) = (e1.as_observable(), e2.clone());
    let res = scheduler.start(move || a.zip_iter(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_completed(210));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn zip_with_enumerable_empty_non_empty() {
    let scheduler = TestScheduler::new();

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);

    let o = MockEnumerable::new(&scheduler, vec![2]);

    let (a, b) = (e.as_observable(), o.clone());
    let res = scheduler.start(move || a.zip_iter(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_completed(210));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 210));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn zip_with_enumerable_non_empty_empty() {
    let scheduler = TestScheduler::new();

    let e = MockEnumerable::new(&scheduler, Vec::<i32>::new());

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(220)]);

    let (a, b) = (o.as_observable(), e.clone());
    let res = scheduler.start(move || a.zip_iter(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_completed(215));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 215));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 215));
}

#[test]
fn zip_with_enumerable_never_non_empty() {
    let scheduler = TestScheduler::new();

    let o = MockEnumerable::new(&scheduler, vec![2]);

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (a, b) = (n.as_observable(), o.clone());
    let res = scheduler.start(move || a.zip_iter(b, |x, y| x + y));

    assert_messages!(res.messages());

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn zip_with_enumerable_non_empty_non_empty() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(230)]);

    let o2 = MockEnumerable::new(&scheduler, vec![3]);

    let (a, b) = (o1.as_observable(), o2.clone());
    let res = scheduler.start(move || a.zip_iter(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_next(215, 2 + 3), RT::on_completed(230));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn zip_with_enumerable_empty_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let f = MockEnumerable::new(&scheduler, throw_enumerable(&ex));

    let (a, b) = (e.as_observable(), f.clone());
    let res = scheduler.start(move || a.try_zip_iter(move || Ok(b.clone()), |x: i32, y: i32| Ok(x + y)));

    assert_messages!(res.messages(), RT::on_completed(230));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(f.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn zip_with_enumerable_error_empty() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let e = MockEnumerable::new(&scheduler, Vec::<i32>::new());

    let f = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (a, b) = (f.as_observable(), e.clone());
    let res = scheduler.start(move || a.zip_iter(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(f.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn zip_with_enumerable_never_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let n = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let f = MockEnumerable::new(&scheduler, throw_enumerable(&ex));

    let (a, b) = (n.as_observable(), f.clone());
    let res = scheduler.start(move || a.try_zip_iter(move || Ok(b.clone()), |x: i32, y: i32| Ok(x + y)));

    assert_messages!(res.messages());

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(f.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn zip_with_enumerable_error_never() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let n = MockEnumerable::new(&scheduler, enumerable_never());

    let f = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (a, b) = (f.as_observable(), n.clone());
    let res = scheduler.start(move || a.zip_iter(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(n.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(f.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn zip_with_enumerable_error_error() {
    let scheduler = TestScheduler::new();

    let ex1 = new_error("ex1");
    let ex2 = new_error("ex2");

    let f1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(230, ex1.clone())]);

    let f2 = MockEnumerable::new(&scheduler, throw_enumerable(&ex2));

    let (a, b) = (f1.as_observable(), f2.clone());
    let res = scheduler.start(move || a.try_zip_iter(move || Ok(b.clone()), |x: i32, y: i32| Ok(x + y)));

    assert_messages!(res.messages(), RT::on_error(230, ex1.clone()));

    assert_subscriptions!(f1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(f2.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn zip_with_enumerable_some_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_completed(230)]);

    let e = MockEnumerable::new(&scheduler, throw_enumerable(&ex));

    let (a, b) = (o.as_observable(), e.clone());
    let res = scheduler.start(move || a.try_zip_iter(move || Ok(b.clone()), |x: i32, y: i32| Ok(x + y)));

    assert_messages!(res.messages(), RT::on_error(215, ex.clone()));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 215));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 215));
}

#[test]
fn zip_with_enumerable_error_some() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o = MockEnumerable::new(&scheduler, vec![2]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(220, ex.clone())]);

    let (a, b) = (e.as_observable(), o.clone());
    let res = scheduler.start(move || a.zip_iter(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220));
}

#[test]
fn zip_with_enumerable_some_data_both_sides() {
    let scheduler = TestScheduler::new();

    let o = MockEnumerable::new(&scheduler, vec![5, 4, 3, 2]);

    let e = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_next(230, 4), RT::on_next(240, 5)]);

    let (a, b) = (e.as_observable(), o.clone());
    let res = scheduler.start(move || a.zip_iter(b, |x, y| x + y));

    assert_messages!(res.messages(), RT::on_next(210, 7), RT::on_next(220, 7), RT::on_next(230, 7), RT::on_next(240, 7));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn zip_with_enumerable_enumerator_throws_move_next() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_next(225, 4), RT::on_completed(240)]);

    let o2 = MockEnumerable::new(&scheduler, my_enumerable(false, &ex));

    let (a, b) = (o1.as_observable(), o2.clone());
    let res = scheduler.start(move || a.try_zip_iter(move || Ok(b.clone()), |x: i32, y: i32| Ok(x + y)));

    assert_messages!(res.messages(), RT::on_error(215, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 215));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 215));
}

#[test]
fn zip_with_enumerable_enumerator_throws_current() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_next(225, 4), RT::on_completed(240)]);

    let o2 = MockEnumerable::new(&scheduler, my_enumerable(true, &ex));

    let (a, b) = (o1.as_observable(), o2.clone());
    let res = scheduler.start(move || a.try_zip_iter(move || Ok(b.clone()), |x: i32, y: i32| Ok(x + y)));

    assert_messages!(res.messages(), RT::on_error(215, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 215));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 215));
}

#[test]
fn zip_with_enumerable_selector_throws() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(215, 2), RT::on_next(225, 4), RT::on_completed(240)]);

    let o2 = MockEnumerable::new(&scheduler, vec![3, 5]);

    let ex = new_error("ex");

    let (a, b, ex1) = (o1.as_observable(), o2.clone(), ex.clone());
    let res = scheduler.start(move || {
        a.try_zip_iter(move || Ok(b.clone().into_iter().map(Ok::<i32, ObservableError>)), move |x: i32, y: i32| if y == 5 { Err(ex1.clone()) } else { Ok(x + y) })
    });

    assert_messages!(res.messages(), RT::on_next(215, 2 + 3), RT::on_error(225, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 225));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(200, 225));
}

/// The state of a [`MoveNextDisposeDetectEnumerable`] and of its enumerator.
struct DisposeDetectState {
    disposable: Rc<SingleAssignmentDisposable>,
    dispose_on_move_next: bool,
    move_next_running: Cell<bool>,
    current_running: Cell<bool>,
    disposed_while_move_next: Cell<bool>,
    disposed_while_current: Cell<bool>,
    is_disposed: Cell<bool>,
}

/// `MoveNextDisposeDetectEnumerable`: an enumerable whose enumerator disposes the subscription
/// while it moves (`MoveNext`) or while it gives its value (`Current`), and records whether it was
/// disposed meanwhile.
#[derive(Clone)]
struct MoveNextDisposeDetectEnumerable {
    state: Rc<DisposeDetectState>,
}

impl MoveNextDisposeDetectEnumerable {
    fn new(disposable: Rc<SingleAssignmentDisposable>, dispose_on_move_next: bool) -> Self {
        Self {
            state: Rc::new(DisposeDetectState {
                disposable,
                dispose_on_move_next,
                move_next_running: Cell::new(false),
                current_running: Cell::new(false),
                disposed_while_move_next: Cell::new(false),
                disposed_while_current: Cell::new(false),
                is_disposed: Cell::new(false),
            }),
        }
    }
}

/// The enumerator of a [`MoveNextDisposeDetectEnumerable`]: `next` is `MoveNext` then `Current`;
/// dropping it is `Dispose`.
struct DisposeDetectEnumerator {
    state: Rc<DisposeDetectState>,
}

impl IntoIterator for MoveNextDisposeDetectEnumerable {
    type Item = i32;
    type IntoIter = DisposeDetectEnumerator;

    fn into_iter(self) -> Self::IntoIter {
        DisposeDetectEnumerator { state: self.state }
    }
}

impl Iterator for DisposeDetectEnumerator {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        let state = self.state.clone();

        state.move_next_running.set(true);
        if state.dispose_on_move_next {
            state.disposable.dispose();
        }
        state.move_next_running.set(false);

        state.current_running.set(true);
        if !state.dispose_on_move_next {
            state.disposable.dispose();
        }
        state.current_running.set(false);
        Some(0)
    }
}

impl Drop for DisposeDetectEnumerator {
    fn drop(&mut self) {
        self.state.disposed_while_move_next.set(self.state.move_next_running.get());
        self.state.disposed_while_current.set(self.state.current_running.get());
        self.state.is_disposed.set(true);
    }
}

#[test]
fn zip_with_enumerable_no_async_dispose_on_move_next() {
    let source = Subject::<i32>::new();

    let disposable = SingleAssignmentDisposable::new();

    let other = MoveNextDisposeDetectEnumerable::new(disposable.clone(), true);

    disposable.set_disposable(Some(source.as_observable().zip_iter(other.clone(), |a, b| a + b).subscribe_ignore()));

    source.on_next(1);

    assert!(other.state.is_disposed.get());
    assert!(!other.state.disposed_while_move_next.get());
    assert!(!other.state.disposed_while_current.get());
}

#[test]
fn zip_with_enumerable_no_async_dispose_on_current() {
    let source = Subject::<i32>::new();

    let disposable = SingleAssignmentDisposable::new();

    let other = MoveNextDisposeDetectEnumerable::new(disposable.clone(), false);

    disposable.set_disposable(Some(source.as_observable().zip_iter(other.clone(), |a, b| a + b).subscribe_ignore()));

    source.on_next(1);

    assert!(other.state.is_disposed.get());
    assert!(!other.state.disposed_while_move_next.get());
    assert!(!other.state.disposed_while_current.get());
}

#[test]
fn zip_n_ary_symmetric() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(250, 4), RT::on_completed(420)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(240, 5), RT::on_completed(410)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(260, 6), RT::on_completed(400)]);

    let sources = vec![e0.as_observable(), e1.as_observable(), e2.as_observable()];
    let res = scheduler.start(move || Observable::zip_all(sources));

    assert_messages!(
        res.messages(),
        RT::on_next_where(230, |l: &Vec<i32>| *l == [1, 2, 3]),
        RT::on_next_where(260, |l: &Vec<i32>| *l == [4, 5, 6]),
        RT::on_completed(420)
    );

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 420));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 410));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn zip_n_ary_symmetric_selector() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(250, 4), RT::on_completed(420)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(240, 5), RT::on_completed(410)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(260, 6), RT::on_completed(400)]);

    let sources = vec![e0.as_observable(), e1.as_observable(), e2.as_observable()];
    let res = scheduler.start(move || Observable::zip_all_with(sources, |xs: Vec<i32>| xs.iter().sum::<i32>()));

    assert_messages!(res.messages(), RT::on_next(230, [1, 2, 3].iter().sum::<i32>()), RT::on_next(260, [4, 5, 6].iter().sum::<i32>()), RT::on_completed(420));

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 420));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 410));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 400));
}

#[test]
fn zip_n_ary_asymmetric() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(250, 4), RT::on_completed(270)]);
    let e1 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 2),
        RT::on_next(240, 5),
        RT::on_next(290, 7),
        RT::on_next(310, 9),
        RT::on_completed(410),
    ]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(260, 6), RT::on_next(280, 8), RT::on_completed(300)]);

    let sources = vec![e0.as_observable(), e1.as_observable(), e2.as_observable()];
    let res = scheduler.start(move || Observable::zip_all(sources));

    assert_messages!(
        res.messages(),
        RT::on_next_where(230, |l: &Vec<i32>| *l == [1, 2, 3]),
        RT::on_next_where(260, |l: &Vec<i32>| *l == [4, 5, 6]),
        RT::on_completed(310)
    );

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 270));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn zip_n_ary_asymmetric_selector() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(250, 4), RT::on_completed(270)]);
    let e1 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(220, 2),
        RT::on_next(240, 5),
        RT::on_next(290, 7),
        RT::on_next(310, 9),
        RT::on_completed(410),
    ]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(260, 6), RT::on_next(280, 8), RT::on_completed(300)]);

    let sources = vec![e0.as_observable(), e1.as_observable(), e2.as_observable()];
    let res = scheduler.start(move || Observable::zip_all_with(sources, |xs: Vec<i32>| xs.iter().sum::<i32>()));

    assert_messages!(res.messages(), RT::on_next(230, [1, 2, 3].iter().sum::<i32>()), RT::on_next(260, [4, 5, 6].iter().sum::<i32>()), RT::on_completed(310));

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 270));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 310));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn zip_n_ary_error() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_error(250, ex.clone())]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(240, 5), RT::on_completed(410)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(260, 6), RT::on_completed(400)]);

    let sources = vec![e0.as_observable(), e1.as_observable(), e2.as_observable()];
    let res = scheduler.start(move || Observable::zip_all(sources));

    assert_messages!(res.messages(), RT::on_next_where(230, |l: &Vec<i32>| *l == [1, 2, 3]), RT::on_error(250, ex.clone()));

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn zip_n_ary_error_selector() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_error(250, ex.clone())]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(240, 5), RT::on_completed(410)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(260, 6), RT::on_completed(400)]);

    let sources = vec![e0.as_observable(), e1.as_observable(), e2.as_observable()];
    let res = scheduler.start(move || Observable::zip_all_with(sources, |xs: Vec<i32>| xs.iter().sum::<i32>()));

    assert_messages!(res.messages(), RT::on_next(230, [1, 2, 3].iter().sum::<i32>()), RT::on_error(250, ex.clone()));

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 250));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 250));
}

/// `GetSources(Action start, params ITestableObservable<int>[] sources)`: an enumerable of sources
/// that runs `start` when it is enumerated.
#[derive(Clone)]
struct GetSources {
    start: Rc<dyn Fn()>,
    sources: Vec<Rc<dyn IObservable<i32>>>,
}

impl IntoIterator for GetSources {
    type Item = Rc<dyn IObservable<i32>>;
    type IntoIter = std::vec::IntoIter<Rc<dyn IObservable<i32>>>;

    fn into_iter(self) -> Self::IntoIter {
        (self.start)();
        self.sources.into_iter()
    }
}

#[test]
fn zip_n_ary_enumerable_simple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(250, 4), RT::on_completed(420)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(240, 5), RT::on_completed(410)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(260, 6), RT::on_completed(400)]);

    let started = Rc::new(Cell::new(0i64));
    let xss = {
        let (started, scheduler) = (started.clone(), scheduler.clone());
        GetSources { start: Rc::new(move || started.set(scheduler.clock())), sources: vec![e0.as_observable(), e1.as_observable(), e2.as_observable()] }
    };

    let res = scheduler.start(move || Observable::zip_all(xss));

    assert_eq!(200, started.get());

    assert_messages!(
        res.messages(),
        RT::on_next_where(230, |l: &Vec<i32>| *l == [1, 2, 3]),
        RT::on_next_where(260, |l: &Vec<i32>| *l == [4, 5, 6]),
        RT::on_completed(420)
    );

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 420));

    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 410));

    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 400));
}

/// The end of `GetSources<T>(Exception ex, params IObservable<T>[] sources)`, which throws.
fn throw_ex() -> Option<Rc<dyn IObservable<i32>>> {
    panic!("ex")
}

#[test]
fn zip_n_ary_enumerable_throws() {
    let xss = vec![Observable::return_(42)].into_iter().chain(std::iter::from_fn(throw_ex as fn() -> Option<Rc<dyn IObservable<i32>>>));
    let res = Observable::zip_all(xss);

    assert_throws(|| {
        res.subscribe_next(|_| {});
    });
}

#[test]
fn zip_at_least_one_throws4() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(230, ex.clone())]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);

    let (s0, s1, s2, s3) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable());
    let res = scheduler.start(move || s0.zip4(s1, s2, s3, |_, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_error(230, ex.clone()));

    assert_subscriptions!(e0.subscriptions(), RT::subscribe(200, 230));
    assert_subscriptions!(e1.subscriptions(), RT::subscribe(200, 230));
    assert_subscriptions!(e2.subscriptions(), RT::subscribe(200, 230));
    assert_subscriptions!(e3.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn zip2_with_immediate_return() {
    Observable::return_(()).zip(Observable::return_(()), |_, _| ()).subscribe_next(|_| {});
}

#[test]
fn zip3_with_immediate_return() {
    let result = Rc::new(Cell::new(0));

    let r = result.clone();
    Observable::return_(1).zip3(Observable::return_(2), Observable::return_(4), |a, b, c| a + b + c).subscribe_next(move |v| r.set(v));

    assert_eq!(7, result.get());
}

#[test]
fn zip_enumerable_with_immediate_return() {
    let sources: Vec<Rc<dyn IObservable<()>>> = (0..100).map(|_| Observable::return_(())).collect();
    Observable::zip_all(sources).subscribe_next(|_| {});
}

// Not ported: Zip_ArgumentChecking - argument validation
// Not ported: Zip_ArgumentCheckingHighArity - argument validation
// Not ported: Zip_NAry_ArgumentChecking - argument validation
