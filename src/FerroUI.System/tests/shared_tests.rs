//! Tests of `task::Shared` made by hand (`Shared::channel`, `SharedSender`), its removable
//! continuations, `Shared::flatten` and `Shared::result`, the `flatten` of the receivers of a
//! `oneshot` channel, and the conversions of the deprecated `Task` and `TaskCompletionSource` to the
//! new types (step A7 of phase C4 of #423, docs/idiomatic-rust-async.md, section 3.2).

use crate::task::{block_on, oneshot, spawn_local, Canceled, JoinError, Shared};
use crate::threading::tasks::TaskWorkError;
use crate::ObservableError;
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

fn now_or_never<F: Future>(future: F) -> Option<F::Output> {
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    }
}

fn error(message: &'static str) -> ObservableError {
    Rc::new(std::io::Error::other(message))
}

#[test]
fn a_sent_shared_runs_its_continuations_then_resumes_its_awaiters_inside_the_send() {
    let (sender, shared) = Shared::<i32>::channel();
    let log = Rc::new(RefCell::new(Vec::new()));

    let (l, s) = (log.clone(), shared.clone());
    let awaiter = spawn_local(async move {
        let value = s.await.unwrap();
        l.borrow_mut().push(format!("future: {value}"));
    });
    let l = log.clone();
    assert!(shared.on_finished(move |outcome| l.borrow_mut().push(format!("continuation: {}", outcome.as_ref().unwrap()))).is_some());

    log.borrow_mut().push("sending".to_string());
    sender.send(5);
    log.borrow_mut().push("sent".to_string());

    assert_eq!(*log.borrow(), ["sending", "continuation: 5", "future: 5", "sent"]);
    assert!(awaiter.is_finished());
    assert!(shared.is_finished());

    // Once ended, a continuation runs at once and has no id.
    let l = log.clone();
    assert!(shared.on_finished(move |_| l.borrow_mut().push("late".to_string())).is_none());
    assert_eq!(log.borrow().last().unwrap(), "late");
}

#[test]
fn a_removed_continuation_of_a_shared_does_not_run() {
    let (sender, shared) = Shared::<i32>::channel();
    let ran = Rc::new(RefCell::new(Vec::new()));
    let (a, b) = (ran.clone(), ran.clone());
    let first = shared.on_finished(move |_| a.borrow_mut().push("first")).unwrap();
    shared.on_finished(move |_| b.borrow_mut().push("second"));

    assert!(shared.remove_on_finished(first));
    assert!(!shared.remove_on_finished(first));
    sender.send(1);
    assert_eq!(*ran.borrow(), ["second"]);
}

#[test]
fn a_dropped_shared_sender_leaves_the_shared_pending_and_a_canceled_one_ends_it() {
    let (sender, shared) = Shared::<i32>::channel();
    drop(sender);
    assert!(!shared.is_finished());
    assert!(now_or_never(shared).is_none());

    let (sender, shared) = Shared::<i32>::channel();
    sender.cancel();
    assert!(matches!(shared.outcome(), Some(Err(JoinError::Canceled))));
}

#[test]
fn a_ready_shared_has_ended() {
    let shared = Shared::ready(3);
    assert!(shared.is_finished());
    assert_eq!(now_or_never(shared).unwrap().unwrap(), 3);
}

#[test]
fn the_flatten_of_a_shared_result_gives_the_result_and_the_errors_of_the_task_as_errors() {
    let (sender, shared) = Shared::<Result<i32, ObservableError>>::channel();
    assert!(shared.result().is_none());
    sender.send(Ok(7));
    assert_eq!(shared.result().unwrap().unwrap(), 7);
    assert_eq!(block_on(shared.flatten()).unwrap(), 7);

    let failure = error("no");
    let (sender, shared) = Shared::<Result<i32, ObservableError>>::channel();
    sender.send(Err(failure.clone()));
    let got = block_on(shared.flatten()).unwrap_err();
    assert!(Rc::ptr_eq(&got, &failure));

    let (sender, shared) = Shared::<Result<i32, ObservableError>>::channel();
    sender.cancel();
    assert!(Canceled::is_cancellation(&shared.result().unwrap().unwrap_err()));

    let panicked = spawn_local(async {
        if true {
            panic!("boom");
        }
        Ok::<i32, ObservableError>(1)
    })
    .shared();
    let error = panicked.result().unwrap().unwrap_err();
    assert_eq!(error.downcast_ref::<TaskWorkError>().unwrap().message, "boom");
}

#[test]
fn the_flatten_of_a_oneshot_receiver_gives_the_sent_result_or_a_cancellation() {
    let (sender, receiver) = oneshot::channel::<Result<i32, ObservableError>>();
    sender.send(Ok(4)).unwrap();
    assert_eq!(block_on(receiver.flatten()).unwrap(), 4);

    let (sender, receiver) = oneshot::channel::<Result<i32, ObservableError>>();
    drop(sender);
    assert!(Canceled::is_cancellation(&block_on(receiver.flatten()).unwrap_err()));

    let (sender, receiver) = oneshot::channel::<Result<i32, ObservableError>>();
    let shared = receiver.shared();
    let (first, second) = (shared.clone(), shared);
    sender.send(Ok(9)).unwrap();
    assert_eq!(block_on(first.flatten()).unwrap(), 9);
    assert_eq!(block_on(second.flatten()).unwrap(), 9);
}

#[test]
#[allow(deprecated)]
fn a_task_converts_to_the_shared_of_its_outcome() {
    use crate::threading::tasks::{TaskCanceledError, TaskCompletionSource};

    let source = TaskCompletionSource::<i32>::new();
    let shared: Shared<Result<i32, ObservableError>> = source.task().into();
    let log = Rc::new(RefCell::new(Vec::new()));
    let l = log.clone();
    source.task().continue_with(move |_| l.borrow_mut().push("task"));
    let l = log.clone();
    shared.on_finished(move |_| l.borrow_mut().push("shared"));
    source.set_result(2);
    // One list of continuations, in the order they were added.
    assert_eq!(*log.borrow(), ["task", "shared"]);
    assert_eq!(shared.result().unwrap().unwrap(), 2);

    // A canceled task keeps the error that names it.
    let source = TaskCompletionSource::<i32>::new();
    let shared: Shared<Result<i32, ObservableError>> = source.task().into();
    source.set_canceled();
    let error = shared.result().unwrap().unwrap_err();
    assert_eq!(error.downcast_ref::<TaskCanceledError>().unwrap().task_id, Some(source.task().id()));
}

#[test]
#[allow(deprecated)]
fn a_completion_source_converts_to_a_oneshot_sender_and_a_receiver_to_a_task() {
    use crate::threading::tasks::{Task, TaskCompletionSource};

    let source = TaskCompletionSource::<i32>::new();
    let sender: oneshot::Sender<Result<i32, ObservableError>> = source.clone().into();
    sender.send(Ok(3)).unwrap();
    assert_eq!(source.task().result().unwrap(), 3);

    let source = TaskCompletionSource::<i32>::new();
    let sender: oneshot::Sender<Result<i32, ObservableError>> = source.clone().into();
    drop(sender);
    assert!(source.task().is_canceled());

    let (sender, receiver) = oneshot::channel::<Result<i32, ObservableError>>();
    let task: Task<i32> = receiver.into();
    assert!(!task.is_completed());
    sender.send(Ok(8)).unwrap();
    assert_eq!(task.result().unwrap(), 8);
}
