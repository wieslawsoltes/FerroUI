//! Ported from `Tests.System.Reactive/Tests/NotificationTest.cs` of Rx.NET.
//! Adapted: `ToString` is the `Debug` format of `Notification<T>` (`OnNext(42)`, `OnError(message)`,
//! `OnCompleted()`); `Value` of an error or the end is `None` where upstream throws; comparisons
//! with `null` and with values of other types (`Equals(null)`, `Equals("")`, `n == null`) have no
//! form in Rust and are left out of the equality tests.

use ferroui_reactive::{IObserver, Notification, NotificationKind, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn same_error(a: &ObservableError, b: &ObservableError) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

// ToObservable

#[test]
fn to_observable_empty() {
    let scheduler = TestScheduler::new();
    let s = scheduler.scheduler();

    let res = scheduler.start(move || Notification::<i32>::OnCompleted.to_observable_on(s));

    assert_messages!(res.messages(), RT::on_completed::<i32>(201));
}

#[test]
fn to_observable_return() {
    let scheduler = TestScheduler::new();
    let s = scheduler.scheduler();

    let res = scheduler.start(move || Notification::OnNext(42).to_observable_on(s));

    assert_messages!(res.messages(), RT::on_next(201, 42), RT::on_completed::<i32>(201));
}

#[test]
fn to_observable_throw() {
    let ex = new_error("ex");

    let scheduler = TestScheduler::new();
    let s = scheduler.scheduler();

    let error = ex.clone();
    let res = scheduler.start(move || Notification::<i32>::OnError(error).to_observable_on(s));

    assert_messages!(res.messages(), RT::on_error::<i32>(201, ex.clone()));
}

#[test]
fn to_observable_current_thread() {
    // Upstream waits on a ManualResetEvent; the immediate scheduler completes before the
    // subscription returns, so a flag stands in for the event.
    let evt = Rc::new(Cell::new(false));

    let set = evt.clone();
    Notification::<i32>::OnCompleted.to_observable().subscribe_next_completed(|_| {}, move || set.set(true));

    assert!(evt.get());
}

#[test]
fn notifications_equality() {
    let n = Notification::OnNext(42);
    let e = Notification::<i32>::OnError(new_error("ex"));
    let c = Notification::<i32>::OnCompleted;

    let (n1, n2) = (n.clone(), n.clone());
    let (e1, e2) = (e.clone(), e.clone());
    let (c1, c2) = (c.clone(), c.clone());

    assert!(n1 == n2);
    assert!(e1 == e2);
    assert!(c1 == c2);

    assert!(n1.eq(&n2));
    assert!(e1.eq(&e2));
    assert!(c1.eq(&c2));
}

#[test]
fn on_next_ctor_and_props() {
    let n = Notification::OnNext(42);
    assert_eq!(n.kind(), NotificationKind::OnNext);
    assert!(n.has_value());
    assert_eq!(n.value(), Some(&42));
    assert!(n.exception().is_none());
}

#[test]
#[allow(clippy::nonminimal_bool)]
fn on_next_equality() {
    let n1 = Notification::OnNext(42);
    let n2 = Notification::OnNext(42);
    let n3 = Notification::OnNext(24);
    let n4 = Notification::<i32>::OnCompleted;

    assert!(n1.eq(&n1));
    assert!(n1.eq(&n2));
    assert!(n2.eq(&n1));

    assert!(!n1.eq(&n3));
    assert!(!n3.eq(&n1));
    assert!(!n1.eq(&n4));
    assert!(!n4.eq(&n1));

    assert!(n1 == n2);
    assert!(n2 == n1);
    assert!(!(n1 != n2));
    assert!(!(n2 != n1));
}

#[test]
fn on_next_to_string() {
    let n1 = Notification::OnNext(42);
    assert!(format!("{n1:?}").contains("OnNext"));
    assert!(format!("{n1:?}").contains(&42.to_string()));
}

struct CheckOnNextObserver {
    value: Cell<i32>,
}

impl IObserver<i32> for CheckOnNextObserver {
    fn on_next(&self, value: i32) {
        self.value.set(value);
    }

    fn on_error(&self, _error: ObservableError) {
        unimplemented!()
    }

    fn on_completed(&self) {
        unimplemented!()
    }
}

#[test]
fn on_next_accept_observer() {
    let con = CheckOnNextObserver { value: Cell::new(0) };
    let n1 = Notification::OnNext(42);
    n1.accept(&con);

    assert_eq!(con.value.get(), 42);
}

#[test]
fn on_next_accept_action() {
    let mut obs = false;

    let n1 = Notification::OnNext(42);
    n1.accept_fns(|_| obs = true, |_| panic!("unexpected OnError"), || panic!("unexpected OnCompleted"));

    assert!(obs);
}

#[test]
fn on_next_accept_action_with_result() {
    let n1 = Notification::OnNext(42);
    let res = n1.accept_fns(|_| "OK", |_| -> &'static str { panic!("unexpected OnError") }, || -> &'static str { panic!("unexpected OnCompleted") });

    assert_eq!(res, "OK");
}

#[test]
fn on_error_ctor_and_props() {
    let e = new_error("e");
    let n = Notification::<i32>::OnError(e.clone());
    assert_eq!(n.kind(), NotificationKind::OnError);
    assert!(!n.has_value());
    assert!(same_error(&e, n.exception().unwrap()));

    // Upstream `Value` throws the error; the port has no value.
    assert!(n.value().is_none());
}

#[test]
#[allow(clippy::nonminimal_bool)]
fn on_error_equality() {
    let ex1 = new_error("ex1");
    let ex2 = new_error("ex2");

    let n1 = Notification::<i32>::OnError(ex1.clone());
    let n2 = Notification::<i32>::OnError(ex1.clone());
    let n3 = Notification::<i32>::OnError(ex2.clone());
    let n4 = Notification::<i32>::OnCompleted;

    assert!(n1.eq(&n1));
    assert!(n1.eq(&n2));
    assert!(n2.eq(&n1));

    assert!(!n1.eq(&n3));
    assert!(!n3.eq(&n1));
    assert!(!n1.eq(&n4));
    assert!(!n4.eq(&n1));

    assert!(n1 == n2);
    assert!(n2 == n1);
    assert!(!(n1 != n2));
    assert!(!(n2 != n1));
}

#[test]
fn on_error_to_string() {
    let ex = new_error("ex");

    let n1 = Notification::<i32>::OnError(ex.clone());
    assert!(format!("{n1:?}").contains("OnError"));
    // Upstream checks for the type name of the exception; the port prints the error's message.
    assert!(format!("{n1:?}").contains(&ex.to_string()));
}

struct CheckOnErrorObserver {
    error: RefCell<Option<ObservableError>>,
}

impl IObserver<i32> for CheckOnErrorObserver {
    fn on_next(&self, _value: i32) {
        unimplemented!()
    }

    fn on_error(&self, error: ObservableError) {
        *self.error.borrow_mut() = Some(error);
    }

    fn on_completed(&self) {
        unimplemented!()
    }
}

#[test]
fn on_error_accept_observer() {
    let ex = new_error("ex");

    let obs = CheckOnErrorObserver { error: RefCell::new(None) };

    let n1 = Notification::<i32>::OnError(ex.clone());
    n1.accept(&obs);

    assert!(same_error(&ex, obs.error.borrow().as_ref().unwrap()));
}

#[test]
fn on_error_accept_action() {
    let ex = new_error("ex");

    let mut obs = false;

    let n1 = Notification::<i32>::OnError(ex.clone());
    n1.accept_fns(|_| panic!("unexpected OnNext"), |_| obs = true, || panic!("unexpected OnCompleted"));

    assert!(obs);
}

#[test]
fn on_error_accept_action_with_result() {
    let ex = new_error("ex");

    let n1 = Notification::<i32>::OnError(ex.clone());
    let res = n1.accept_fns(|_| -> &'static str { panic!("unexpected OnNext") }, |_| "OK", || -> &'static str { panic!("unexpected OnCompleted") });

    assert_eq!(res, "OK");
}

#[test]
fn on_completed_ctor_and_props() {
    let n = Notification::<i32>::OnCompleted;
    assert_eq!(n.kind(), NotificationKind::OnCompleted);
    assert!(!n.has_value());
    assert!(n.exception().is_none());

    // Upstream `Value` throws InvalidOperationException; the port has no value.
    let ok = n.value().is_none();

    assert!(ok);
}

#[test]
#[allow(clippy::nonminimal_bool)]
fn on_completed_equality() {
    let n1 = Notification::<i32>::OnCompleted;
    let n2 = Notification::<i32>::OnCompleted;
    let n3 = Notification::OnNext(2);

    assert!(n1.eq(&n1));
    assert!(n1.eq(&n2));
    assert!(n2.eq(&n1));

    assert!(!n1.eq(&n3));
    assert!(!n3.eq(&n1));

    assert!(n1 == n2);
    assert!(n2 == n1);
    assert!(!(n1 != n2));
    assert!(!(n2 != n1));
}

#[test]
fn on_completed_to_string() {
    let n1 = Notification::<i32>::OnCompleted;
    assert!(format!("{n1:?}").contains("OnCompleted"));
}

struct CheckOnCompletedObserver {
    completed: Cell<bool>,
}

impl IObserver<i32> for CheckOnCompletedObserver {
    fn on_next(&self, _value: i32) {
        unimplemented!()
    }

    fn on_error(&self, _error: ObservableError) {
        unimplemented!()
    }

    fn on_completed(&self) {
        self.completed.set(true);
    }
}

#[test]
fn on_completed_accept_observer() {
    let obs = CheckOnCompletedObserver { completed: Cell::new(false) };

    let n1 = Notification::<i32>::OnCompleted;
    n1.accept(&obs);

    assert!(obs.completed.get());
}

#[test]
fn on_completed_accept_action() {
    let mut obs = false;

    let n1 = Notification::<i32>::OnCompleted;
    n1.accept_fns(|_| panic!("unexpected OnNext"), |_| panic!("unexpected OnError"), || obs = true);

    assert!(obs);
}

#[test]
fn on_completed_accept_action_with_result() {
    let n1 = Notification::<i32>::OnCompleted;
    let res = n1.accept_fns(|_| -> &'static str { panic!("unexpected OnNext") }, |_| -> &'static str { panic!("unexpected OnError") }, || "OK");

    assert_eq!(res, "OK");
}

// Not ported: ToObservable_ArgumentChecking - argument validation
// Not ported: OnNext_GetHashCode - Notification does not implement Hash
// Not ported: OnNext_AcceptObserverWithResult - IObserver<T, TResult> and Notification.Accept(IObserver<T, TResult>) are not in the crate
// Not ported: OnNext_AcceptObserverWithResult_Null - argument validation
// Not ported: OnNext_Accept_ArgumentChecking - argument validation
// Not ported: OnError_CtorNull - argument validation
// Not ported: OnError_Accept_ArgumentChecking - argument validation
// Not ported: OnError_GetHashCode - Notification does not implement Hash
// Not ported: OnError_AcceptObserverWithResult - IObserver<T, TResult> and Notification.Accept(IObserver<T, TResult>) are not in the crate
// Not ported: OnError_AcceptObserverWithResult_Null - argument validation
// Not ported: OnCompleted_Accept_ArgumentChecking - argument validation
// Not ported: OnCompleted_GetHashCode - Notification does not implement Hash
// Not ported: OnCompleted_AcceptObserverWithResult - IObserver<T, TResult> and Notification.Accept(IObserver<T, TResult>) are not in the crate
// Not ported: OnCompleted_AcceptObserverWithResult_Null - argument validation
