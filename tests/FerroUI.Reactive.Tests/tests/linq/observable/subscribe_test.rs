//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SubscribeTest.cs` of Rx.NET.
//! Adapted: `Observable.Subscribe(IEnumerable<T>, IObserver<T>, IScheduler)` is
//! `Observable::subscribe_enumerable_on`; `MockEnumerable<T>` (of the test helpers upstream) is
//! defined here, as an iterable whose iterator records the virtual time it was made at
//! (`GetEnumerator`) and dropped at (`Dispose`); the enumerable whose iteration throws of
//! `SubscribeToEnumerable_Error` is an iterable of `Result`s, subscribed through
//! `Observable::try_to_observable_on` (the producer `Subscribe` of an enumerable runs upstream); the
//! `Subscribe` overloads of `ObservableExtensions` are the `subscribe_*` methods of `ObservableExt`
//! (`subscribe_ignore`, `subscribe_next`, `subscribe_next_completed`); an unhandled error panics,
//! checked with `assert_throws` (which cannot check that the panic carries the error of the
//! sequence, as `ReactiveAssert.Throws(ex, ...)` does).

use ferroui_reactive::concurrency::Scheduler;
use ferroui_reactive::{IDisposable, IObservable, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, assert_throws, new_error, ReactiveTest as RT, Subscription, TestScheduler};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Slot<T> = Rc<RefCell<Option<T>>>;

fn slot<T>() -> Slot<T> {
    Rc::new(RefCell::new(None))
}

fn get<T: Clone>(slot: &Slot<T>) -> T {
    slot.borrow().clone().expect("the variable has been assigned")
}

fn set<T>(slot: &Slot<T>, value: T) {
    *slot.borrow_mut() = Some(value);
}

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
fn subscribe_to_enumerable_finite() {
    let scheduler = TestScheduler::new();

    let results = scheduler.create_observer::<i32>();
    let d: Slot<Rc<dyn IDisposable>> = slot();
    let xs: Slot<MockEnumerable<Vec<i32>>> = slot();

    {
        let (s, xs) = (scheduler.clone(), xs.clone());
        scheduler.schedule_absolute_action(RT::CREATED, move || set(&xs, MockEnumerable::new(&s, enumerable_finite())));
    }
    {
        let (s, xs, d, results) = (scheduler.scheduler(), xs.clone(), d.clone(), results.clone());
        scheduler.schedule_absolute_action(RT::SUBSCRIBED, move || set(&d, Observable::subscribe_enumerable_on(get(&xs), results, s)));
    }
    {
        let d = d.clone();
        scheduler.schedule_absolute_action(RT::DISPOSED, move || get(&d).dispose());
    }

    scheduler.run();

    assert_messages!(
        results.messages(),
        RT::on_next(201, 1),
        RT::on_next(202, 2),
        RT::on_next(203, 3),
        RT::on_next(204, 4),
        RT::on_next(205, 5),
        RT::on_completed::<i32>(206)
    );

    assert_subscriptions!(get(&xs).subscriptions(), RT::subscribe(200, 206));
}

