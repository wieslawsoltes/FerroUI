//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SelectManyTest.cs` of Rx.NET: the forms
//! on enumerables (the forms on sequences are in `select_many_test`).
//! Adapted: the forms on enumerables are `select_many_iter` (`select_many_iter_with` with a result
//! selector, `_with_index` forms with indexes); an enumerable whose selector, `GetEnumerator`,
//! `MoveNext` or `Current` throws, or a result selector that throws, is the `try_` form with an
//! iterable of `Result`s (`RogueEnumerable`, whose `GetEnumerator` throws, is a selector returning
//! the error); `MockEnumerable` (of the upstream tests) is defined here, as an iterable whose
//! iterator records the virtual time it was made at (`GetEnumerator`) and dropped at (`Dispose`).
//! The upstream tests of the forms without and with an index (`SelectMany_Enumerable_X` and
//! `SelectManyWithIndex_Enumerable_X`) share their scenarios: each scenario is a function run by
//! both tests, with the form chosen by `indexed`. The anonymous values are tuples.

use ferroui_reactive::{IObservable, Notification, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, HotObservable, ReactiveTest as RT, Recorded, Subscription, TestScheduler};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `Enumerable.Repeat(x, x)`.
type Repeated = std::iter::Take<std::iter::Repeat<i32>>;

fn repeat(x: i32) -> Repeated {
    std::iter::repeat(x).take(x as usize)
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

/// `CurrentThrowsEnumerable<T>`: its enumerator moves as the underlying one does, and its
/// `Current` throws.
struct CurrentThrows<I> {
    inner: I,
    ex: ObservableError,
}

impl<I: Iterator> Iterator for CurrentThrows<I> {
    type Item = Result<I::Item, ObservableError>;

    fn next(&mut self) -> Option<Self::Item> {
        let ex = self.ex.clone();
        self.inner.next().map(move |_| Err(ex))
    }
}

/// `MoveNextThrowsEnumerable<T>`: its `MoveNext` throws.
struct MoveNextThrows<I> {
    #[allow(dead_code)]
    inner: I,
    ex: ObservableError,
}

impl<I: Iterator> Iterator for MoveNextThrows<I> {
    type Item = Result<I::Item, ObservableError>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(Err(self.ex.clone()))
    }
}

/// `RogueEnumerable<int>`: its `GetEnumerator` throws.
fn rogue(ex: &ObservableError) -> Result<std::iter::Empty<Result<i32, ObservableError>>, ObservableError> {
    Err(ex.clone())
}

