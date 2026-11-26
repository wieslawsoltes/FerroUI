//! Characterization tests of the order in which the cancellation tokens of `threading` run their
//! callbacks: step A0 of phase C4 of #423 (docs/idiomatic-rust-async.md, section 2). They pin what
//! C4 has to keep, or to change on purpose with a DEVIATIONS row and a new test. The tests of the
//! tasks and completion sources, deprecated in 0.5.0, went with them in 0.7.0 (#625).

use crate::task::run_pending;
use crate::threading::CancellationToken;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn a_cancellation_runs_its_callbacks_once_in_registration_order_on_the_cancelling_thread() {
    let source = CancellationToken::new();
    let token = source.clone();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut registrations = Vec::new();
    for n in 0..4 {
        let calls = calls.clone();
        registrations.push(token.register(move || calls.lock().unwrap().push((n, thread::current().id()))));
    }
    // Removing one keeps the order of the others.
    registrations.remove(1).dispose();

    let canceller = thread::spawn(move || {
        source.cancel();
        source.cancel();
        thread::current().id()
    })
    .join()
    .unwrap();
    assert!(token.is_cancelled());
    assert_eq!(*calls.lock().unwrap(), [(0, canceller), (2, canceller), (3, canceller)]);

    // A callback registered after the cancellation runs at once, on the thread that registers it.
    let c = calls.clone();
    let _late = token.register(move || c.lock().unwrap().push((4, thread::current().id())));
    assert_eq!(calls.lock().unwrap().last(), Some(&(4, thread::current().id())));
}

#[test]
fn callbacks_that_other_threads_send_to_a_thread_run_in_the_order_they_arrived() {
    let tokens: Vec<CancellationToken> = (0..3).map(|_| CancellationToken::new()).collect();
    let log = Rc::new(RefCell::new(Vec::new()));
    let _registrations: Vec<_> = tokens
        .iter()
        .enumerate()
        .map(|(n, token)| {
            let l = log.clone();
            token.register_local(move || l.borrow_mut().push(n))
        })
        .collect();

    let remote = tokens.clone();
    thread::spawn(move || {
        for n in [2, 0, 1] {
            remote[n].cancel();
        }
    })
    .join()
    .unwrap();
    // Without a poster, they wait for the thread to run its queue.
    assert!(log.borrow().is_empty());
    assert!(run_pending());
    assert_eq!(*log.borrow(), [2, 0, 1]);
}
