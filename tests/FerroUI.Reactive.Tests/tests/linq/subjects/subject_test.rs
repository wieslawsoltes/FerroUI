//! Ported from `Tests.System.Reactive/Tests/Linq/Subjects/SubjectTest.cs` of Rx.NET.
//! Adapted: the variables that upstream assigns from scheduled actions are shared slots
//! (`Rc<RefCell<Option<_>>>`); `ObjectDisposedException` is a panic, checked with `assert_throws`;
//! `Subject.Create<TSource, TResult>` and `Subject.Create<T>` are both `SubjectFactory::create`.

use ferroui_reactive::subjects::{Subject, SubjectBase, SubjectFactory};
use ferroui_reactive::{IDisposable, IObservable, IObserver, Observable, ObservableError, ObservableExt, Observer};
use ferroui_reactive_testing::{assert_messages, assert_throws, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A variable that scheduled actions assign and read (`var x = default(X);` captured by lambdas).
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

fn same_error(a: &ObservableError, b: &ObservableError) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

#[test]
fn infinite() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_next(630, 8),
        RT::on_next(710, 9),
        RT::on_next(870, 10),
        RT::on_next(940, 11),
        RT::on_next(1020, 12),
    ]);

    let s: Slot<Rc<Subject<i32>>> = slot();
    let subscription: Slot<Rc<dyn IDisposable>> = slot();

    let results1 = scheduler.create_observer::<i32>();
    let subscription1: Slot<Rc<dyn IDisposable>> = slot();

    let results2 = scheduler.create_observer::<i32>();
    let subscription2: Slot<Rc<dyn IDisposable>> = slot();

    let results3 = scheduler.create_observer::<i32>();
    let subscription3: Slot<Rc<dyn IDisposable>> = slot();

    {
        let s = s.clone();
        scheduler.schedule_absolute_action(100, move || set(&s, Subject::new()));
    }
    {
        let (s, subscription, xs) = (s.clone(), subscription.clone(), xs.clone());
        scheduler.schedule_absolute_action(200, move || {
            let subject = get(&s);
            set(&subscription, xs.subscribe(subject));
        });
    }
    {
        let subscription = subscription.clone();
        scheduler.schedule_absolute_action(1000, move || get(&subscription).dispose());
    }

    {
        let (s, subscription1, results1) = (s.clone(), subscription1.clone(), results1.clone());
        scheduler.schedule_absolute_action(300, move || set(&subscription1, get(&s).subscribe(results1)));
    }
    {
        let (s, subscription2, results2) = (s.clone(), subscription2.clone(), results2.clone());
        scheduler.schedule_absolute_action(400, move || set(&subscription2, get(&s).subscribe(results2)));
    }
    {
        let (s, subscription3, results3) = (s.clone(), subscription3.clone(), results3.clone());
        scheduler.schedule_absolute_action(900, move || set(&subscription3, get(&s).subscribe(results3)));
    }

    {
        let subscription1 = subscription1.clone();
        scheduler.schedule_absolute_action(600, move || get(&subscription1).dispose());
    }
    {
        let subscription2 = subscription2.clone();
        scheduler.schedule_absolute_action(700, move || get(&subscription2).dispose());
    }
    {
        let subscription1 = subscription1.clone();
        scheduler.schedule_absolute_action(800, move || get(&subscription1).dispose());
    }
    {
        let subscription3 = subscription3.clone();
        scheduler.schedule_absolute_action(950, move || get(&subscription3).dispose());
    }

    scheduler.run();

    assert_messages!(results1.messages(), RT::on_next(340, 5), RT::on_next(410, 6), RT::on_next(520, 7));

    assert_messages!(results2.messages(), RT::on_next(410, 6), RT::on_next(520, 7), RT::on_next(630, 8));

    assert_messages!(results3.messages(), RT::on_next(940, 11));
}