/// `SelectMany(selector)` to enumerables, or `SelectMany((x, _) => selector(x))` when `indexed`.
fn many_iter<I>(xs: Rc<dyn IObservable<i32>>, indexed: bool, selector: impl Fn(i32) -> I + 'static) -> Rc<dyn IObservable<i32>>
where
    I: IntoIterator<Item = i32>,
    I::IntoIter: 'static,
{
    if indexed {
        xs.select_many_iter_with_index(move |x, _| selector(x))
    } else {
        xs.select_many_iter(selector)
    }
}

/// `SelectMany(collectionSelector, resultSelector)` to enumerables, or `SelectMany((x, _) =>
/// collectionSelector(x), (x, _, y, __) => resultSelector(x, y))` when `indexed`.
fn many_iter_with<I>(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    collection_selector: impl Fn(i32) -> I + 'static,
    result_selector: impl Fn(i32, i32) -> i32 + 'static,
) -> Rc<dyn IObservable<i32>>
where
    I: IntoIterator<Item = i32>,
    I::IntoIter: 'static,
{
    if indexed {
        xs.select_many_iter_with_index_with(move |x, _| collection_selector(x), move |x, _, y, _| result_selector(x, y))
    } else {
        xs.select_many_iter_with(collection_selector, result_selector)
    }
}

/// [`many_iter`] of an enumerable that may fail.
fn try_many_iter<I>(xs: Rc<dyn IObservable<i32>>, indexed: bool, selector: impl Fn(i32) -> Result<I, ObservableError> + 'static) -> Rc<dyn IObservable<i32>>
where
    I: IntoIterator<Item = Result<i32, ObservableError>>,
    I::IntoIter: 'static,
{
    if indexed {
        xs.try_select_many_iter_with_index(move |x, _| selector(x))
    } else {
        xs.try_select_many_iter(selector)
    }
}

/// [`many_iter_with`] of an enumerable and a result selector that may fail.
fn try_many_iter_with<I>(
    xs: Rc<dyn IObservable<i32>>,
    indexed: bool,
    collection_selector: impl Fn(i32) -> Result<I, ObservableError> + 'static,
    result_selector: impl Fn(i32, i32) -> Result<i32, ObservableError> + 'static,
) -> Rc<dyn IObservable<i32>>
where
    I: IntoIterator<Item = Result<i32, ObservableError>>,
    I::IntoIter: 'static,
{
    if indexed {
        xs.try_select_many_iter_with_index_with(move |x, _| collection_selector(x), move |x, _, y, _| result_selector(x, y))
    } else {
        xs.try_select_many_iter_with(collection_selector, result_selector)
    }
}

/// The source of the scenarios, ending with `end`.
fn source(scheduler: &TestScheduler, end: Recorded<Notification<i32>>) -> Rc<HotObservable<i32>> {
    scheduler.create_hot_observable(vec![RT::on_next(210, 2), RT::on_next(340, 4), RT::on_next(420, 3), RT::on_next(510, 2), end])
}

/// `SelectMany_Enumerable_Complete` and `SelectManyWithIndex_Enumerable_Complete`.
fn select_many_enumerable_complete_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let inners: Rc<RefCell<Vec<MockEnumerable<Repeated>>>> = Rc::new(RefCell::new(Vec::new()));

    let res = scheduler.start({
        let (source, s, inners) = (xs.as_observable(), scheduler.clone(), inners.clone());
        move || {
            many_iter(source, indexed, move |x| {
                let ys = MockEnumerable::new(&s, repeat(x));
                inners.borrow_mut().push(ys.clone());
                ys
            })
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(210, 2),
        RT::on_next(340, 4),
        RT::on_next(340, 4),
        RT::on_next(340, 4),
        RT::on_next(340, 4),
        RT::on_next(420, 3),
        RT::on_next(420, 3),
        RT::on_next(420, 3),
        RT::on_next(510, 2),
        RT::on_next(510, 2),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));

    let inners = inners.borrow();

    assert_eq!(4, inners.len());

    assert_subscriptions!(inners[0].subscriptions(), RT::subscribe(210, 210));

    assert_subscriptions!(inners[1].subscriptions(), RT::subscribe(340, 340));

    assert_subscriptions!(inners[2].subscriptions(), RT::subscribe(420, 420));

    assert_subscriptions!(inners[3].subscriptions(), RT::subscribe(510, 510));
}

/// `SelectMany_Enumerable_Complete_ResultSelector` and `SelectManyWithIndex_Enumerable_Complete_ResultSelector`.
fn select_many_enumerable_complete_result_selector_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let source = xs.as_observable();
    let res = scheduler.start(move || many_iter_with(source, indexed, repeat, |x, y| x + y));

    assert_messages!(
        res.messages(),
        RT::on_next(210, 4),
        RT::on_next(210, 4),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_next(420, 6),
        RT::on_next(420, 6),
        RT::on_next(420, 6),
        RT::on_next(510, 4),
        RT::on_next(510, 4),
        RT::on_completed(600)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

/// `SelectMany_Enumerable_Error` and `SelectManyWithIndex_Enumerable_Error`.
fn select_many_enumerable_error_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = source(&scheduler, RT::on_error(600, ex.clone()));

    let source = xs.as_observable();
    let res = scheduler.start(move || many_iter(source, indexed, repeat));

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(210, 2),
        RT::on_next(340, 4),
        RT::on_next(340, 4),
        RT::on_next(340, 4),
        RT::on_next(340, 4),
        RT::on_next(420, 3),
        RT::on_next(420, 3),
        RT::on_next(420, 3),
        RT::on_next(510, 2),
        RT::on_next(510, 2),
        RT::on_error(600, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

/// `SelectMany_Enumerable_Error_ResultSelector` and `SelectManyWithIndex_Enumerable_Error_ResultSelector`.
fn select_many_enumerable_error_result_selector_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = source(&scheduler, RT::on_error(600, ex.clone()));

    let source = xs.as_observable();
    let res = scheduler.start(move || many_iter_with(source, indexed, repeat, |x, y| x + y));

    assert_messages!(
        res.messages(),
        RT::on_next(210, 4),
        RT::on_next(210, 4),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_next(420, 6),
        RT::on_next(420, 6),
        RT::on_next(420, 6),
        RT::on_next(510, 4),
        RT::on_next(510, 4),
        RT::on_error(600, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 600));
}

/// `SelectMany_Enumerable_Dispose` and `SelectManyWithIndex_Enumerable_Dispose`.
fn select_many_enumerable_dispose_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let source = xs.as_observable();
    let res = scheduler.start_disposed(move || many_iter(source, indexed, repeat), 350);

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(210, 2),
        RT::on_next(340, 4),
        RT::on_next(340, 4),
        RT::on_next(340, 4),
        RT::on_next(340, 4)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 350));
}