#[test]
fn subscribe_to_enumerable_infinite() {
    let scheduler = TestScheduler::new();

    let results = scheduler.create_observer::<i32>();
    let d: Slot<Rc<dyn IDisposable>> = slot();
    let xs: Slot<MockEnumerable<std::iter::Repeat<i32>>> = slot();

    {
        let (s, xs) = (scheduler.clone(), xs.clone());
        scheduler.schedule_absolute_action(RT::CREATED, move || set(&xs, MockEnumerable::new(&s, enumerable_infinite())));
    }
    {
        let (s, xs, d, results) = (scheduler.scheduler(), xs.clone(), d.clone(), results.clone());
        scheduler.schedule_absolute_action(RT::SUBSCRIBED, move || set(&d, Observable::subscribe_enumerable_on(get(&xs), results, s)));
    }
    {
        let d = d.clone();
        scheduler.schedule_absolute_action(210, move || get(&d).dispose());
    }

    scheduler.run();

    assert_messages!(
        results.messages(),
        RT::on_next(201, 1),
        RT::on_next(202, 1),
        RT::on_next(203, 1),
        RT::on_next(204, 1),
        RT::on_next(205, 1),
        RT::on_next(206, 1),
        RT::on_next(207, 1),
        RT::on_next(208, 1),
        RT::on_next(209, 1)
    );

    assert_subscriptions!(get(&xs).subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn subscribe_to_enumerable_error() {
    let scheduler = TestScheduler::new();

    let results = scheduler.create_observer::<i32>();
    let d: Slot<Rc<dyn IDisposable>> = slot();
    let xs: Slot<MockEnumerable<Vec<Result<i32, ObservableError>>>> = slot();
    let ex = new_error("ex");

    {
        let (s, xs, ex) = (scheduler.clone(), xs.clone(), ex.clone());
        scheduler.schedule_absolute_action(RT::CREATED, move || set(&xs, MockEnumerable::new(&s, enumerable_error(ex))));
    }
    {
        let (s, xs, d, results) = (scheduler.scheduler(), xs.clone(), d.clone(), results.clone());
        scheduler.schedule_absolute_action(RT::SUBSCRIBED, move || set(&d, Observable::try_to_observable_on(get(&xs), s).subscribe(results)));
    }
    {
        let d = d.clone();
        scheduler.schedule_absolute_action(RT::DISPOSED, move || get(&d).dispose());
    }

    scheduler.run();

    assert_messages!(
        results.messages(),
        RT::on_next(201, 1),
        RT::on_next(202, 2),
        RT::on_next(203, 3),
        RT::on_error::<i32>(204, ex)
    );

    assert_subscriptions!(get(&xs).subscriptions(), RT::subscribe(200, 204));
}

fn enumerable_finite() -> Vec<i32> {
    vec![1, 2, 3, 4, 5]
}

fn enumerable_infinite() -> std::iter::Repeat<i32> {
    std::iter::repeat(1)
}

fn enumerable_error(exception: ObservableError) -> Vec<Result<i32, ObservableError>> {
    vec![Ok(1), Ok(2), Ok(3), Err(exception)]
}

// Subscribe

#[test]
fn subscribe_none_return() {
    Observable::return_on(1, Scheduler::immediate()).subscribe_ignore();
}

#[test]
fn subscribe_none_throw() {
    let ex = new_error("ex");

    let xs = Observable::throw_on::<i32>(ex, Scheduler::immediate());

    assert_throws(|| {
        xs.subscribe_ignore();
    });
}

#[test]
fn subscribe_none_empty() {
    Observable::empty_on::<i32>(Scheduler::immediate()).subscribe_next(|_| panic!("unexpected value"));
}

#[test]
fn subscribe_on_next_return() {
    let x = Rc::new(Cell::new(-1));
    let target = x.clone();
    Observable::return_on(42, Scheduler::immediate()).subscribe_next(move |value| target.set(value));
    assert_eq!(42, x.get());
}

#[test]
fn subscribe_on_next_throw() {
    let ex = new_error("ex");

    let xs = Observable::throw_on::<i32>(ex, Scheduler::immediate());

    assert_throws(|| {
        xs.subscribe_next(|_| panic!("unexpected value"));
    });
}

#[test]
fn subscribe_on_next_empty() {
    Observable::empty_on::<i32>(Scheduler::immediate()).subscribe_next(|_| panic!("unexpected value"));
}

#[test]
fn subscribe_on_next_on_completed_return() {
    let finished = Rc::new(Cell::new(false));
    let x = Rc::new(Cell::new(-1));
    let (target, done) = (x.clone(), finished.clone());
    Observable::return_on(42, Scheduler::immediate()).subscribe_next_completed(move |value| target.set(value), move || done.set(true));
    assert_eq!(42, x.get());
    assert!(finished.get());
}

#[test]
fn subscribe_on_next_on_completed_throw() {
    let ex = new_error("ex");

    let xs: Rc<dyn IObservable<i32>> = Observable::throw_on(ex, Scheduler::immediate());

    assert_throws(|| {
        xs.subscribe_next_completed(|_| panic!("unexpected value"), || panic!("unexpected completion"));
    });
}

#[test]
fn subscribe_on_next_on_completed_empty() {
    let finished = Rc::new(Cell::new(false));
    let done = finished.clone();
    Observable::empty_on::<i32>(Scheduler::immediate()).subscribe_next_completed(|_| panic!("unexpected value"), move || done.set(true));
    assert!(finished.get());
}

// Not ported: SubscribeToEnumerable_ArgumentChecking - argument validation (null arguments, an enumerable whose GetEnumerator returns null)
// Not ported: SubscribeToEnumerable_DefaultScheduler - real threads (Semaphore, DefaultScheduler.Instance)
// Not ported: Subscribe_ArgumentChecking - argument validation
// Not ported: Subscribe_CT_ArgumentChecking - needs the Subscribe overloads with a CancellationToken (not in ObservableExt)
// Not ported: Subscribe_CT_None - needs the Subscribe overloads with a CancellationToken (not in ObservableExt)
// Not ported: Subscribe_CT_CancelBeforeBegin - needs the Subscribe overloads with a CancellationToken (not in ObservableExt)
// Not ported: Subscribe_CT_CancelMiddle - needs the Subscribe overloads with a CancellationToken (not in ObservableExt)
// Not ported: Subscribe_CT_CancelAfterEnd - needs the Subscribe overloads with a CancellationToken (not in ObservableExt)
// Not ported: Subscribe_CT_NeverCancel - needs the Subscribe overloads with a CancellationToken (not in ObservableExt)
// Not ported: Subscribe_CT_Overloads_AlreadyCancelled - needs the Subscribe overloads with a CancellationToken (not in ObservableExt)
// Not ported: Subscribe_CT_Overloads_None - needs the Subscribe overloads with a CancellationToken (not in ObservableExt)
// Not ported: Subscribe_CT_CancelDuringCallback - needs the Subscribe overloads with a CancellationToken (not in ObservableExt)