#[test]
fn finite() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_completed(630),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let s: Slot<Rc<Subject<i32>>> = slot();
    let subscription: Slot<Rc<dyn IDisposable>> = slot();

    let results1 = scheduler.create_observer::<i32>();
    let subscription1: Slot<Rc<dyn IDisposable>> = slot();

    let results2 = scheduler.create_observer::<i32>();
    let subscription2: Slot<Rc<dyn IDisposable>> = slot();

    let results3 = scheduler.create_observer::<i32>();
    let subscription3: Slot<Rc<dyn IDisposable>> = slot();

    {
        let s = s.clone();
        scheduler.schedule_absolute_action(100, move || set(&s, Subject::new()));
    }
    {
        let (s, subscription, xs) = (s.clone(), subscription.clone(), xs.clone());
        scheduler.schedule_absolute_action(200, move || {
            let subject = get(&s);
            set(&subscription, xs.subscribe(subject));
        });
    }
    {
        let subscription = subscription.clone();
        scheduler.schedule_absolute_action(1000, move || get(&subscription).dispose());
    }

    {
        let (s, subscription1, results1) = (s.clone(), subscription1.clone(), results1.clone());
        scheduler.schedule_absolute_action(300, move || set(&subscription1, get(&s).subscribe(results1)));
    }
    {
        let (s, subscription2, results2) = (s.clone(), subscription2.clone(), results2.clone());
        scheduler.schedule_absolute_action(400, move || set(&subscription2, get(&s).subscribe(results2)));
    }
    {
        let (s, subscription3, results3) = (s.clone(), subscription3.clone(), results3.clone());
        scheduler.schedule_absolute_action(900, move || set(&subscription3, get(&s).subscribe(results3)));
    }

    {
        let subscription1 = subscription1.clone();
        scheduler.schedule_absolute_action(600, move || get(&subscription1).dispose());
    }
    {
        let subscription2 = subscription2.clone();
        scheduler.schedule_absolute_action(700, move || get(&subscription2).dispose());
    }
    {
        let subscription1 = subscription1.clone();
        scheduler.schedule_absolute_action(800, move || get(&subscription1).dispose());
    }
    {
        let subscription3 = subscription3.clone();
        scheduler.schedule_absolute_action(950, move || get(&subscription3).dispose());
    }

    scheduler.run();

    assert_messages!(results1.messages(), RT::on_next(340, 5), RT::on_next(410, 6), RT::on_next(520, 7));

    assert_messages!(results2.messages(), RT::on_next(410, 6), RT::on_next(520, 7), RT::on_completed::<i32>(630));

    assert_messages!(results3.messages(), RT::on_completed::<i32>(900));
}

#[test]
fn error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(70, 1),
        RT::on_next(110, 2),
        RT::on_next(220, 3),
        RT::on_next(270, 4),
        RT::on_next(340, 5),
        RT::on_next(410, 6),
        RT::on_next(520, 7),
        RT::on_error(630, ex.clone()),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("other")),
    ]);

    let s: Slot<Rc<Subject<i32>>> = slot();
    let subscription: Slot<Rc<dyn IDisposable>> = slot();

    let results1 = scheduler.create_observer::<i32>();
    let subscription1: Slot<Rc<dyn IDisposable>> = slot();

    let results2 = scheduler.create_observer::<i32>();
    let subscription2: Slot<Rc<dyn IDisposable>> = slot();

    let results3 = scheduler.create_observer::<i32>();
    let subscription3: Slot<Rc<dyn IDisposable>> = slot();

    {
        let s = s.clone();
        scheduler.schedule_absolute_action(100, move || set(&s, Subject::new()));
    }
    {
        let (s, subscription, xs) = (s.clone(), subscription.clone(), xs.clone());
        scheduler.schedule_absolute_action(200, move || {
            let subject = get(&s);
            set(&subscription, xs.subscribe(subject));
        });
    }
    {
        let subscription = subscription.clone();
        scheduler.schedule_absolute_action(1000, move || get(&subscription).dispose());
    }

    {
        let (s, subscription1, results1) = (s.clone(), subscription1.clone(), results1.clone());
        scheduler.schedule_absolute_action(300, move || set(&subscription1, get(&s).subscribe(results1)));
    }
    {
        let (s, subscription2, results2) = (s.clone(), subscription2.clone(), results2.clone());
        scheduler.schedule_absolute_action(400, move || set(&subscription2, get(&s).subscribe(results2)));
    }
    {
        let (s, subscription3, results3) = (s.clone(), subscription3.clone(), results3.clone());
        scheduler.schedule_absolute_action(900, move || set(&subscription3, get(&s).subscribe(results3)));
    }

    {
        let subscription1 = subscription1.clone();
        scheduler.schedule_absolute_action(600, move || get(&subscription1).dispose());
    }
    {
        let subscription2 = subscription2.clone();
        scheduler.schedule_absolute_action(700, move || get(&subscription2).dispose());
    }
    {
        let subscription1 = subscription1.clone();
        scheduler.schedule_absolute_action(800, move || get(&subscription1).dispose());
    }
    {
        let subscription3 = subscription3.clone();
        scheduler.schedule_absolute_action(950, move || get(&subscription3).dispose());
    }

    scheduler.run();

    assert_messages!(results1.messages(), RT::on_next(340, 5), RT::on_next(410, 6), RT::on_next(520, 7));

    assert_messages!(results2.messages(), RT::on_next(410, 6), RT::on_next(520, 7), RT::on_error::<i32>(630, ex.clone()));

    assert_messages!(results3.messages(), RT::on_error::<i32>(900, ex.clone()));
}