/// `SelectMany_Enumerable_Dispose_ResultSelector` and `SelectManyWithIndex_Enumerable_Dispose_ResultSelector`.
fn select_many_enumerable_dispose_result_selector_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let source = xs.as_observable();
    let res = scheduler.start_disposed(move || many_iter_with(source, indexed, repeat, |x, y| x + y), 350);

    assert_messages!(
        res.messages(),
        RT::on_next(210, 4),
        RT::on_next(210, 4),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_next(340, 8)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 350));
}

/// `SelectMany_Enumerable_SelectorThrows` and `SelectManyWithIndex_Enumerable_SelectorThrows`.
fn select_many_enumerable_selector_throws_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let invoked = Rc::new(Cell::new(0));
    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, invoked, ex) = (xs.as_observable(), invoked.clone(), ex.clone());
        move || {
            try_many_iter(source, indexed, move |x| {
                invoked.set(invoked.get() + 1);
                if invoked.get() == 3 {
                    return Err(ex.clone());
                }

                Ok(repeat(x).map(Ok::<i32, ObservableError>))
            })
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(210, 2),
        RT::on_next(340, 4),
        RT::on_next(340, 4),
        RT::on_next(340, 4),
        RT::on_next(340, 4),
        RT::on_error(420, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));

    assert_eq!(3, invoked.get());
}

/// `SelectMany_Enumerable_ResultSelectorThrows` and `SelectManyWithIndex_Enumerable_ResultSelectorThrows`.
fn select_many_enumerable_result_selector_throws_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let ex = new_error("ex");

    let inners: Rc<RefCell<Vec<MockEnumerable<Repeated>>>> = Rc::new(RefCell::new(Vec::new()));

    let res = scheduler.start({
        let (source, s, inners, ex) = (xs.as_observable(), scheduler.clone(), inners.clone(), ex.clone());
        move || {
            try_many_iter_with(
                source,
                indexed,
                move |x| {
                    let ys = MockEnumerable::new(&s, repeat(x));
                    inners.borrow_mut().push(ys.clone());
                    Ok(ys.into_iter().map(Ok::<i32, ObservableError>))
                },
                move |x, y| {
                    if x == 3 {
                        return Err(ex.clone());
                    }

                    Ok(x + y)
                },
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 4),
        RT::on_next(210, 4),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_error(420, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));

    let inners = inners.borrow();

    assert_eq!(3, inners.len());

    assert_subscriptions!(inners[0].subscriptions(), RT::subscribe(210, 210));

    assert_subscriptions!(inners[1].subscriptions(), RT::subscribe(340, 340));

    assert_subscriptions!(inners[2].subscriptions(), RT::subscribe(420, 420));
}

/// `SelectMany_Enumerable_ResultSelector_GetEnumeratorThrows` and
/// `SelectManyWithIndex_Enumerable_ResultSelector_GetEnumeratorThrows`.
fn select_many_enumerable_result_selector_get_enumerator_throws_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, ex) = (xs.as_observable(), ex.clone());
        move || try_many_iter_with(source, indexed, move |_| rogue(&ex), |x, y| Ok(x + y))
    });

    assert_messages!(res.messages(), RT::on_error(210, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

/// `SelectMany_Enumerable_SelectorThrows_ResultSelector` and
/// `SelectManyWithIndex_Enumerable_SelectorThrows_ResultSelector`.
fn select_many_enumerable_selector_throws_result_selector_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let invoked = Rc::new(Cell::new(0));
    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, invoked, ex) = (xs.as_observable(), invoked.clone(), ex.clone());
        move || {
            try_many_iter_with(
                source,
                indexed,
                move |x| {
                    invoked.set(invoked.get() + 1);
                    if invoked.get() == 3 {
                        return Err(ex.clone());
                    }

                    Ok(repeat(x).map(Ok::<i32, ObservableError>))
                },
                |x, y| Ok(x + y),
            )
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(210, 4),
        RT::on_next(210, 4),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_next(340, 8),
        RT::on_error(420, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));

    assert_eq!(3, invoked.get());
}

