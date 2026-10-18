//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/OnErrorResumeNextTest.cs` of Rx.NET.
//! Adapted: `OnErrorResumeNext(IEnumerable<IObservable<T>>)` and `OnErrorResumeNext(params
//! IObservable<T>[])` are `Observable::on_error_resume_next_all`, and
//! `Observable::try_on_error_resume_next_all` when the enumeration throws (`RogueEnumerable`, an
//! iterator that throws); `MockEnumerable` (of the upstream tests) is ported below, its enumerator
//! recording its disposal when it is dropped; the `DefaultScheduler` tests run without the event
//! they wait on (the sequences are synchronous).

use ferroui_reactive::{IObservable, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{
    assert_messages, assert_recorded_eq, assert_subscriptions, new_error, Expected, ReactiveTest as RT, Recorded, Subscription, TestScheduler,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Source = Rc<dyn IObservable<i32>>;
type TrySource = Result<Source, ObservableError>;

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
fn on_error_resume_next_ie_of_io_get_enumerator_throws() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();

    // `RogueEnumerable`: `GetEnumerator` throws.
    let res = scheduler.start({
        let ex = ex.clone();
        move || Observable::try_on_error_resume_next_all(move || Err::<Vec<TrySource>, _>(ex.clone()))
    });

    assert_messages!(res.messages(), RT::on_error(200, ex.clone()));
}

#[test]
fn on_error_resume_next_ie_of_io() {
    let scheduler = TestScheduler::new();

    let xs1 = scheduler.create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_completed(40)]);

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
        move || Observable::on_error_resume_next_all(sources)
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
fn on_error_resume_next_no_errors() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_completed(230)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_completed(250)]);

    let res = scheduler.start({
        let (o1, o2) = (o1.clone(), o2.clone());
        move || o1.as_observable().on_error_resume_next(o2.as_observable())
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_next(240, 4), RT::on_completed(250));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 250));
}

#[test]
fn on_error_resume_next_error() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_error(230, new_error("ex"))]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(240, 4), RT::on_completed(250)]);

    let res = scheduler.start({
        let (o1, o2) = (o1.clone(), o2.clone());
        move || o1.as_observable().on_error_resume_next(o2.as_observable())
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_next(240, 4), RT::on_completed(250));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 250));
}

#[test]
fn on_error_resume_next_error_multiple() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_error(220, new_error("ex"))]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(230, 3), RT::on_error(240, new_error("ex"))]);

    let o3 = scheduler.create_hot_observable(vec![RT::on_completed::<i32>(250)]);

    let res = scheduler.start({
        let sources: Vec<Source> = vec![o1.as_observable(), o2.as_observable(), o3.as_observable()];
        move || Observable::on_error_resume_next_all(sources)
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(230, 3), RT::on_completed(250));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(220, 240));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(240, 250));
}

#[test]
fn on_error_resume_next_empty_return_throw_and_more() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(205)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(215, 2), RT::on_completed(220)]);

    let o3 = scheduler.create_hot_observable(vec![RT::on_next(225, 3), RT::on_next(230, 4), RT::on_completed(235)]);

    let o4 = scheduler.create_hot_observable(vec![RT::on_error::<i32>(240, new_error("ex"))]);

    let o5 = scheduler.create_hot_observable(vec![RT::on_next(245, 5), RT::on_completed(250)]);

    let res = scheduler.start({
        let sources: Vec<Source> = vec![o1.as_observable(), o2.as_observable(), o3.as_observable(), o4.as_observable(), o5.as_observable()];
        move || Observable::on_error_resume_next_all(sources)
    });

    assert_messages!(res.messages(), RT::on_next(215, 2), RT::on_next(225, 3), RT::on_next(230, 4), RT::on_next(245, 5), RT::on_completed(250));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 205));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(205, 220));

    assert_subscriptions!(o3.subscriptions(), RT::subscribe(220, 235));

    assert_subscriptions!(o4.subscriptions(), RT::subscribe(235, 240));

    assert_subscriptions!(o5.subscriptions(), RT::subscribe(240, 250));
}

#[test]
fn on_error_resume_next_last_isnt_special() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(220)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_error::<i32>(230, ex.clone())]);

    let res = scheduler.start({
        let (o1, o2) = (o1.clone(), o2.clone());
        move || o1.as_observable().on_error_resume_next(o2.as_observable())
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_completed(230));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(220, 230));
}

#[test]
fn on_error_resume_next_single_source_doesnt_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let o1 = scheduler.create_hot_observable(vec![RT::on_error::<i32>(230, ex.clone())]);

    let res = scheduler.start({
        let sources: Vec<Source> = vec![o1.as_observable()];
        move || Observable::on_error_resume_next_all(sources)
    });

    assert_messages!(res.messages(), RT::on_completed(230));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));
}

#[test]
fn on_error_resume_next_end_with_never() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(220)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let res = scheduler.start({
        let sources: Vec<Source> = vec![o1.as_observable(), o2.as_observable()];
        move || Observable::on_error_resume_next_all(sources)
    });

    assert_messages!(res.messages(), RT::on_next(210, 2));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 220));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(220, 1000));
}

