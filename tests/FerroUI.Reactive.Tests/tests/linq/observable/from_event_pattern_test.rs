//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/FromEventPatternTest.cs` of Rx.NET.
//! Adapted: the events of the fixtures (`FromEventPattern`, `MyEventArgs`, `MyEventScheduler`) are
//! FerroUI events (`ferroui_system::events::Event`), whose `add` returns the token that `remove`
//! takes; an event of a sender and arguments is an event of both, `Event<(Sender, A)>`, and the
//! sender (an `object` compared by reference upstream) is a `Sender`, the identity of the object
//! that raises the event. `FromEventPattern<TEventArgs>(add, remove)` is
//! `Observable::from_event_pattern`; the overloads with a delegate type and no conversion
//! (`FromEventPattern<EventHandler<A>, A>(add, remove)`, with a sender type or not), which upstream
//! makes the delegate of through reflection, are the closure forms of the port; the conversion
//! overloads are `Observable::from_event_pattern_conversion`, and the `_on` forms take the
//! scheduler. The anonymous objects `new { evt.Sender, evt.EventArgs }` are tuples; the cast of the
//! `EventArgs` of the non-generic `EventHandler` (`E5`) is not needed, the arguments having their
//! own type.

use ferroui_reactive::concurrency::{IScheduler, ScheduledAction, Scheduler};
use ferroui_reactive::{IDisposable, IObservable, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, new_error, ReactiveTest as RT, TestScheduler};
use ferroui_system::events::Event;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// The identity of the object that raises an event (the sender of upstream, an `object` compared
/// by reference).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sender(usize);

impl Sender {
    fn of<T>(object: &T) -> Self {
        Self(object as *const T as usize)
    }
}

/// The handler of an event of a sender and arguments, as a handler of a FerroUI event of both.
fn pattern_handler<S: Clone + 'static, A: Clone + 'static>(handler: Rc<dyn Fn(S, A)>) -> Rc<dyn Fn(&(S, A))> {
    Rc::new(move |(sender, event_args): &(S, A)| handler(sender.clone(), event_args.clone()))
}

/// `FromEventPattern.TestEventArgs`.
#[derive(Clone, Debug, PartialEq)]
struct TestEventArgs {
    id: i32,
}

/// `FromEventPattern`: the events of the strongly typed tests.
#[derive(Default)]
struct FromEventPattern {
    e2: Event<(Sender, TestEventArgs)>,
    e4: Event<i32>,
    e5: Event<(Sender, TestEventArgs)>,
}

impl FromEventPattern {
    fn m2(&self, i: i32) {
        self.e2.raise(&(Sender::of(self), TestEventArgs { id: i }));
    }

    fn m4(&self, i: i32) {
        self.e4.raise(&i);
    }

    fn m5(&self, i: i32) {
        self.e5.raise(&(Sender::of(self), TestEventArgs { id: i }));
    }
}

/// `EventArgs` (of the non-generic `EventHandler`).
#[derive(Clone, Debug, PartialEq)]
struct EventArgs;

/// `MyEventArgs`.
#[derive(Clone, Debug, PartialEq)]
struct MyEventArgs {
    value: i32,
}

/// `MyEventScheduler`: a scheduler that counts the work it is given and runs it at once.
struct MyEventScheduler {
    this: Weak<MyEventScheduler>,
    schedule: Box<dyn Fn()>,
}

impl MyEventScheduler {
    fn new(schedule: impl Fn() + 'static) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), schedule: Box::new(schedule) })
    }
}

impl IScheduler for MyEventScheduler {
    fn now(&self) -> SystemTime {
        Scheduler::now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        (self.schedule)();
        let scheduler: Rc<dyn IScheduler> = self.this.upgrade().expect("the scheduler is alive");
        action(&scheduler)
    }

    fn schedule_after(&self, _due_time: Duration, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        panic!("The method or operation is not implemented.")
    }

    fn schedule_at(&self, _due_time: SystemTime, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        panic!("The method or operation is not implemented.")
    }
}

/// Schedules the calls of `method` of the fixture at 50, 150, 250, 350, 450 and 1050, with the
/// arguments 1 to 6.
fn schedule_calls(scheduler: &TestScheduler, fe: &Rc<FromEventPattern>, method: fn(&FromEventPattern, i32)) {
    for (time, i) in [(50, 1), (150, 2), (250, 3), (350, 4), (450, 5), (1050, 6)] {
        let fe = fe.clone();
        scheduler.schedule_absolute_action(time, move || method(&fe, i));
    }
}

#[test]
fn from_event_pattern_e2() {
    let scheduler = TestScheduler::new();

    let fe = Rc::new(FromEventPattern::default());

    schedule_calls(&scheduler, &fe, FromEventPattern::m2);

    let (f1, f2) = (fe.clone(), fe.clone());
    let results = scheduler.start(move || {
        Observable::from_event_pattern(move |h: Rc<dyn Fn(Sender, TestEventArgs)>| f1.e2.add(pattern_handler(h)), move |token| {
            f2.e2.remove(token);
        })
        .select(|evt| (evt.sender, evt.event_args))
    });

    let sender = Sender::of(&*fe);
    assert_messages!(
        results.messages(),
        RT::on_next(250, (sender, TestEventArgs { id: 3 })),
        RT::on_next(350, (sender, TestEventArgs { id: 4 })),
        RT::on_next(450, (sender, TestEventArgs { id: 5 }))
    );
}

