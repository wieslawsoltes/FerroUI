//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ConcatTest.cs` of Rx.NET.
//! Adapted: `Observable.Concat(params)` and `Concat` of an enumerable are `Observable::concat_all`;
//! the tests on the default scheduler wait for nothing (`Return` notifies on the immediate scheduler,
//! so the subscription has ended when `subscribe` returns, where upstream waits on an event);
//! `MockEnumerable<T>` of the test library is a local enumerable that records the times its
//! enumerators live (an enumerator is disposed when it is dropped).

use ferroui_reactive::{IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, Subscription, TestScheduler};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An enumerable whose enumerators record the virtual times they live (`MockEnumerable<T>`).
#[derive(Clone)]
struct MockEnumerable<T> {
    scheduler: TestScheduler,
    subscriptions: Rc<RefCell<Vec<Subscription>>>,
    items: Vec<T>,
}

impl<T: Clone> MockEnumerable<T> {
    fn new(scheduler: &TestScheduler, items: Vec<T>) -> Self {
        Self { scheduler: scheduler.clone(), subscriptions: Rc::new(RefCell::new(Vec::new())), items }
    }

    fn subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions.borrow().clone()
    }
}

impl<T: Clone> IntoIterator for MockEnumerable<T> {
    type Item = T;
    type IntoIter = MockEnumerator<T>;

    fn into_iter(self) -> MockEnumerator<T> {
        let index = {
            let mut subscriptions = self.subscriptions.borrow_mut();
            subscriptions.push(Subscription::starting(self.scheduler.clock()));
            subscriptions.len() - 1
        };
        MockEnumerator { scheduler: self.scheduler, subscriptions: self.subscriptions, index, items: self.items.into_iter() }
    }
}

/// An enumerator of a [`MockEnumerable`]; dropping it is its disposal.
struct MockEnumerator<T> {
    scheduler: TestScheduler,
    subscriptions: Rc<RefCell<Vec<Subscription>>>,
    index: usize,
    items: std::vec::IntoIter<T>,
}

impl<T> Iterator for MockEnumerator<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.items.next()
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
fn concat_default_scheduler() {
    let sum = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(false));

    let (s, c) = (sum.clone(), completed.clone());
    let _d = Observable::concat_all(vec![Observable::return_(1), Observable::return_(2), Observable::return_(3)])
        .subscribe_next_completed(move |n| s.set(s.get() + n), move || c.set(true));

    assert!(completed.get());

    assert_eq!(6, sum.get());
}

#[test]
fn concat_ie_of_io_default_scheduler() {
    let sources: Vec<Rc<dyn IObservable<i32>>> = vec![Observable::return_(1), Observable::return_(2), Observable::return_(3)];

    let sum = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(false));

    let (s, c) = (sum.clone(), completed.clone());
    let _d = Observable::concat_all(sources).subscribe_next_completed(move |n| s.set(s.get() + n), move || c.set(true));

    assert!(completed.get());

    assert_eq!(6, sum.get());
}

#[test]
fn concat_ie_of_io() {
    let scheduler = TestScheduler::new();

    let xs1 = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_completed(40)]);

    let xs2 = scheduler.create_cold_observable(vec![RT::on_next(10, 4), RT::on_next(20, 5), RT::on_completed(30)]);

    let xs3 = scheduler.create_cold_observable(vec![
        RT::on_next(10, 6),
        RT::on_next(20, 7),
        RT::on_next(30, 8),
        RT::on_next(40, 9),
        RT::on_completed(50),
    ]);

    let sources = vec![xs1.as_observable(), xs2.as_observable(), xs3.as_observable()];
    let res = scheduler.start(move || Observable::concat_all(sources));

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
fn concat_empty_empty() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages(), RT::on_completed::<i32>(250));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 250));
}

#[test]
fn concat_empty_never() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages());

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 1000));
}

#[test]
fn concat_never_empty() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages());

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(o2.subscriptions());
}

#[test]
fn concat_never_never() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages());

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(o2.subscriptions());
}

#[test]
fn concat_empty_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(250, ex.clone())]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages(), RT::on_error::<i32>(250, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 250));
}

