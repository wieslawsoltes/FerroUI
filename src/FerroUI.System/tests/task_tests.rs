//! Tests of `threading::tasks`: tasks, completion sources, and futures run on their thread.

use crate::threading::tasks::{block_on, run, run_pending_wakes, set_thread_poster, Task, TaskCanceledError, TaskCompletionSource, TaskStatus, ThreadPoster};
use crate::ObservableError;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct TestError(&'static str);

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for TestError {}

fn error(text: &'static str) -> ObservableError {
    Rc::new(TestError(text))
}

#[test]
fn completed_tasks() {
    let ok = Task::from_result(42);
    assert_eq!(ok.status(), TaskStatus::RanToCompletion);
    assert!(ok.is_completed() && ok.is_completed_successfully());
    assert_eq!(ok.result().unwrap(), 42);

    let ex = error("ex");
    let faulted = Task::<i32>::from_exception(ex.clone());
    assert!(faulted.is_faulted());
    assert!(Rc::ptr_eq(&faulted.exception().unwrap(), &ex));
    assert!(Rc::ptr_eq(&faulted.result().unwrap_err(), &ex));

    let canceled = Task::<i32>::from_canceled();
    assert!(canceled.is_canceled());
    assert!(canceled.exception().is_none());
    assert!(TaskCanceledError::is_cancellation(&canceled.result().unwrap_err()));
}

#[test]
fn a_cancellation_error_cancels() {
    let task = Task::<i32>::from_exception(Rc::new(TaskCanceledError::default()));
    assert_eq!(task.status(), TaskStatus::Canceled);
}

#[test]
fn completion_source_runs_continuations_synchronously_in_order() {
    let source = TaskCompletionSource::new();
    let task = source.task();
    assert_eq!(task.status(), TaskStatus::WaitingForActivation);

    let log = Rc::new(RefCell::new(Vec::new()));
    for n in 0..3 {
        let log = log.clone();
        task.continue_with(move |t| log.borrow_mut().push((n, t.result().unwrap())));
    }
    assert!(log.borrow().is_empty());

    source.set_result(7);
    assert_eq!(*log.borrow(), vec![(0, 7), (1, 7), (2, 7)]);

    // A continuation of an ended task runs at once.
    let log2 = log.clone();
    task.continue_with(move |t| log2.borrow_mut().push((3, t.result().unwrap())));
    assert_eq!(log.borrow().last(), Some(&(3, 7)));
}

#[test]
fn completion_source_ends_once() {
    let source = TaskCompletionSource::new();
    assert!(source.try_set_result(1));
    assert!(!source.try_set_result(2));
    assert!(!source.try_set_exception(error("ex")));
    assert!(!source.try_set_canceled());
    assert_eq!(source.task().result().unwrap(), 1);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| source.set_result(3))).is_err());
}

#[test]
#[should_panic]
fn result_of_a_running_task_panics() {
    let source = TaskCompletionSource::<i32>::new();
    let _ = source.task().result();
}

#[test]
fn spawn_local_runs_until_the_first_await() {
    let source = TaskCompletionSource::new();
    let steps = Rc::new(Cell::new(0));
    let (task_of_source, s) = (source.task(), steps.clone());
    let task = Task::spawn_local(async move {
        s.set(1);
        let value = task_of_source.await?;
        s.set(2);
        Ok(value * 2)
    });
    assert_eq!(steps.get(), 1);
    assert_eq!(task.status(), TaskStatus::Running);

    // Ending the awaited task polls the future again, on the spot.
    source.set_result(21);
    assert_eq!(steps.get(), 2);
    assert_eq!(task.result().unwrap(), 42);
}

#[test]
fn spawn_local_propagates_errors_and_cancellations() {
    let ex = error("ex");
    let source = TaskCompletionSource::<i32>::new();
    let awaited = source.task();
    let task = Task::spawn_local(async move { awaited.await });
    source.set_exception(ex.clone());
    assert!(task.is_faulted());
    assert!(Rc::ptr_eq(&task.exception().unwrap(), &ex));

    let source = TaskCompletionSource::<i32>::new();
    let awaited = source.task();
    let task = Task::spawn_local(async move { awaited.await });
    source.set_canceled();
    assert!(task.is_canceled());
}