/// `SelectMany_Enumerable_CurrentThrows` and `SelectManyWithIndex_Enumerable_CurrentThrows`.
fn select_many_enumerable_current_throws_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, ex) = (xs.as_observable(), ex.clone());
        move || try_many_iter(source, indexed, move |x| Ok(CurrentThrows { inner: repeat(x), ex: ex.clone() }))
    });

    assert_messages!(res.messages(), RT::on_error(210, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

/// `SelectMany_Enumerable_CurrentThrows_ResultSelector` and
/// `SelectManyWithIndex_Enumerable_CurrentThrows_ResultSelector`.
fn select_many_enumerable_current_throws_result_selector_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, ex) = (xs.as_observable(), ex.clone());
        move || try_many_iter_with(source, indexed, move |x| Ok(CurrentThrows { inner: repeat(x), ex: ex.clone() }), |x, y| Ok(x + y))
    });

    assert_messages!(res.messages(), RT::on_error(210, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

/// `SelectMany_Enumerable_GetEnumeratorThrows` and `SelectManyWithIndex_Enumerable_GetEnumeratorThrows`.
fn select_many_enumerable_get_enumerator_throws_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, ex) = (xs.as_observable(), ex.clone());
        move || try_many_iter(source, indexed, move |_| rogue(&ex))
    });

    assert_messages!(res.messages(), RT::on_error(210, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

/// `SelectMany_Enumerable_MoveNextThrows` and `SelectManyWithIndex_Enumerable_MoveNextThrows`.
fn select_many_enumerable_move_next_throws_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, ex) = (xs.as_observable(), ex.clone());
        move || try_many_iter(source, indexed, move |x| Ok(MoveNextThrows { inner: repeat(x), ex: ex.clone() }))
    });

    assert_messages!(res.messages(), RT::on_error(210, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

/// `SelectMany_Enumerable_MoveNextThrows_ResultSelector` and
/// `SelectManyWithIndex_Enumerable_MoveNextThrows_ResultSelector`.
fn select_many_enumerable_move_next_throws_result_selector_case(indexed: bool) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(600));

    let ex = new_error("ex");

    let res = scheduler.start({
        let (source, ex) = (xs.as_observable(), ex.clone());
        move || try_many_iter_with(source, indexed, move |x| Ok(MoveNextThrows { inner: repeat(x), ex: ex.clone() }), |x, y| Ok(x + y))
    });

    assert_messages!(res.messages(), RT::on_error(210, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

#[test]
fn select_many_enumerable_complete() {
    select_many_enumerable_complete_case(false);
}

#[test]
fn select_many_enumerable_complete_result_selector() {
    select_many_enumerable_complete_result_selector_case(false);
}

#[test]
fn select_many_enumerable_error() {
    select_many_enumerable_error_case(false);
}

#[test]
fn select_many_enumerable_error_result_selector() {
    select_many_enumerable_error_result_selector_case(false);
}

#[test]
fn select_many_enumerable_dispose() {
    select_many_enumerable_dispose_case(false);
}

#[test]
fn select_many_enumerable_dispose_result_selector() {
    select_many_enumerable_dispose_result_selector_case(false);
}

#[test]
fn select_many_enumerable_selector_throws() {
    select_many_enumerable_selector_throws_case(false);
}

#[test]
fn select_many_enumerable_result_selector_throws() {
    select_many_enumerable_result_selector_throws_case(false);
}

#[test]
fn select_many_enumerable_result_selector_get_enumerator_throws() {
    select_many_enumerable_result_selector_get_enumerator_throws_case(false);
}

#[test]
fn select_many_enumerable_selector_throws_result_selector() {
    select_many_enumerable_selector_throws_result_selector_case(false);
}

#[test]
fn select_many_enumerable_current_throws() {
    select_many_enumerable_current_throws_case(false);
}

#[test]
fn select_many_enumerable_current_throws_result_selector() {
    select_many_enumerable_current_throws_result_selector_case(false);
}

#[test]
fn select_many_enumerable_get_enumerator_throws() {
    select_many_enumerable_get_enumerator_throws_case(false);
}

#[test]
fn select_many_enumerable_move_next_throws() {
    select_many_enumerable_move_next_throws_case(false);
}

#[test]
fn select_many_enumerable_move_next_throws_result_selector() {
    select_many_enumerable_move_next_throws_result_selector_case(false);
}

#[test]
fn select_many_with_index_enumerable_index() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 4), RT::on_next(220, 3), RT::on_next(250, 5), RT::on_next(270, 1), RT::on_completed(290)]);

    let source = xs.as_observable();
    let res = scheduler.start(move || source.select_many_iter_with_index(|x, i| vec![(x, i)]));

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
fn select_many_with_index_enumerable_result_selector_index() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, 4), RT::on_next(220, 3), RT::on_next(250, 5), RT::on_next(270, 1), RT::on_completed(290)]);

    let source = xs.as_observable();
    // `Enumerable.Range(10, i + 1)`.
    let res = scheduler.start(move || source.select_many_iter_with_index_with(|_, i| 10..(11 + i as i32), |x, i, y, j| (x, i, y, j)));

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
fn select_many_with_index_enumerable_complete() {
    select_many_enumerable_complete_case(true);
}