#[test]
fn concat_throw_empty() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(230, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages(), RT::on_error::<i32>(230, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions());
}

#[test]
fn concat_throw_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(230, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(250, new_error("other"))]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages(), RT::on_error::<i32>(230, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions());
}

#[test]
fn concat_return_empty() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_completed(250));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 250));
}

#[test]
fn concat_empty_return() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 2), RT::on_completed(250)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages(), RT::on_next(240, 2), RT::on_completed(250));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 250));
}

#[test]
fn concat_return_never() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages(), RT::on_next(210, 2));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 1000));
}

#[test]
fn concat_never_return() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(230)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages());

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(o2.subscriptions());
}

#[test]
fn concat_return_return() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 3), RT::on_completed(250)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages(), RT::on_next(220, 2), RT::on_next(240, 3), RT::on_completed(250));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 250));
}

#[test]
fn concat_throw_return() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(230, ex.clone())]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 2), RT::on_completed(250)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages(), RT::on_error::<i32>(230, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions());
}

#[test]
fn concat_return_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_error(250, ex.clone())]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(res.messages(), RT::on_next(220, 2), RT::on_error(250, ex.clone()));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 250));
}

#[test]
fn concat_some_data_some_data() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_completed(225)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 4), RT::on_next(240, 5), RT::on_completed(250)]);

    let (a, b) = (o1.as_observable(), o2.as_observable());
    let res = scheduler.start(move || a.concat(b));

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(230, 4),
        RT::on_next(240, 5),
        RT::on_completed(250)
    );

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 225));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(225, 250));
}

#[test]
fn concat_enumerable_timing() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![
        RT::on_next(150, 1),
        RT::on_next(210, 2), // !
        RT::on_next(220, 3), // !
        RT::on_completed(230),
    ]);

    let o2 = scheduler.create_cold_observable(vec![
        RT::on_next(50, 4), // !
        RT::on_next(60, 5), // !
        RT::on_next(70, 6), // !
        RT::on_completed(80),
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

    let xss = MockEnumerable::new(&scheduler, vec![o1.as_observable(), o2.as_observable(), o3.as_observable(), o2.as_observable()]);

    let sources = xss.clone();
    let res = scheduler.start(move || Observable::concat_all(sources));

    assert_messages!(
        res.messages(),
        RT::on_next(210, 2),
        RT::on_next(220, 3),
        RT::on_next(280, 4),
        RT::on_next(290, 5),
        RT::on_next(300, 6),
        RT::on_next(320, 7),
        RT::on_next(330, 8),
        RT::on_next(390, 4),
        RT::on_next(400, 5),
        RT::on_next(410, 6),
        RT::on_completed(420)
    );

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 310), RT::subscribe(340, 420));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(310, 340));

    assert_subscriptions!(xss.subscriptions(), RT::subscribe(200, 420));
}

#[test]
fn concat_enumerable_dispose() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_completed(230)]);

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

    let xss = MockEnumerable::new(&scheduler, vec![o1.as_observable(), o2.as_observable()]);

    let sources = xss.clone();
    let res = scheduler.start_disposed(move || Observable::concat_all(sources), 300);

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_next(240, 4), RT::on_next(270, 5));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 300));

    assert_subscriptions!(xss.subscriptions(), RT::subscribe(200, 300));
}

// Not ported: Concat_ArgumentChecking - argument validation
// Not ported: Concat_IEofIO_GetEnumeratorThrows - an enumerable whose GetEnumerator throws: a Rust iterator cannot fail to be created
// Not ported: Concat_EnumerableThrows - an enumerable that throws while it is enumerated: a Rust iterator cannot fail
// Not ported: Concat_Optimization_DeferEvalTiming - needs operator Defer (phase 3, later)
// Not ported: Concat_Optimization_DeferExceptionPropagation - needs operator Defer (phase 3, later)
// Not ported: Concat_TailRecursive1 - needs operator Defer (phase 3, later)
// Not ported: Concat_TailRecursive2 - real threads (ThreadPoolScheduler, stack traces); needs operators Defer, Take, ForEach (phase 3, later)
// Not ported: Concat_Task - needs async/await (phase 4)
