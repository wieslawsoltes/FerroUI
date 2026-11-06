use ferroui_reactive::concurrency::{IScheduler, ImmediateScheduler};
use ferroui_reactiveui::RxSchedulers;
use ferroui_system::task::{set_current_schedule, Schedule};
use std::rc::Rc;
use std::sync::Arc;

/// The environment of a test of the services: no current schedule (the synchronization context
/// of the reference), and the immediate scheduler as the Rx main-thread scheduler; both are
/// restored when it is dropped.
pub(crate) struct Environment {
    schedule: Option<Arc<dyn Schedule>>,
    scheduler: Rc<dyn IScheduler>,
}
impl Environment {
    pub(crate) fn new() -> Self {
        let previous = Self {
            schedule: set_current_schedule(None),
            scheduler: RxSchedulers::main_thread_scheduler(),
        };
        RxSchedulers::set_main_thread_scheduler(ImmediateScheduler::instance());
        previous
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        drop(set_current_schedule(self.schedule.take()));
        RxSchedulers::set_main_thread_scheduler(self.scheduler.clone());
    }
}

/// The states of a task that the tests assert (`IsCompleted`, `IsCompletedSuccessfully`,
/// `IsFaulted`, `IsCanceled`), on the `Shared` of its result.
pub(crate) trait TaskState {
    fn is_completed(&self) -> bool;
    fn is_completed_successfully(&self) -> bool;
    fn is_faulted(&self) -> bool;
    fn is_canceled(&self) -> bool;
    fn exception(&self) -> Option<ferroui_system::ObservableError>;
}
impl<T: Clone + 'static> TaskState for ferroui_system::task::Shared<Result<T, ferroui_system::ObservableError>> {
    fn is_completed(&self) -> bool {
        self.is_finished()
    }
    fn is_completed_successfully(&self) -> bool {
        matches!(self.result(), Some(Ok(_)))
    }
    fn is_faulted(&self) -> bool {
        matches!(self.result(), Some(Err(error)) if !ferroui_system::task::Canceled::is_cancellation(&error))
    }
    fn is_canceled(&self) -> bool {
        matches!(self.result(), Some(Err(error)) if ferroui_system::task::Canceled::is_cancellation(&error))
    }
    fn exception(&self) -> Option<ferroui_system::ObservableError> {
        match self.result() {
            Some(Err(error)) if !ferroui_system::task::Canceled::is_cancellation(&error) => Some(error),
            _ => None,
        }
    }
}

/// A task that has ended with success (`Task.CompletedTask`).
pub(crate) fn completed_task() -> ferroui_system::task::Shared<Result<(), ferroui_system::ObservableError>> {
    ferroui_system::task::Shared::ready(Ok(()))
}

/// A task that has ended with `error` (`Task.FromException`).
pub(crate) fn faulted_task(error: ferroui_system::ObservableError) -> ferroui_system::task::Shared<Result<(), ferroui_system::ObservableError>> {
    ferroui_system::task::Shared::ready(Err(error))
}