#[test]
fn canceled() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_completed::<i32>(630),
        RT::on_next(640, 9),
        RT::on_completed(650),
        RT::on_error(660, new_error("ex")),
    ]);

    let s: Slot<Rc<Subject<i32>>> = slot();
    let subscription: Slot<Rc<dyn IDisposable>> = slot();

    let results1 = scheduler.create_observer::<i32>();
    let subscription1: Slot<Rc<dyn IDisposable>> = slot();

    let results2 = scheduler.create_observer::<i32>();
    let subscription2: Slot<Rc<dyn IDisposable>> = slot();

    let results3 = scheduler.create_observer::<i32>();
    let subscription3: Slot<Rc<dyn IDisposable>> = slot();

    {
        let s = s.clone();
        scheduler.schedule_absolute_action(100, move || set(&s, Subject::new()));
    }
    {
        let (s, subscription, xs) = (s.clone(), subscription.clone(), xs.clone());
        scheduler.schedule_absolute_action(200, move || {
            let subject = get(&s);
            set(&subscription, xs.subscribe(subject));
        });
    }
    {
        let subscription = subscription.clone();
        scheduler.schedule_absolute_action(1000, move || get(&subscription).dispose());
    }

    {
        let (s, subscription1, results1) = (s.clone(), subscription1.clone(), results1.clone());
        scheduler.schedule_absolute_action(300, move || set(&subscription1, get(&s).subscribe(results1)));
    }
    {
        let (s, subscription2, results2) = (s.clone(), subscription2.clone(), results2.clone());
        scheduler.schedule_absolute_action(400, move || set(&subscription2, get(&s).subscribe(results2)));
    }
    {
        let (s, subscription3, results3) = (s.clone(), subscription3.clone(), results3.clone());
        scheduler.schedule_absolute_action(900, move || set(&subscription3, get(&s).subscribe(results3)));
    }

    {
        let subscription1 = subscription1.clone();
        scheduler.schedule_absolute_action(600, move || get(&subscription1).dispose());
    }
    {
        let subscription2 = subscription2.clone();
        scheduler.schedule_absolute_action(700, move || get(&subscription2).dispose());
    }
    {
        let subscription1 = subscription1.clone();
        scheduler.schedule_absolute_action(800, move || get(&subscription1).dispose());
    }
    {
        let subscription3 = subscription3.clone();
        scheduler.schedule_absolute_action(950, move || get(&subscription3).dispose());
    }

    scheduler.run();

    assert_messages!(results1.messages());

    assert_messages!(results2.messages(), RT::on_completed::<i32>(630));

    assert_messages!(results3.messages(), RT::on_completed::<i32>(900));
}

#[test]
fn dispose() {
    let scheduler = TestScheduler::new();

    let s = Subject::<i32>::new();

    {
        let s = s.clone();
        scheduler.schedule_absolute_action(300, move || s.on_next(1));
    }
    {
        let s = s.clone();
        scheduler.schedule_absolute_action(998, move || s.on_next(2));
    }
    {
        let s = s.clone();
        scheduler.schedule_absolute_action(999, move || s.on_next(3));
    }
    {
        let s = s.clone();
        scheduler.schedule_absolute_action(1001, move || s.on_next(3));
    }

    let results = {
        let s = s.clone();
        scheduler.start(move || s.as_observable())
    };

    assert_messages!(results.messages(), RT::on_next(300, 1), RT::on_next(998, 2), RT::on_next(999, 3));
}

#[test]
fn pre_complete() {
    let scheduler = TestScheduler::new();

    let s = Subject::<i32>::new();

    {
        let s = s.clone();
        scheduler.schedule_absolute_action(90, move || s.on_completed());
    }

    let results = {
        let s = s.clone();
        scheduler.start(move || s.as_observable())
    };

    assert_messages!(results.messages(), RT::on_completed::<i32>(200));
}

