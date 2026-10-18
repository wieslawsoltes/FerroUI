//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/UsingTest.cs` of Rx.NET.
//! Adapted: the variables that the factories assign are shared slots; a factory that throws is a
//! factory of `Observable::try_using` returning the error; `MockDisposable` (of the test helpers
//! upstream) is defined here: it records the virtual time it was created at and the times it was
//! disposed at.

use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{IDisposable, IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ColdObservable, ReactiveTest as RT, TestScheduler};
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

/// A disposable that records the clock of the scheduler when it is created and each time it is
/// disposed (`MockDisposable`).
struct MockDisposable {
    scheduler: TestScheduler,
    ticks: RefCell<Vec<i64>>,
}

impl MockDisposable {
    fn new(scheduler: &TestScheduler) -> Rc<Self> {
        Rc::new(Self { scheduler: scheduler.clone(), ticks: RefCell::new(vec![scheduler.clock()]) })
    }

    fn ticks(&self) -> Vec<i64> {
        self.ticks.borrow().clone()
    }
}

impl IDisposable for MockDisposable {
    fn dispose(&self) {
        self.ticks.borrow_mut().push(self.scheduler.clock());
    }
}

/// The scenario of `Using_Complete`, `Using_Error` and `Using_Dispose`: a mock resource, and a cold
/// sequence made of `messages(clock)` at the time of the subscription.
struct Scenario {
    dispose_invoked: Rc<Cell<i32>>,
    create_invoked: Rc<Cell<i32>>,
    xs: Slot<Rc<ColdObservable<i64>>>,
    disposable: Slot<Rc<MockDisposable>>,
    d: Slot<Rc<MockDisposable>>,
}

impl Scenario {
    fn new() -> Self {
        Self { dispose_invoked: Rc::new(Cell::new(0)), create_invoked: Rc::new(Cell::new(0)), xs: slot(), disposable: slot(), d: slot() }
    }

    fn using(
        &self,
        scheduler: &TestScheduler,
        messages: impl Fn(i64) -> Vec<ferroui_reactive_testing::Recorded<ferroui_reactive::Notification<i64>>> + 'static,
    ) -> impl FnOnce() -> Rc<dyn IObservable<i64>> + 'static {
        let (s1, s2) = (scheduler.clone(), scheduler.clone());
        let (dispose_invoked, create_invoked) = (self.dispose_invoked.clone(), self.create_invoked.clone());
        let (xs, disposable, d) = (self.xs.clone(), self.disposable.clone(), self.d.clone());
        move || {
            Observable::using(
                move || {
                    dispose_invoked.set(dispose_invoked.get() + 1);
                    let resource = MockDisposable::new(&s1);
                    set(&disposable, resource.clone());
                    resource
                },
                move |resource: Rc<MockDisposable>| -> Rc<dyn IObservable<i64>> {
                    set(&d, resource);
                    create_invoked.set(create_invoked.get() + 1);
                    let cold = s2.create_cold_observable(messages(s2.clock()));
                    set(&xs, cold.clone());
                    cold
                },
            )
        }
    }

    fn assert_same_resource(&self) {
        assert!(Rc::ptr_eq(&get(&self.disposable), &get(&self.d)));
    }
}

#[test]
fn using_complete() {
    let scheduler = TestScheduler::new();

    let scenario = Scenario::new();
    let create = scenario.using(&scheduler, |clock| vec![RT::on_next(100, clock), RT::on_completed(200)]);
    let res = scheduler.start(create);

    scenario.assert_same_resource();

    assert_messages!(res.messages(), RT::on_next(300, 200i64), RT::on_completed::<i64>(400));

    assert_eq!(1, scenario.create_invoked.get());
    assert_eq!(1, scenario.dispose_invoked.get());

    assert_subscriptions!(get(&scenario.xs).subscriptions(), RT::subscribe(200, 400));

    assert_eq!(vec![200, 400], get(&scenario.disposable).ticks());
}

#[test]
fn using_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let scenario = Scenario::new();
    let error = ex.clone();
    let create = scenario.using(&scheduler, move |clock| vec![RT::on_next(100, clock), RT::on_error(200, error.clone())]);
    let res = scheduler.start(create);

    scenario.assert_same_resource();

    assert_messages!(res.messages(), RT::on_next(300, 200i64), RT::on_error::<i64>(400, ex));

    assert_eq!(1, scenario.create_invoked.get());
    assert_eq!(1, scenario.dispose_invoked.get());

    assert_subscriptions!(get(&scenario.xs).subscriptions(), RT::subscribe(200, 400));

    assert_eq!(vec![200, 400], get(&scenario.disposable).ticks());
}

