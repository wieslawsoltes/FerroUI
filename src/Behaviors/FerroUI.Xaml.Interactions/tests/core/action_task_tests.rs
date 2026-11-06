//! Not from the reference: the task of the asynchronous actions ([`TaskHandle`]) over the handle of
//! a task of the dispatcher (step A6 of phase C4 of #423).

use crate::tests::support::{run_jobs, start_app};
use crate::{ActionTask, TaskHandle};
use ferroui_base::threading::{Dispatcher, JoinError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The message of the payload of a panic.
fn message_of(payload: &(dyn std::any::Any + Send)) -> String {
    match payload.downcast_ref::<&str>() {
        Some(message) => (*message).to_owned(),
        None => payload.downcast_ref::<String>().cloned().unwrap_or_default(),
    }
}

#[test]
fn a_task_gives_its_result_to_its_awaiter_and_unit_to_every_awaiter() {
    let _app = start_app();
    let task = TaskHandle::spawn(async { 7 });
    assert!(!task.is_completed());
    let result = Rc::new(Cell::new(None));
    {
        let (task, result) = (task.clone(), result.clone());
        drop(ActionTask::spawn(async move {
            result.set(task.await.ok());
        }));
    }
    run_jobs();
    assert!(task.is_completed_successfully());
    assert_eq!(result.get(), Some(7));

    let unit = ActionTask::spawn(async {});
    let awaited = Rc::new(Cell::new(0));
    for _ in 0..2 {
        let (unit, awaited) = (unit.clone(), awaited.clone());
        drop(ActionTask::spawn(async move {
            if unit.await.is_ok() {
                awaited.set(awaited.get() + 1);
            }
        }));
    }
    run_jobs();
    assert_eq!(awaited.get(), 2);
}

#[test]
fn a_panic_of_the_task_is_a_value_for_every_awaiter() {
    let _app = start_app();
    let task = ActionTask::spawn(async {
        panic!("the action failed");
    });
    let seen = Rc::new(RefCell::new(Vec::new()));
    for _ in 0..2 {
        let (task, seen) = (task.clone(), seen.clone());
        drop(ActionTask::spawn(async move {
            if let Err(JoinError::Panicked(payload)) = task.await {
                seen.borrow_mut().push(message_of(&*payload));
            }
        }));
    }
    run_jobs();
    assert!(task.is_faulted());
    assert_eq!(*seen.borrow(), ["the action failed", "the action failed"]);
}

#[test]
fn an_aborted_task_is_canceled() {
    let _app = start_app();
    let task = ActionTask::spawn(std::future::pending::<()>());
    run_jobs();
    assert!(!task.is_completed());

    task.abort();
    assert!(task.is_canceled());
    let canceled = Rc::new(Cell::new(false));
    {
        let (task, canceled) = (task.clone(), canceled.clone());
        drop(ActionTask::spawn(async move {
            canceled.set(matches!(task.await, Err(JoinError::Canceled)));
        }));
    }
    run_jobs();
    assert!(canceled.get());
}

#[test]
#[allow(deprecated)]
fn a_dispatcher_task_still_makes_a_task() {
    let _app = start_app();
    let task = TaskHandle::from_task(Dispatcher::ui_thread().invoke_async_task_local(|| async { 3 }));
    run_jobs();
    assert!(task.is_completed_successfully());
}
