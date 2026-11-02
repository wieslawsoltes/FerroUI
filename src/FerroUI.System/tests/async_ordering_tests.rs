//! Characterization tests of the order in which the tasks, completion sources and cancellation
//! tokens of `threading` run their callbacks: step A0 of phase C4 of #423
//! (docs/idiomatic-rust-async.md, section 2). They pin what C4 has to keep, or to change on
//! purpose with a DEVIATIONS row and a new test.

use crate::threading::tasks::{run_pending_wakes, spawn_local, TaskCompletionSource, TaskStatus};
use crate::threading::CancellationTokenSource;
use crate::ObservableError;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn a_completion_runs_the_continuations_then_resumes_the_awaiting_futures_inside_the_call() {
    let source = TaskCompletionSource::<i32>::new();
    let log = Rc::new(RefCell::new(Vec::<String>::new()));

    let mut futures = Vec::new();
    for name in ["first future", "second future"] {
        let (l, awaited) = (log.clone(), source.task());
        futures.push(spawn_local(async move {
            let value = awaited.await?;
            l.borrow_mut().push(format!("{name}: {value}"));
            Ok::<_, ObservableError>(())
        }));
        let l = log.clone();
        let continuation = name.replace("future", "continuation");
        let _ = source.task().continue_with(move |task| {
            l.borrow_mut().push(format!("{continuation}: {}", task.result().unwrap()));
        });
    }

    log.borrow_mut().push("completing".to_string());
    source.set_result(5);
    log.borrow_mut().push("completed".to_string());

    // Every continuation first, in the order of registration, then the futures in the order they
    // awaited, each polled again at once: all of it inside `set_result`.
    assert_eq!(
        *log.borrow(),
        [
            "completing",
            "first continuation: 5",
            "second continuation: 5",
            "first future: 5",
            "second future: 5",
            "completed"
        ]
    );
    assert!(futures.iter().all(|task| task.status() == TaskStatus::RanToCompletion));
}

#[test]
fn a_future_awaiting_a_dropped_completion_source_stays_pending() {
    let source = TaskCompletionSource::<i32>::new();
    let awaited = source.task();
    let waiting = spawn_local(async move { awaited.await });
    drop(source);
    // Dropping the producer side ends nothing: the task is not canceled.
    assert_eq!(waiting.status(), TaskStatus::Running);
    assert!(!waiting.is_completed());
}

#[test]
fn a_cancellation_runs_its_callbacks_once_in_registration_order_on_the_cancelling_thread() {
    let source = CancellationTokenSource::new();
    let token = source.token();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut registrations = Vec::new();
    for n in 0..4 {
        let calls = calls.clone();
        registrations.push(token.register(move || calls.lock().unwrap().push((n, thread::current().id()))));
    }
    // Removing one keeps the order of the others.
    registrations[1].dispose();

    let canceller = thread::spawn(move || {
        source.cancel();
        source.cancel();
        thread::current().id()
    })
    .join()
    .unwrap();
    assert!(token.is_cancellation_requested());
    assert_eq!(*calls.lock().unwrap(), [(0, canceller), (2, canceller), (3, canceller)]);

    // A callback registered after the cancellation runs at once, on the thread that registers it.
    let c = calls.clone();
    let _late = token.register(move || c.lock().unwrap().push((4, thread::current().id())));
    assert_eq!(calls.lock().unwrap().last(), Some(&(4, thread::current().id())));
}

#[test]
fn callbacks_that_other_threads_send_to_a_thread_run_in_the_order_they_arrived() {
    let sources: Vec<CancellationTokenSource> = (0..3).map(|_| CancellationTokenSource::new()).collect();
    let log = Rc::new(RefCell::new(Vec::new()));
    let _registrations: Vec<_> = sources
        .iter()
        .enumerate()
        .map(|(n, source)| {
            let l = log.clone();
            source.token().register_local(move || l.borrow_mut().push(n))
        })
        .collect();

    let remote = sources.clone();
    thread::spawn(move || {
        for n in [2, 0, 1] {
            remote[n].cancel();
        }
    })
    .join()
    .unwrap();
    // Without a poster, they wait for the thread to run its queue.
    assert!(log.borrow().is_empty());
    assert!(run_pending_wakes());
    assert_eq!(*log.borrow(), [2, 0, 1]);
}