#[test]
fn from_event_pattern_conversion_e4() {
    let scheduler = TestScheduler::new();

    let fe = Rc::new(FromEventPattern::default());

    schedule_calls(&scheduler, &fe, FromEventPattern::m4);

    let sender = Sender::of(&*fe);
    let (f1, f2) = (fe.clone(), fe.clone());
    let results = scheduler.start(move || {
        Observable::from_event_pattern_conversion(
            move |h: Rc<dyn Fn(Sender, TestEventArgs)>| -> Rc<dyn Fn(&i32)> { Rc::new(move |x: &i32| h(sender, TestEventArgs { id: *x })) },
            move |h: Rc<dyn Fn(&i32)>| f1.e4.add(h),
            move |token| {
                f2.e4.remove(token);
            },
        )
        .select(|evt| (evt.sender, evt.event_args))
    });

    assert_messages!(
        results.messages(),
        RT::on_next(250, (sender, TestEventArgs { id: 3 })),
        RT::on_next(350, (sender, TestEventArgs { id: 4 })),
        RT::on_next(450, (sender, TestEventArgs { id: 5 }))
    );
}

#[test]
fn from_event_pattern_conversion_e5() {
    let scheduler = TestScheduler::new();

    let fe = Rc::new(FromEventPattern::default());

    schedule_calls(&scheduler, &fe, FromEventPattern::m5);

    let (f1, f2) = (fe.clone(), fe.clone());
    let results = scheduler.start(move || {
        Observable::from_event_pattern(move |h: Rc<dyn Fn(Sender, TestEventArgs)>| f1.e5.add(pattern_handler(h)), move |token| {
            f2.e5.remove(token);
        })
        .select(|evt| (evt.sender, evt.event_args))
    });

    let sender = Sender::of(&*fe);
    assert_messages!(
        results.messages(),
        RT::on_next(250, (sender, TestEventArgs { id: 3 })),
        RT::on_next(350, (sender, TestEventArgs { id: 4 })),
        RT::on_next(450, (sender, TestEventArgs { id: 5 }))
    );
}

#[test]
fn from_event_pattern_conversion_throws() {
    let ex = new_error("ex");

    let fe = Rc::new(FromEventPattern::default());

    let (f1, f2, e) = (fe.clone(), fe.clone(), ex.clone());
    let res = Observable::try_from_event_pattern_conversion(
        move |_h: Rc<dyn Fn(Sender, TestEventArgs)>| Err::<Rc<dyn Fn(&i32)>, _>(e.clone()),
        move |h: Rc<dyn Fn(&i32)>| f1.e4.add(h),
        move |token| {
            f2.e4.remove(token);
        },
    );

    let err: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let target = err.clone();
    let _d = res.subscribe_all(|_| panic!("no value is expected"), move |error| *target.borrow_mut() = Some(error), || panic!("no completion is expected"));

    let err = err.borrow().clone().expect("the error is notified");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&ex), Rc::as_ptr(&err)));
}

#[test]
fn from_event_pattern_e2_with_sender() {
    let scheduler = TestScheduler::new();

    let fe = Rc::new(FromEventPattern::default());

    schedule_calls(&scheduler, &fe, FromEventPattern::m2);

    // `FromEventPattern<EventHandler<TestEventArgs>, object, TestEventArgs>(add, remove)`: the
    // sender type is a parameter of every form of the port.
    let (f1, f2) = (fe.clone(), fe.clone());
    let results = scheduler.start(move || {
        Observable::from_event_pattern(move |h: Rc<dyn Fn(Sender, TestEventArgs)>| f1.e2.add(pattern_handler(h)), move |token| {
            f2.e2.remove(token);
        })
        .select(|evt| (evt.sender, evt.event_args))
    });

    let sender = Sender::of(&*fe);
    assert_messages!(
        results.messages(),
        RT::on_next(250, (sender, TestEventArgs { id: 3 })),
        RT::on_next(350, (sender, TestEventArgs { id: 4 })),
        RT::on_next(450, (sender, TestEventArgs { id: 5 }))
    );
}

#[test]
fn from_event_pattern_add_remove_e4() {
    let scheduler = TestScheduler::new();

    let fe = Rc::new(FromEventPattern::default());

    schedule_calls(&scheduler, &fe, FromEventPattern::m2);

    let (f1, f2) = (fe.clone(), fe.clone());
    let results = scheduler.start(move || {
        Observable::from_event_pattern(move |h: Rc<dyn Fn(Sender, TestEventArgs)>| f1.e2.add(pattern_handler(h)), move |token| {
            f2.e2.remove(token);
        })
        .select(|evt| (evt.sender, evt.event_args))
    });

    let sender = Sender::of(&*fe);
    assert_messages!(
        results.messages(),
        RT::on_next(250, (sender, TestEventArgs { id: 3 })),
        RT::on_next(350, (sender, TestEventArgs { id: 4 })),
        RT::on_next(450, (sender, TestEventArgs { id: 5 }))
    );
}

