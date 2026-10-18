//! Ported from `Tests.System.Reactive/Tests/Concurrency/AsyncLockTest.cs` of Rx.NET.
//! Adapted: a thrown exception is a panic with a marker payload, caught with `catch_unwind`;
//! `Assert.True(false)` inside an action is a panic; the lock is shared through an `Rc` where an
//! action queues more work on it.

use ferroui_reactive::concurrency::AsyncLock;
use std::cell::Cell;
use std::panic::{catch_unwind, panic_any, AssertUnwindSafe};
use std::rc::Rc;

/// The exception thrown by the action of `wait_fail`.
struct Ex;

#[test]
fn wait_graceful() {
    let ok = Rc::new(Cell::new(false));
    let o = ok.clone();
    AsyncLock::new().wait(move || o.set(true));
    assert!(ok.get());
}

#[test]
fn wait_fail() {
    let l = AsyncLock::new();

    let result = catch_unwind(AssertUnwindSafe(|| {
        l.wait(|| panic_any(Ex));
    }));
    match result {
        Ok(()) => panic!("the action should have thrown"),
        Err(e) => assert!(e.downcast_ref::<Ex>().is_some()),
    }

    // has faulted; should not run
    l.wait(|| panic!("the lock has faulted; the action should not run"));
}

#[test]
fn wait_queues_work() {
    let l = Rc::new(AsyncLock::new());

    let l1 = Rc::new(Cell::new(false));
    let l2 = Rc::new(Cell::new(false));
    {
        let lock = l.clone();
        let (l1, l2) = (l1.clone(), l2.clone());
        l.wait(move || {
            let inner_l1 = l1.clone();
            lock.wait(move || {
                assert!(inner_l1.get());
                l2.set(true);
            });
            l1.set(true);
        });
    }
    assert!(l2.get());
}

#[test]
fn dispose() {
    let l = Rc::new(AsyncLock::new());

    let l1 = Rc::new(Cell::new(false));
    let l2 = Rc::new(Cell::new(false));
    let l3 = Rc::new(Cell::new(false));
    let l4 = Rc::new(Cell::new(false));

    {
        let lock = l.clone();
        let (l1, l2, l3, l4) = (l1.clone(), l2.clone(), l3.clone(), l4.clone());
        l.wait(move || {
            let lock2 = lock.clone();
            lock.wait(move || {
                lock2.wait(move || {
                    l3.set(true);
                });

                l2.set(true);

                lock2.dispose();

                lock2.wait(move || {
                    l4.set(true);
                });
            });

            l1.set(true);
        });
    }

    assert!(l1.get());
    assert!(l2.get());
    assert!(!l3.get());
    assert!(!l4.get());
}

// Not ported: Wait_ArgumentChecking - argument validation
