//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ToObservableTest.cs` of Rx.NET.
//! Adapted: `MockEnumerable<T>` (of the test helpers upstream) is defined here, as an iterable
//! whose iterator records the virtual time it was made at (`GetEnumerator`) and dropped at
//! (`Dispose`); an enumerable whose `MoveNext` throws is an iterable of `Result`s, converted with
//! `Observable::try_to_observable_on`.

use ferroui_reactive::{ObservableError, Observable};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, Subscription, TestScheduler};
use std::cell::RefCell;
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
fn enumerable_to_observable_complete() {
    let scheduler = TestScheduler::new();

    let e = MockEnumerable::new(&scheduler, vec![3, 1, 2, 4]);

    let (source, s) = (e.clone(), scheduler.scheduler());
    let results = scheduler.start(move || Observable::to_observable_on(source, s));

    assert_messages!(
        results.messages(),
        RT::on_next(201, 3),
        RT::on_next(202, 1),
        RT::on_next(203, 2),
        RT::on_next(204, 4),
        RT::on_completed::<i32>(205)
    );

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 205));
}

#[test]
fn enumerable_to_observable_dispose() {
    let scheduler = TestScheduler::new();

    let e = MockEnumerable::new(&scheduler, vec![3, 1, 2, 4]);

    let (source, s) = (e.clone(), scheduler.scheduler());
    let results = scheduler.start_disposed(move || Observable::to_observable_on(source, s), 203);

    assert_messages!(results.messages(), RT::on_next(201, 3), RT::on_next(202, 1));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 203));
}

#[test]
fn enumerable_to_observable_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let e = MockEnumerable::new(&scheduler, enumerable_to_observable_error_core(ex.clone()));

    let (source, s) = (e.clone(), scheduler.scheduler());
    let results = scheduler.start(move || Observable::try_to_observable_on(source, s));

    assert_messages!(results.messages(), RT::on_next(201, 1), RT::on_next(202, 2), RT::on_error::<i32>(203, ex));

    assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 203));
}

/// `yield return 1; yield return 2; throw ex;`.
fn enumerable_to_observable_error_core(ex: ObservableError) -> Vec<Result<i32, ObservableError>> {
    vec![Ok(1), Ok(2), Err(ex)]
}

// Not ported: EnumerableToObservable_ArgumentChecking - argument validation (null arguments, an enumerable whose GetEnumerator returns null)
// Not ported: EnumerableToObservable_Default_ArgumentChecking - argument validation
// Not ported: EnumerableToObservable_Default - real threads (compares with the sequence on DefaultScheduler.Instance, through the blocking ToEnumerable of AssertEqual)
// Not ported: EnumerableToObservable_LongRunning_Complete - real threads (TestLongRunningScheduler; the port has no ISchedulerLongRunning)
// Not ported: EnumerableToObservable_LongRunning_Dispose - real threads (TestLongRunningScheduler; the port has no ISchedulerLongRunning)
// Not ported: EnumerableToObservable_LongRunning_Error - real threads (TestLongRunningScheduler; the port has no ISchedulerLongRunning)
// Not ported: EnumerableToObservable_GetEnumeratorThrows - an enumerable whose GetEnumerator throws (`IntoIterator::into_iter` cannot fail)
