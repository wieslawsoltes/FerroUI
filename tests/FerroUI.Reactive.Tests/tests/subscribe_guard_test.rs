//! The guard forms of the subscription methods (`subscribe_guard` of `SubscribeGuard`, the
//! `_guard` siblings of `subscribe_next` and its family): not upstream, step D5 of the disposal
//! plan (`docs/idiomatic-rust-disposal.md`).

use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactive::{AnonymousObserver, IObserver, ObservableExt, SubscribeGuard, Subscription};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn subscribe_guard_unsubscribes_on_drop() {
    let s = Subject::<i32>::new();
    let seen = Rc::new(Cell::new(0));
    let observed = seen.clone();
    let guard = s
        .as_observable()
        .subscribe_guard(Rc::new(AnonymousObserver::new(move |x: i32| observed.set(observed.get() + x), |_| {}, || {})));
    assert!(s.has_observers());
    s.on_next(2);
    assert_eq!(seen.get(), 2);
    drop(guard);
    assert!(!s.has_observers());
    s.on_next(3);
    assert_eq!(seen.get(), 2);
}

#[test]
fn subscribe_next_guards_unsubscribe_on_drop() {
    let s = Subject::<i32>::new();
    let guards: Vec<Subscription> = vec![
        s.as_observable().subscribe_next_guard(|_| {}),
        s.as_observable().subscribe_next_error_guard(|_| {}, |_| {}),
        s.as_observable().subscribe_next_completed_guard(|_| {}, || {}),
        s.as_observable().subscribe_all_guard(|_| {}, |_| {}, || {}),
    ];
    assert!(s.has_observers());
    drop(guards);
    assert!(!s.has_observers());
}

#[test]
fn a_detached_guard_keeps_the_subscription() {
    let s = Subject::<i32>::new();
    let handle = s.as_observable().subscribe_next_guard(|_| {}).detach();
    assert!(s.has_observers());
    drop(handle.clone());
    assert!(s.has_observers());
    handle.dispose();
    assert!(!s.has_observers());
}
