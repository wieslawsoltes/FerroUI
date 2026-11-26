//! Tests of `threading::tasks`: `run` (`Task.Run`), whose outcome ends on the thread that started
//! it, and the wakes and local callbacks that other threads send to a thread. The tasks,
//! completion sources and wrappers over `crate::task` that the module had were deprecated in 0.5.0
//! and removed in 0.7.0 (#625), with their tests.

use crate::task::{block_on, run_pending, set_thread_schedule, Schedule};
use crate::threading::tasks::run;
use crate::ObservableError;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// A schedule that keeps the work it is given, for the test to run.
struct Posted(Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>>);

impl Schedule for Posted {
    fn schedule(&self, poll: Box<dyn FnOnce() + Send>) {
        self.0.lock().unwrap().push(poll);
    }
}

#[test]
fn a_wake_from_another_thread_waits_for_run_pending() {
    let task = run(|| Ok(5));
    // Without a schedule the wake waits in the queue of the thread.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !task.is_finished() {
        run_pending();
        assert!(std::time::Instant::now() < deadline, "the work did not come back");
        std::thread::yield_now();
    }
    assert_eq!(task.result().unwrap().unwrap(), 5);
}

#[test]
fn a_wake_from_another_thread_goes_through_the_schedule_of_the_thread() {
    let posted: Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>> = Arc::new(Mutex::new(Vec::new()));
    set_thread_schedule(Some(Arc::new(Posted(posted.clone()))));

    // The work waits until `run` has polled the task once, so that its end comes as a wake from
    // the other thread; without the gate, work that ends before that poll completes the task with
    // no wake at all.
    let (release, gate) = std::sync::mpsc::channel::<()>();
    let task = run(move || {
        let _ = gate.recv();
        Ok("done".to_string())
    });
    release.send(()).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let work = posted.lock().unwrap().pop();
        if let Some(work) = work {
            work();
            break;
        }
        assert!(std::time::Instant::now() < deadline, "nothing was posted");
        std::thread::yield_now();
    }
    set_thread_schedule(None);
    assert_eq!(task.result().unwrap().unwrap(), "done");
}

#[test]
fn run_faults_with_the_error_or_the_panic_of_the_work() {
    let failed = block_on(run::<i32>(|| Err("no".into())).flatten());
    assert_eq!(failed.unwrap_err().to_string(), "no");

    let panicked = block_on(run::<i32>(|| panic!("boom")).flatten());
    assert_eq!(panicked.unwrap_err().to_string(), "boom");
}

#[test]
fn block_on_waits_for_work_of_other_threads() {
    let value = block_on(async {
        let a = run(|| Ok(20)).flatten().await?;
        let b = run(|| Ok(22)).flatten().await?;
        Ok::<_, ObservableError>(a + b)
    });
    assert_eq!(value.unwrap(), 42);
}

#[test]
fn register_local_runs_on_the_thread_that_registered() {
    use crate::threading::CancellationToken;

    // Canceled on this thread: at once.
    let source = CancellationToken::new();
    let ran = Rc::new(Cell::new(0));
    let r = ran.clone();
    let _registration = source.register_local(move || r.set(r.get() + 1));
    source.cancel();
    assert_eq!(ran.get(), 1);

    // Canceled already: at once.
    let r = ran.clone();
    let _registration = source.register_local(move || r.set(r.get() + 1));
    assert_eq!(ran.get(), 2);

    // Disposed: never.
    let source = CancellationToken::new();
    let r = ran.clone();
    let registration = source.register_local(move || r.set(r.get() + 100));
    registration.dispose();
    source.cancel();
    assert_eq!(ran.get(), 2);

    // Canceled on another thread: when this thread runs its pending wakes.
    let source = CancellationToken::new();
    let r = ran.clone();
    let _registration = source.register_local(move || r.set(r.get() + 10));
    let remote = source.clone();
    std::thread::spawn(move || remote.cancel()).join().unwrap();
    assert_eq!(ran.get(), 2);
    assert!(run_pending());
    assert_eq!(ran.get(), 12);
}