#[test]
fn using_dispose() {
    let scheduler = TestScheduler::new();

    let scenario = Scenario::new();
    let create = scenario.using(&scheduler, |clock| vec![RT::on_next(100, clock), RT::on_next(1000, clock + 1)]);
    let res = scheduler.start(create);

    scenario.assert_same_resource();

    assert_messages!(res.messages(), RT::on_next(300, 200i64));

    assert_eq!(1, scenario.create_invoked.get());
    assert_eq!(1, scenario.dispose_invoked.get());

    assert_subscriptions!(get(&scenario.xs).subscriptions(), RT::subscribe(200, 1000));

    assert_eq!(vec![200, 1000], get(&scenario.disposable).ticks());
}

#[test]
fn using_throw_resource_selector() {
    let scheduler = TestScheduler::new();

    let dispose_invoked = Rc::new(Cell::new(0));
    let create_invoked = Rc::new(Cell::new(0));
    let ex = new_error("ex");

    let (di, ci, error) = (dispose_invoked.clone(), create_invoked.clone(), ex.clone());
    let res = scheduler.start(move || {
        Observable::try_using(
            move || -> Result<Rc<dyn IDisposable>, _> {
                di.set(di.get() + 1);
                Err(error.clone())
            },
            move |_d: Rc<dyn IDisposable>| {
                ci.set(ci.get() + 1);
                Ok(Observable::never::<i32>())
            },
        )
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex));

    assert_eq!(0, create_invoked.get());
    assert_eq!(1, dispose_invoked.get());
}

#[test]
fn using_throw_resource_usage() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let dispose_invoked = Rc::new(Cell::new(0));
    let create_invoked = Rc::new(Cell::new(0));
    let disposable: Slot<Rc<MockDisposable>> = slot();

    let (s, di, ci, disp, error) = (scheduler.clone(), dispose_invoked.clone(), create_invoked.clone(), disposable.clone(), ex.clone());
    let res = scheduler.start(move || {
        Observable::try_using(
            move || {
                di.set(di.get() + 1);
                let resource = MockDisposable::new(&s);
                set(&disp, resource.clone());
                Ok(resource)
            },
            move |_d: Rc<MockDisposable>| -> Result<Rc<dyn IObservable<i32>>, _> {
                ci.set(ci.get() + 1);
                Err(error.clone())
            },
        )
    });

    assert_messages!(res.messages(), RT::on_error::<i32>(200, ex));

    assert_eq!(1, create_invoked.get());
    assert_eq!(1, dispose_invoked.get());

    assert_eq!(vec![200, 200], get(&disposable).ticks());
}

/// `Observable.Using(() => Disposable.Create(() => order += mark), _ => inner())`.
fn using_appending(order: &Rc<RefCell<String>>, mark: &'static str, inner: impl Fn() -> Rc<dyn IObservable<()>> + 'static) -> Rc<dyn IObservable<()>> {
    let order = order.clone();
    Observable::using(
        move || -> Rc<dyn IDisposable> {
            let order = order.clone();
            Disposable::create(move || order.borrow_mut().push_str(mark))
        },
        move |_resource: Rc<dyn IDisposable>| inner(),
    )
}

#[test]
fn using_nested_completed() {
    let order = Rc::new(RefCell::new(String::new()));

    let (o2, o1, o4) = (order.clone(), order.clone(), order.clone());
    using_appending(&order, "3", move || {
        let o1 = o1.clone();
        using_appending(&o2, "2", move || using_appending(&o1, "1", || Observable::return_(())))
    })
    .finally(move || o4.borrow_mut().push('4'))
    .subscribe_ignore();

    assert_eq!("1234", *order.borrow());
}

#[test]
fn using_nested_disposed() {
    let order = Rc::new(RefCell::new(String::new()));

    let (o2, o1, o4) = (order.clone(), order.clone(), order.clone());
    using_appending(&order, "3", move || {
        let o1 = o1.clone();
        using_appending(&o2, "2", move || using_appending(&o1, "1", Observable::never::<()>))
    })
    .finally(move || o4.borrow_mut().push('4'))
    .subscribe_ignore()
    .dispose();

    assert_eq!("1234", *order.borrow());
}

// Not ported: Using_ArgumentChecking - argument validation (null factories, a factory that returns a null sequence, a null observer)
// Not ported: Using_Null - a null resource (the resource factory of the port returns an `Rc`, which cannot be null)