#[test]
fn on_error_resume_next_start_with_never() {
    let scheduler = TestScheduler::new();

    let o1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let o2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(220)]);

    let res = scheduler.start({
        let sources: Vec<Source> = vec![o1.as_observable(), o2.as_observable()];
        move || Observable::on_error_resume_next_all(sources)
    });

    assert_messages!(res.messages());

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 1000));

    assert_subscriptions!(o2.subscriptions());
}

#[test]
fn on_error_resume_next_default_scheduler_binary() {
    let sum = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(false));
    let _d = Observable::return_(1).on_error_resume_next(Observable::return_(2)).subscribe_next_completed(
        {
            let sum = sum.clone();
            move |x| sum.set(sum.get() + x)
        },
        {
            let completed = completed.clone();
            move || completed.set(true)
        },
    );

    assert!(completed.get());
    assert_eq!(3, sum.get());
}

#[test]
fn on_error_resume_next_default_scheduler_nary() {
    let sum = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(false));
    let _d = Observable::on_error_resume_next_all(vec![Observable::return_(1), Observable::return_(2), Observable::return_(3)]).subscribe_next_completed(
        {
            let sum = sum.clone();
            move |x| sum.set(sum.get() + x)
        },
        {
            let completed = completed.clone();
            move || completed.set(true)
        },
    );

    assert!(completed.get());
    assert_eq!(6, sum.get());
}

#[test]
fn on_error_resume_next_default_scheduler_nary_enumerable() {
    let sources: Vec<Source> = vec![Observable::return_(1), Observable::return_(2), Observable::return_(3)];

    let sum = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(false));
    let _d = Observable::on_error_resume_next_all(sources).subscribe_next_completed(
        {
            let sum = sum.clone();
            move |x| sum.set(sum.get() + x)
        },
        {
            let completed = completed.clone();
            move || completed.set(true)
        },
    );

    assert!(completed.get());
    assert_eq!(6, sum.get());
}

#[test]
fn on_error_resume_next_iterator_throws() {
    let scheduler = TestScheduler::new();
    let ex = new_error("ex");

    // `Catch_IteratorThrows_Source(ex, true)`: the first `MoveNext` throws.
    let res = scheduler.start({
        let ex = ex.clone();
        move || Observable::try_on_error_resume_next_all(move || Ok::<_, ObservableError>(std::iter::once(Err::<Source, _>(ex.clone()))))
    });

    assert_messages!(res.messages(), RT::on_error(200, ex.clone()));
}

#[test]
fn on_error_resume_next_enumerable_throws() {
    let scheduler = TestScheduler::new();

    let o = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_error(225, new_error("ex"))]);

    let ex = new_error("ex");
    // `GetObservablesForOnErrorResumeNextThrow(o, ex)`: yields `o`, then throws.
    let xss = MockEnumerable::new(&scheduler, {
        let (o, ex) = (o.as_observable(), ex.clone());
        move || Box::new(vec![Ok(o.clone()), Err(ex.clone())].into_iter()) as Box<dyn Iterator<Item = TrySource>>
    });

    let res = scheduler.start({
        let xss = xss.clone();
        move || Observable::try_on_error_resume_next_all(move || Ok::<_, ObservableError>(xss.clone()))
    });

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_error(225, ex.clone()));

    assert_subscriptions!(o.subscriptions(), RT::subscribe(200, 225));

    assert_subscriptions!(xss.subscriptions(), RT::subscribe(200, 225));
}

#[test]
fn on_error_resume_next_enumerable_timing() {
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
        move || Observable::on_error_resume_next_all(xss)
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
fn on_error_resume_next_enumerable_dispose() {
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
            move || Observable::on_error_resume_next_all(xss)
        },
        300,
    );

    assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_next(220, 3), RT::on_next(240, 4), RT::on_next(270, 5));

    assert_subscriptions!(o1.subscriptions(), RT::subscribe(200, 230));

    assert_subscriptions!(o2.subscriptions(), RT::subscribe(230, 300));

    assert_subscriptions!(xss.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn on_error_resume_next_tail_recursive1() {
    let create = 0;
    let start = 200;
    let end = 1000;

    let scheduler = TestScheduler::new();

    let o = scheduler
        .create_cold_observable(vec![RT::on_next(10, 1), RT::on_next(20, 2), RT::on_next(30, 3), RT::on_error(40, new_error("ex"))])
        .as_observable();

    fn f(o: Source) -> Source {
        Observable::defer(move || o.on_error_resume_next(f(o.clone())))
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

// Not ported: OnErrorResumeNext_ArgumentChecking - argument validation
// Not ported: OnErrorResumeNext_TailRecursive2 - real threads (ThreadPoolScheduler, stack traces); blocking operator ForEach (waived: one thread)
// Not ported: OnErrorResumeNext_TailRecursive3 - blocking operator Wait (waived: one thread)
