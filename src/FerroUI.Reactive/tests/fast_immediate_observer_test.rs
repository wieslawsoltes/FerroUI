//! The tests of `FastImmediateObserver` from `Tests.System.Reactive/Tests/Linq/Subjects/ReplaySubjectTest.cs`
//! of Rx.NET (`FastImmediateObserver_Simple1` to `_Simple3` and `_Fault`), which test an internal
//! type. The recording observer stamps each notification with its position, as upstream's helper
//! does. `FastImmediateObserver_Ownership1` and `_Ownership2` use real threads and are not ported.

use crate::internal::{FastImmediateObserver, IScheduledObserver};
use crate::{Notification, Observer};
use ferroui_system::{IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
struct Fault;

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("fault")
    }
}

impl std::error::Error for Fault {}

fn fast_immediate_observer_test(run: impl FnOnce(&Rc<FastImmediateObserver<i32>>)) -> Vec<(i64, Notification<i32>)> {
    let recorded: Rc<RefCell<Vec<(i64, Notification<i32>)>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = recorded.clone();
    let observer: Rc<dyn IObserver<i32>> = Observer::to_observer(move |n: Notification<i32>| {
        let time = sink.borrow().len() as i64;
        sink.borrow_mut().push((time, n));
    });
    let fio = FastImmediateObserver::new(observer);
    run(&fio);
    let result = recorded.borrow().clone();
    result
}

#[test]
fn fast_immediate_observer_simple1() {
    let res = fast_immediate_observer_test(|fio| {
        fio.on_next(1);
        fio.on_next(2);
        fio.on_next(3);
        fio.on_completed();
        fio.ensure_active_n(4);
    });
    assert_eq!(
        res,
        vec![(0, Notification::OnNext(1)), (1, Notification::OnNext(2)), (2, Notification::OnNext(3)), (3, Notification::OnCompleted)]
    );
}

#[test]
fn fast_immediate_observer_simple2() {
    let ex: ObservableError = Rc::new(Fault);
    let error = ex.clone();
    let res = fast_immediate_observer_test(move |fio| {
        fio.on_next(1);
        fio.on_next(2);
        fio.on_next(3);
        fio.on_error(error);
        fio.ensure_active_n(4);
    });
    assert_eq!(
        res,
        vec![(0, Notification::OnNext(1)), (1, Notification::OnNext(2)), (2, Notification::OnNext(3)), (3, Notification::OnError(ex))]
    );
}

#[test]
fn fast_immediate_observer_simple3() {
    let res = fast_immediate_observer_test(|fio| {
        fio.on_next(1);
        fio.ensure_active();
        fio.on_next(2);
        fio.ensure_active();
        fio.on_next(3);
        fio.ensure_active();
        fio.on_completed();
        fio.ensure_active();
    });
    assert_eq!(
        res,
        vec![(0, Notification::OnNext(1)), (1, Notification::OnNext(2)), (2, Notification::OnNext(3)), (3, Notification::OnCompleted)]
    );
}

#[test]
fn fast_immediate_observer_fault() {
    let xs: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(Vec::new()));
    let seen = xs.clone();
    let o = Observer::create_all(
        move |x: i32| {
            seen.borrow_mut().push(x);
            if x == 2 {
                panic!("fault");
            }
        },
        |_| {},
        || {},
    );
    let fio = FastImmediateObserver::new(o);
    fio.on_next(1);
    fio.on_next(2);
    fio.on_next(3);
    let target = fio.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || target.ensure_active()));
    assert!(result.is_err());
    fio.on_next(4);
    fio.ensure_active();
    fio.on_next(2);
    fio.ensure_active();
    assert_eq!(2, xs.borrow().len());
}