#[test]
fn from_event_pattern_scheduler1() {
    run_with_scheduler(|s, add, remove| {
        Observable::from_event_pattern_on(move |_h: Rc<dyn Fn(Sender, MyEventArgs)>| add(), move |_: ()| remove(), s)
    });
}

#[test]
fn from_event_pattern_scheduler4() {
    run_with_scheduler(|s, add, remove| Observable::from_event_pattern_on(move |_h: Rc<dyn Fn(Sender, EventArgs)>| add(), move |_: ()| remove(), s));
}

#[test]
fn from_event_pattern_scheduler7() {
    // `FromEventPattern<EventHandler<MyEventArgs>, MyEventArgs>(add, remove, s)`.
    run_with_scheduler(|s, add, remove| {
        Observable::from_event_pattern_on(move |_h: Rc<dyn Fn(Sender, MyEventArgs)>| add(), move |_: ()| remove(), s)
    });
}

#[test]
fn from_event_pattern_scheduler8() {
    run_with_scheduler(|s, add, remove| {
        Observable::from_event_pattern_conversion_on(
            |h: Rc<dyn Fn(Sender, MyEventArgs)>| h,
            move |_h: Rc<dyn Fn(Sender, MyEventArgs)>| add(),
            move |_: ()| remove(),
            s,
        )
    });
}

#[test]
fn from_event_pattern_scheduler11() {
    // `FromEventPattern<EventHandler<MyEventArgs>, object, MyEventArgs>(add, remove, s)`.
    run_with_scheduler(|s, add, remove| {
        Observable::from_event_pattern_on(move |_h: Rc<dyn Fn(Sender, MyEventArgs)>| add(), move |_: ()| remove(), s)
    });
}

/// `RunWithScheduler`: the handler is added on the scheduler at the first subscription only, and
/// removed on it at the disposal of the last subscription.
fn run_with_scheduler<T: 'static>(run: impl FnOnce(Rc<dyn IScheduler>, Rc<dyn Fn()>, Rc<dyn Fn()>) -> Rc<dyn IObservable<T>>) {
    let n = Rc::new(Cell::new(0));
    let a = Rc::new(Cell::new(0));
    let r = Rc::new(Cell::new(0));

    let counter = n.clone();
    let s: Rc<dyn IScheduler> = MyEventScheduler::new(move || counter.set(counter.get() + 1));

    let (added, removed) = (a.clone(), r.clone());
    let add: Rc<dyn Fn()> = Rc::new(move || added.set(added.get() + 1));
    let rem: Rc<dyn Fn()> = Rc::new(move || removed.set(removed.get() + 1));

    let xs = run(s, add, rem);

    assert_eq!(0, n.get());
    assert_eq!(0, a.get());
    assert_eq!(0, r.get());

    let d1 = xs.subscribe_ignore();
    assert_eq!(1, n.get());
    assert_eq!(1, a.get());
    assert_eq!(0, r.get());

    let d2 = xs.subscribe_ignore();
    assert_eq!(1, n.get());
    assert_eq!(1, a.get());
    assert_eq!(0, r.get());

    d1.dispose();
    assert_eq!(1, n.get());
    assert_eq!(1, a.get());
    assert_eq!(0, r.get());

    d2.dispose();
    assert_eq!(2, n.get());
    assert_eq!(1, a.get());
    assert_eq!(1, r.get());
}

// Not ported: FromEventPattern_Conversion_ArgumentChecking - argument validation
// Not ported: FromEventPattern_AddRemove_ArgumentChecking - argument validation
// Not ported: FromEventPattern_Reflection_Instance_ArgumentChecking - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Instance_InvalidVariance - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Instance_VarianceArgs - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Instance_VarianceSender - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Instance_NonGeneric - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Instance_Throws - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Instance_E1 - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Instance_E2 - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Instance_E2_WithSender - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Instance_E3 - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Instance_MissingAccessors - reflection (the event found by its name; compiled upstream for .NET Framework 4.7.2 only)
// Not ported: FromEventPattern_Reflection_Static_ArgumentChecking - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Static_E6 - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Static_E6_WithSender - reflection (the event found by its name)
// Not ported: FromEventPattern_Reflection_Static_NonGeneric_E6 - reflection (the event found by its name)
// Not ported: FromEventPattern_Scheduler2 - reflection (the event found by its name)
// Not ported: FromEventPattern_Scheduler3 - reflection (the event found by its name)
// Not ported: FromEventPattern_Scheduler5 - reflection (the event found by its name)
// Not ported: FromEventPattern_Scheduler6 - reflection (the event found by its name)
// Not ported: FromEventPattern_Scheduler9 - reflection (the event found by its name)
// Not ported: FromEventPattern_Scheduler10 - reflection (the event found by its name)