#[test]
fn a_wake_from_inside_the_poll_polls_again_after_it() {
    // The future wakes itself while it is polled: the executor polls it again once the poll returns.
    let polls = Rc::new(Cell::new(0));
    let p = polls.clone();
    let task = Task::spawn_local(std::future::poll_fn(move |context| {
        p.set(p.get() + 1);
        if p.get() < 3 {
            context.waker().wake_by_ref();
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(Ok(p.get()))
        }
    }));
    assert_eq!(task.result().unwrap(), 3);
    assert_eq!(polls.get(), 3);
}

#[test]
fn a_wake_from_another_thread_waits_for_run_pending_wakes() {
    let task = run(|| Ok(5));
    // Without a poster the wake waits in the queue of the thread.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !task.is_completed() {
        run_pending_wakes();
        assert!(std::time::Instant::now() < deadline, "the work did not come back");
        std::thread::yield_now();
    }
    assert_eq!(task.result().unwrap(), 5);
}

#[test]
fn a_wake_from_another_thread_goes_through_the_poster() {
    let posted: Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>> = Arc::new(Mutex::new(Vec::new()));
    let queue = posted.clone();
    let poster: ThreadPoster = Arc::new(move |work| queue.lock().unwrap().push(work));
    set_thread_poster(Some(poster));

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
    set_thread_poster(None);
    assert_eq!(task.result().unwrap(), "done");
}

#[test]
fn run_faults_with_the_error_or_the_panic_of_the_work() {
    let failed = block_on(run::<i32>(|| Err("no".into())));
    assert_eq!(failed.unwrap_err().to_string(), "no");

    let panicked = block_on(run::<i32>(|| panic!("boom")));
    assert_eq!(panicked.unwrap_err().to_string(), "boom");
}

#[test]
fn block_on_waits_for_work_of_other_threads() {
    let value = block_on(async {
        let a = run(|| Ok(20)).await?;
        let b = run(|| Ok(22)).await?;
        Ok(a + b)
    });
    assert_eq!(value.unwrap(), 42);
}

#[test]
fn a_removed_continuation_does_not_run() {
    let source = TaskCompletionSource::new();
    let task = source.task();
    let ran = Rc::new(Cell::new(0));
    let (a, b) = (ran.clone(), ran.clone());
    let first = task.continue_with(move |_| a.set(a.get() + 1)).unwrap();
    task.continue_with(move |_| b.set(b.get() + 10));
    assert!(task.remove_continuation(first));
    assert!(!task.remove_continuation(first));
    source.set_result(());
    assert_eq!(ran.get(), 10);
    assert!(Task::from_result(()).continue_with(|_| {}).is_none());
}

#[test]
fn register_local_runs_on_the_thread_that_registered() {
    use crate::threading::CancellationTokenSource;

    // Canceled on this thread: at once.
    let source = CancellationTokenSource::new();
    let ran = Rc::new(Cell::new(0));
    let r = ran.clone();
    let _registration = source.token().register_local(move || r.set(r.get() + 1));
    source.cancel();
    assert_eq!(ran.get(), 1);

    // Canceled already: at once.
    let r = ran.clone();
    let _registration = source.token().register_local(move || r.set(r.get() + 1));
    assert_eq!(ran.get(), 2);

    // Disposed: never.
    let source = CancellationTokenSource::new();
    let r = ran.clone();
    let registration = source.token().register_local(move || r.set(r.get() + 100));
    registration.dispose();
    source.cancel();
    assert_eq!(ran.get(), 2);

    // Canceled on another thread: when this thread runs its pending wakes.
    let source = CancellationTokenSource::new();
    let r = ran.clone();
    let _registration = source.token().register_local(move || r.set(r.get() + 10));
    let remote = source.clone();
    std::thread::spawn(move || remote.cancel()).join().unwrap();
    assert_eq!(ran.get(), 2);
    assert!(run_pending_wakes());
    assert_eq!(ran.get(), 12);
}