#[test]
fn select_many_with_index_enumerable_complete_result_selector() {
    select_many_enumerable_complete_result_selector_case(true);
}

#[test]
fn select_many_with_index_enumerable_error() {
    select_many_enumerable_error_case(true);
}

#[test]
fn select_many_with_index_enumerable_error_result_selector() {
    select_many_enumerable_error_result_selector_case(true);
}

#[test]
fn select_many_with_index_enumerable_dispose() {
    select_many_enumerable_dispose_case(true);
}

#[test]
fn select_many_with_index_enumerable_dispose_result_selector() {
    select_many_enumerable_dispose_result_selector_case(true);
}

#[test]
fn select_many_with_index_enumerable_selector_throws() {
    select_many_enumerable_selector_throws_case(true);
}

#[test]
fn select_many_with_index_enumerable_result_selector_throws() {
    select_many_enumerable_result_selector_throws_case(true);
}

#[test]
fn select_many_with_index_enumerable_result_selector_get_enumerator_throws() {
    select_many_enumerable_result_selector_get_enumerator_throws_case(true);
}

#[test]
fn select_many_with_index_enumerable_selector_throws_result_selector() {
    select_many_enumerable_selector_throws_result_selector_case(true);
}

#[test]
fn select_many_with_index_enumerable_current_throws() {
    select_many_enumerable_current_throws_case(true);
}

#[test]
fn select_many_with_index_enumerable_current_throws_result_selector() {
    select_many_enumerable_current_throws_result_selector_case(true);
}

#[test]
fn select_many_with_index_enumerable_get_enumerator_throws() {
    select_many_enumerable_get_enumerator_throws_case(true);
}

#[test]
fn select_many_with_index_enumerable_move_next_throws() {
    select_many_enumerable_move_next_throws_case(true);
}

#[test]
fn select_many_with_index_enumerable_move_next_throws_result_selector() {
    select_many_enumerable_move_next_throws_result_selector_case(true);
}

// Not ported: SelectMany_Enumerable_ArgumentChecking - argument validation
// Not ported: SelectManyWithIndex_Enumerable_ArgumentChecking - argument validation
