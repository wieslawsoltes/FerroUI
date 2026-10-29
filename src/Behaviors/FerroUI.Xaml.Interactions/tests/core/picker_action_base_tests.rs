//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/PickerActionBaseTests.cs` of
//! Xaml.Behaviors.
//!
//! Adapted: the tasks are the tasks of the dispatcher of the user interface ([`ActionTask`]); an
//! awaited task is one that completed after the jobs of the dispatcher ran, and a task that throws
//! is a task whose future panicked. The completion source of the reference is a future that the
//! test completes. The operation that the reference dispatches from a thread of the thread pool
//! is dispatched from the thread of the user interface (the operations of the port use the objects
//! of that thread, see `track_dispatched_picker_operation`).

use super::command::Command;
use crate::tests::support::{boxed, run_jobs, start_app};
use crate::{ActionTask, OpenFilePickerAction, OpenFolderPickerAction, PickerActionBase, SaveFilePickerAction};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::Border;
use ferroui_xaml_interactivity::StyledElementActionImpl;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// `TestPickerAction`.
#[repr(C)]
pub(crate) struct TestPickerAction {
    base: PickerActionBase,
}

ferro_class!(TestPickerAction: PickerActionBase);
ferroui_base::ferro_impl_classes!(TestPickerAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for TestPickerAction {
    fn execute(_this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        None
    }
}

impl TestPickerAction {
    fn new() -> Ref<Self> {
        instantiate(Self { base: PickerActionBase::construct() })
    }

    fn track(&self, task: ActionTask) -> ActionTask {
        self.track_picker_operation(task)
    }

    fn track_dispatched<F, Fut>(&self, operation: F) -> ActionTask
    where
        F: FnOnce() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        self.track_dispatched_picker_operation(operation)
    }
}

/// The state of a [`Completion`]: its outcome (true when it ran to the end, false when it failed)
/// and who waits for it.
#[derive(Default)]
struct CompletionState {
    outcome: Option<bool>,
    waker: Option<Waker>,
}

/// `TaskCompletionSource`: a future the test completes.
#[derive(Clone, Default)]
struct Completion(Rc<RefCell<CompletionState>>);

impl Completion {
    fn complete(&self, outcome: bool) {
        let waker = {
            let mut state = self.0.borrow_mut();
            state.outcome = Some(outcome);
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// `SetResult()`.
    fn set_result(&self) {
        self.complete(true);
    }

    /// `SetException(new InvalidOperationException("Picker failed."))`.
    fn set_exception(&self) {
        self.complete(false);
    }
}

impl Future for Completion {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.0.borrow_mut();
        match state.outcome {
            Some(true) => Poll::Ready(()),
            Some(false) => {
                drop(state);
                panic!("Picker failed.");
            }
            None => {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// `WaitForNoActiveOperations(action)`: runs the jobs of the dispatcher until no picker operation
/// is active.
fn wait_for_no_active_operations(action: &PickerActionBase) {
    for _ in 0..20 {
        if action.active_picker_operation_count() == 0 {
            return;
        }

        run_jobs();
    }

    assert_eq!(0, action.active_picker_operation_count());
}

/// The task an action returned (`Assert.IsAssignableFrom<Task>(result)`), awaited.
fn await_task(result: Option<BoxedValue>) -> ActionTask {
    let task = result.and_then(|result| result.downcast_ref::<ActionTask>().cloned()).expect("a task");
    run_jobs();
    assert!(task.is_completed_successfully());
    task
}

#[test]
fn track_picker_operation_keeps_operation_active_until_completion() {
    let _app = start_app();
    let action = TestPickerAction::new();
    let completion = Completion::default();

    let task = action.track(ActionTask::spawn(completion.clone()));

    assert_eq!(1, action.active_picker_operation_count());

    completion.set_result();

    run_jobs();
    assert!(task.is_completed_successfully());
    wait_for_no_active_operations(&action);
}

#[test]
fn track_picker_operation_removes_operation_when_provider_fails() {
    let _app = start_app();
    let action = TestPickerAction::new();
    let completion = Completion::default();

    let task = action.track(ActionTask::spawn(completion.clone()));

    assert_eq!(1, action.active_picker_operation_count());

    completion.set_exception();

    run_jobs();
    assert!(task.is_faulted());
    wait_for_no_active_operations(&action);
}

#[test]
fn open_file_picker_action_returns_task_when_sender_is_visual() {
    let _app = start_app();
    let action = OpenFilePickerAction::new();
    action.set_command(Some(Command::new(|_| {})));

    let result = action.execute(Some(&boxed(&Border::new())), None);

    await_task(result);
}

#[test]
fn track_dispatched_picker_operation_runs_operation_on_ui_thread() {
    let _app = start_app();
    let action = TestPickerAction::new();
    let operation_was_on_ui_thread = Rc::new(Cell::new(false));

    let on_ui_thread = operation_was_on_ui_thread.clone();
    let task = action.track_dispatched(move || {
        on_ui_thread.set(Dispatcher::ui_thread().check_access());
        std::future::ready(())
    });

    run_jobs();
    assert!(task.is_completed_successfully());

    assert!(operation_was_on_ui_thread.get());
    wait_for_no_active_operations(&action);
}

#[test]
fn open_folder_picker_action_returns_task_when_sender_is_visual() {
    let _app = start_app();
    let action = OpenFolderPickerAction::new();
    action.set_command(Some(Command::new(|_| {})));

    let result = action.execute(Some(&boxed(&Border::new())), None);

    await_task(result);
}

#[test]
fn save_file_picker_action_returns_task_when_sender_is_visual() {
    let _app = start_app();
    let action = SaveFilePickerAction::new();
    action.set_command(Some(Command::new(|_| {})));

    let result = action.execute(Some(&boxed(&Border::new())), None);

    await_task(result);
}
