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
//! sequence, as `ReactiveAssert.Throws(ex, ...)` does); the overloads with a `CancellationToken`
//! are the `*_with_token` methods (`subscribe_with_token`, `subscribe_ignore_with_token`,
//! `subscribe_next_with_token`, ...), and `CancellationToken.None` is `CancellationToken::none()`.

use ferroui_reactive::concurrency::Scheduler;
use ferroui_reactive::{IDisposable, IObservable, Observable, ObservableError, ObservableExt, Observer};
use ferroui_system::threading::{CancellationToken, CancellationTokenSource};
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

#[test]
fn subscribe_ct_none() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3), RT::on_completed(240)]);

    let obs = scheduler.create_observer::<i32>();

    {
        let (source, observer) = (xs.as_observable(), obs.clone());
        scheduler.schedule_absolute_action(200, move || source.subscribe_with_token(observer, CancellationToken::none()));
    }

    scheduler.run();

    assert_messages!(obs.messages(), RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3), RT::on_completed(240));

    // No auto-dispose when using a hot observable.
    assert_subscriptions!(xs.subscriptions(), RT::subscribe_from(200));
}

#[test]
fn subscribe_ct_cancel_before_begin() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3), RT::on_completed(240)]);

    let cts = CancellationTokenSource::new();

    let obs = scheduler.create_observer::<i32>();

    {
        let (source, observer, token) = (xs.as_observable(), obs.clone(), cts.token());
        scheduler.schedule_absolute_action(200, move || source.subscribe_with_token(observer, token));
    }
    {
        let cts = cts.clone();
        scheduler.schedule_absolute_action(150, move || cts.cancel());
    }

    scheduler.run();

    assert_messages!(obs.messages());

    assert_subscriptions!(xs.subscriptions());
}

#[test]
fn subscribe_ct_cancel_middle() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3), RT::on_completed(240)]);

    let cts = CancellationTokenSource::new();

    let obs = scheduler.create_observer::<i32>();

    {
        let (source, observer, token) = (xs.as_observable(), obs.clone(), cts.token());
        scheduler.schedule_absolute_action(200, move || source.subscribe_with_token(observer, token));
    }
    {
        let cts = cts.clone();
        scheduler.schedule_absolute_action(225, move || cts.cancel());
    }

    scheduler.run();

    assert_messages!(obs.messages(), RT::on_next(210, 1), RT::on_next(220, 2));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 225));
}

#[test]
fn subscribe_ct_cancel_after_end() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3), RT::on_error(240, ex.clone())]);

    let cts = CancellationTokenSource::new();

    let obs = scheduler.create_observer::<i32>();

    {
        let (source, observer, token) = (xs.as_observable(), obs.clone(), cts.token());
        scheduler.schedule_absolute_action(200, move || source.subscribe_with_token(observer, token));
    }
    {
        let cts = cts.clone();
        scheduler.schedule_absolute_action(250, move || cts.cancel());
    }

    scheduler.run();

    assert_messages!(obs.messages(), RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3), RT::on_error(240, ex));

    // No auto-dispose when using a hot observable.
    assert_subscriptions!(xs.subscriptions(), RT::subscribe_from(200));
}

#[test]
fn subscribe_ct_never_cancel() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3), RT::on_completed(240)]);

    let cts = CancellationTokenSource::new();

    let obs = scheduler.create_observer::<i32>();

    {
        let (source, observer, token) = (xs.as_observable(), obs.clone(), cts.token());
        scheduler.schedule_absolute_action(200, move || source.subscribe_with_token(observer, token));
    }

    scheduler.run();

    assert_messages!(obs.messages(), RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3), RT::on_completed(240));

    // No auto-dispose when using a hot observable.
    assert_subscriptions!(xs.subscriptions(), RT::subscribe_from(200));
}

#[test]
fn subscribe_ct_overloads_already_cancelled() {
    let xs: Rc<dyn IObservable<i32>> = Observable::defer(|| -> Rc<dyn IObservable<i32>> {
        panic!("the factory of the deferred sequence ran");
    });

    let cts = CancellationTokenSource::new();
    cts.cancel();

    xs.subscribe_ignore_with_token(cts.token());
    xs.subscribe_next_with_token(|_| {}, cts.token());
    xs.subscribe_next_error_with_token(|_| {}, |_| {}, cts.token());
    xs.subscribe_next_completed_with_token(|_| {}, || {}, cts.token());
    xs.subscribe_all_with_token(|_| {}, |_| {}, || {}, cts.token());
    xs.subscribe_with_token(Observer::create_all(|_: i32| {}, |_| {}, || {}), cts.token());
}

#[test]
fn subscribe_ct_overloads_none() {
    let i = Rc::new(Cell::new(0));
    let n = Rc::new(Cell::new(0));
    let e = Rc::new(Cell::new(0));
    let c = Rc::new(Cell::new(0));

    let xs: Rc<dyn IObservable<i32>> = {
        let i = i.clone();
        Observable::defer(move || {
            i.set(i.get() + 1);
            Observable::return_on(42, Scheduler::immediate())
        })
    };

    let incr = |counter: &Rc<Cell<i32>>| {
        let counter = counter.clone();
        move || counter.set(counter.get() + 1)
    };

    xs.subscribe_ignore_with_token(CancellationToken::none());
    {
        let on_next = incr(&n);
        xs.subscribe_next_with_token(move |_| on_next(), CancellationToken::none());
    }
    {
        let (on_next, on_error) = (incr(&n), incr(&e));
        xs.subscribe_next_error_with_token(move |_| on_next(), move |_| on_error(), CancellationToken::none());
    }
    {
        let (on_next, on_completed) = (incr(&n), incr(&c));
        xs.subscribe_next_completed_with_token(move |_| on_next(), on_completed, CancellationToken::none());
    }
    {
        let (on_next, on_error, on_completed) = (incr(&n), incr(&e), incr(&c));
        xs.subscribe_all_with_token(move |_| on_next(), move |_| on_error(), on_completed, CancellationToken::none());
    }
    {
        let (on_next, on_error, on_completed) = (incr(&n), incr(&e), incr(&c));
        xs.subscribe_with_token(Observer::create_all(move |_: i32| on_next(), move |_| on_error(), on_completed), CancellationToken::none());
    }

    assert_eq!(6, i.get());
    assert_eq!(5, n.get());
    assert_eq!(0, e.get());
    assert_eq!(3, c.get());
}

#[test]
fn subscribe_ct_cancel_during_callback() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 1), RT::on_next(220, 2), RT::on_next(230, 3), RT::on_completed(240)]);

    let cts = CancellationTokenSource::new();

    let n = Rc::new(Cell::new(0));

    {
        let (source, token, cts, n) = (xs.as_observable(), cts.token(), cts.clone(), n.clone());
        scheduler.schedule_absolute_action(200, move || {
            source.subscribe_next_with_token(
                move |x| {
                    n.set(n.get() + 1);

                    if x == 2 {
                        cts.cancel();
                    }
                },
                token,
            );
        });
    }

    scheduler.run();

    assert_eq!(2, n.get());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 220));
}

// Not ported: SubscribeToEnumerable_ArgumentChecking - argument validation (null arguments, an enumerable whose GetEnumerator returns null)
// Not ported: SubscribeToEnumerable_DefaultScheduler - real threads (Semaphore, DefaultScheduler.Instance)
// Not ported: Subscribe_ArgumentChecking - argument validation
// Not ported: Subscribe_CT_ArgumentChecking - argument validation