#[test]
fn subject_disposed() {
    let scheduler = TestScheduler::new();

    let subject: Slot<Rc<Subject<i32>>> = slot();

    let results1 = scheduler.create_observer::<i32>();
    let subscription1: Slot<Rc<dyn IDisposable>> = slot();

    let results2 = scheduler.create_observer::<i32>();
    let subscription2: Slot<Rc<dyn IDisposable>> = slot();

    let results3 = scheduler.create_observer::<i32>();
    let subscription3: Slot<Rc<dyn IDisposable>> = slot();

    {
        let subject = subject.clone();
        scheduler.schedule_absolute_action(100, move || set(&subject, Subject::new()));
    }
    {
        let (subject, subscription1, results1) = (subject.clone(), subscription1.clone(), results1.clone());
        scheduler.schedule_absolute_action(200, move || set(&subscription1, get(&subject).subscribe(results1)));
    }
    {
        let (subject, subscription2, results2) = (subject.clone(), subscription2.clone(), results2.clone());
        scheduler.schedule_absolute_action(300, move || set(&subscription2, get(&subject).subscribe(results2)));
    }
    {
        let (subject, subscription3, results3) = (subject.clone(), subscription3.clone(), results3.clone());
        scheduler.schedule_absolute_action(400, move || set(&subscription3, get(&subject).subscribe(results3)));
    }
    {
        let subscription1 = subscription1.clone();
        scheduler.schedule_absolute_action(500, move || get(&subscription1).dispose());
    }
    {
        let subject = subject.clone();
        scheduler.schedule_absolute_action(600, move || get(&subject).dispose());
    }
    {
        let subscription2 = subscription2.clone();
        scheduler.schedule_absolute_action(700, move || get(&subscription2).dispose());
    }
    {
        let subscription3 = subscription3.clone();
        scheduler.schedule_absolute_action(800, move || get(&subscription3).dispose());
    }

    {
        let subject = subject.clone();
        scheduler.schedule_absolute_action(150, move || get(&subject).on_next(1));
    }
    {
        let subject = subject.clone();
        scheduler.schedule_absolute_action(250, move || get(&subject).on_next(2));
    }
    {
        let subject = subject.clone();
        scheduler.schedule_absolute_action(350, move || get(&subject).on_next(3));
    }
    {
        let subject = subject.clone();
        scheduler.schedule_absolute_action(450, move || get(&subject).on_next(4));
    }
    {
        let subject = subject.clone();
        scheduler.schedule_absolute_action(550, move || get(&subject).on_next(5));
    }
    {
        let subject = subject.clone();
        scheduler.schedule_absolute_action(650, move || assert_throws(|| get(&subject).on_next(6)));
    }
    {
        let subject = subject.clone();
        scheduler.schedule_absolute_action(750, move || assert_throws(|| get(&subject).on_completed()));
    }
    {
        let subject = subject.clone();
        scheduler.schedule_absolute_action(850, move || assert_throws(|| get(&subject).on_error(new_error("ex"))));
    }
    {
        let subject = subject.clone();
        scheduler.schedule_absolute_action(950, move || {
            assert_throws(|| {
                get(&subject).as_observable().subscribe_ignore();
            })
        });
    }

    scheduler.run();

    assert_messages!(results1.messages(), RT::on_next(250, 2), RT::on_next(350, 3), RT::on_next(450, 4));

    assert_messages!(results2.messages(), RT::on_next(350, 3), RT::on_next(450, 4), RT::on_next(550, 5));

    assert_messages!(results3.messages(), RT::on_next(450, 4), RT::on_next(550, 5));
}

#[test]
fn subject_create1() {
    let x = Rc::new(Cell::new(0));
    let ex: Slot<ObservableError> = slot();
    let done = Rc::new(Cell::new(false));

    let v = {
        let (x, ex, done) = (x.clone(), ex.clone(), done.clone());
        Observer::create_all(move |value: i32| x.set(value), move |error| set(&ex, error), move || done.set(true))
    };
    let o = Observable::return_(42);

    let s = SubjectFactory::create::<i32, i32>(v, o);

    // `s.Subscribe(null)` throwing `ArgumentNullException` is argument validation.
    {
        let x = x.clone();
        s.subscribe(Observer::create(move |value: i32| x.set(value)));
    }
    assert_eq!(x.get(), 42);

    s.on_next(21);
    assert_eq!(x.get(), 21);

    // `s.OnError(null)` throwing `ArgumentNullException` is argument validation.
    let e = new_error("e");
    s.on_error(e.clone());
    assert!(same_error(&e, &get(&ex)));

    s.on_completed();
    assert!(!done.get()); // already cut off
}

#[test]
fn subject_create2() {
    let x = Rc::new(Cell::new(0));
    let ex: Slot<ObservableError> = slot();
    let done = Rc::new(Cell::new(false));

    let v = {
        let (x, ex, done) = (x.clone(), ex.clone(), done.clone());
        Observer::create_all(move |value: i32| x.set(value), move |error| set(&ex, error), move || done.set(true))
    };
    let o = Observable::return_(42);

    // `Subject.Create<T>(IObserver<T>, IObservable<T>)`: the same factory with one type.
    let s = SubjectFactory::create::<i32, i32>(v, o);

    // `s.Subscribe(null)` throwing `ArgumentNullException` is argument validation.
    {
        let x = x.clone();
        s.subscribe(Observer::create(move |value: i32| x.set(value)));
    }
    assert_eq!(x.get(), 42);

    s.on_next(21);
    assert_eq!(x.get(), 21);

    // `s.OnError(null)` throwing `ArgumentNullException` is argument validation.
    let e = new_error("e");
    s.on_error(e.clone());
    assert!(same_error(&e, &get(&ex)));

    s.on_completed();
    assert!(!done.get()); // already cut off
}

#[test]
fn has_observers() {
    let s = Subject::<i32>::new();
    assert!(!s.has_observers());

    let d1 = s.as_observable().subscribe_next(|_| {});
    assert!(s.has_observers());

    d1.dispose();
    assert!(!s.has_observers());

    let d2 = s.as_observable().subscribe_next(|_| {});
    assert!(s.has_observers());

    let d3 = s.as_observable().subscribe_next(|_| {});
    assert!(s.has_observers());

    d2.dispose();
    assert!(s.has_observers());

    d3.dispose();
    assert!(!s.has_observers());
}

#[test]
fn has_observers_dispose1() {
    let s = Subject::<i32>::new();
    assert!(!s.has_observers());
    assert!(!s.is_disposed());

    let d = s.as_observable().subscribe_next(|_| {});
    assert!(s.has_observers());
    assert!(!s.is_disposed());

    s.dispose();
    assert!(!s.has_observers());
    assert!(s.is_disposed());

    d.dispose();
    assert!(!s.has_observers());
    assert!(s.is_disposed());
}

#[test]
fn has_observers_dispose2() {
    let s = Subject::<i32>::new();
    assert!(!s.has_observers());
    assert!(!s.is_disposed());

    let d = s.as_observable().subscribe_next(|_| {});
    assert!(s.has_observers());
    assert!(!s.is_disposed());

    d.dispose();
    assert!(!s.has_observers());
    assert!(!s.is_disposed());

    s.dispose();
    assert!(!s.has_observers());
    assert!(s.is_disposed());
}

#[test]
fn has_observers_dispose3() {
    let s = Subject::<i32>::new();
    assert!(!s.has_observers());
    assert!(!s.is_disposed());

    s.dispose();
    assert!(!s.has_observers());
    assert!(s.is_disposed());
}

#[test]
fn has_observers_on_completed() {
    let s = Subject::<i32>::new();
    assert!(!s.has_observers());

    let _d = s.as_observable().subscribe_next(|_| {});
    assert!(s.has_observers());

    s.on_next(42);
    assert!(s.has_observers());

    s.on_completed();
    assert!(!s.has_observers());
}

#[test]
fn has_observers_on_error() {
    let s = Subject::<i32>::new();
    assert!(!s.has_observers());

    let _d = s.as_observable().subscribe_next_error(|_| {}, |_| {});
    assert!(s.has_observers());

    s.on_next(42);
    assert!(s.has_observers());

    s.on_error(new_error("ex"));
    assert!(!s.has_observers());
}

// Not ported: Subscribe_ArgumentChecking - argument validation
// Not ported: OnError_ArgumentChecking - argument validation
// Not ported: Subject_Create_ArgumentChecking - argument validation
// Not ported: Subject_Synchronize_ArgumentChecking - argument validation
// Not ported: Subject_Synchronize1 - real threads
// Not ported: Subject_Synchronize2 - real threads
