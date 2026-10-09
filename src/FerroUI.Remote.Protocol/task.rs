//! The completion of a send: the port's counterpart of the `Task` the
//! upstream `Send` returns and of the `TaskCompletionSource` behind it.
//!
//! The library has no asynchronous runtime (see the threading contract in
//! `i_transport.rs`): a task is a result that another thread may still be
//! producing, and waiting for it blocks the calling thread.

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::error::Error;

struct TaskState {
    result: Mutex<Option<Result<(), Error>>>,
    completed: Condvar,
}

/// The result of an operation that completes on another thread. A clone is
/// another handle of the same operation.
#[derive(Clone)]
pub struct Task {
    state: Arc<TaskState>,
}

impl Task {
    /// `Task.CompletedTask`.
    pub fn completed() -> Task {
        Task::from_result(Ok(()))
    }

    /// A task that is already completed with the given result.
    pub fn from_result(result: Result<(), Error>) -> Task {
        Task { state: Arc::new(TaskState { result: Mutex::new(Some(result)), completed: Condvar::new() }) }
    }

    /// `Task.IsCompleted`.
    pub fn is_completed(&self) -> bool {
        self.state.result.lock().unwrap().is_some()
    }

    /// `Task.Wait()`: blocks until the operation is finished and returns its
    /// result.
    pub fn wait(&self) -> Result<(), Error> {
        let mut result = self.state.result.lock().unwrap();
        loop {
            if let Some(result) = result.as_ref() {
                return result.clone();
            }
            result = self.state.completed.wait(result).unwrap();
        }
    }

    /// `Task.Wait(timeout)`: `None` when the operation is not finished after
    /// the given time.
    pub fn wait_timeout(&self, timeout: Duration) -> Option<Result<(), Error>> {
        let result = self.state.result.lock().unwrap();
        let (result, _) = self.state.completed.wait_timeout_while(result, timeout, |result| result.is_none()).unwrap();
        result.clone()
    }
}

/// `TaskCompletionSource<int>` of the upstream code: the producing side of a
/// [`Task`].
#[derive(Clone)]
pub struct TaskCompletionSource {
    task: Task,
}

impl TaskCompletionSource {
    pub fn new() -> TaskCompletionSource {
        TaskCompletionSource {
            task: Task { state: Arc::new(TaskState { result: Mutex::new(None), completed: Condvar::new() }) },
        }
    }

    /// `TaskCompletionSource.Task`.
    pub fn task(&self) -> Task {
        self.task.clone()
    }

    /// `TrySetResult`: false when the task was already completed.
    pub fn try_set_result(&self) -> bool {
        self.try_complete(Ok(()))
    }

    /// `TrySetException`: false when the task was already completed.
    pub fn try_set_exception(&self, error: Error) -> bool {
        self.try_complete(Err(error))
    }

    fn try_complete(&self, value: Result<(), Error>) -> bool {
        let mut result = self.task.state.result.lock().unwrap();
        if result.is_some() {
            return false;
        }
        *result = Some(value);
        self.task.state.completed.notify_all();
        true
    }
}

impl Default for TaskCompletionSource {
    fn default() -> Self {
        TaskCompletionSource::new()
    }
}

// Tests of the port: the upstream types are the ones of the runtime library.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_completed_task_returns_its_result() {
        assert!(Task::completed().is_completed());
        assert!(Task::completed().wait().is_ok());
        assert!(matches!(Task::from_result(Err(Error::EndOfStream)).wait(), Err(Error::EndOfStream)));
    }

    #[test]
    fn a_task_completes_once() {
        let source = TaskCompletionSource::new();
        let task = source.task();
        assert!(!task.is_completed());
        assert!(task.wait_timeout(Duration::from_millis(10)).is_none());
        assert!(source.try_set_result());
        assert!(!source.try_set_exception(Error::EndOfStream));
        assert!(task.wait().is_ok());
    }

    #[test]
    fn waiting_ends_when_another_thread_completes_the_task() {
        let source = TaskCompletionSource::new();
        let task = source.task();
        let thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            source.try_set_exception(Error::EndOfStream);
        });
        assert!(matches!(task.wait(), Err(Error::EndOfStream)));
        thread.join().unwrap();
    }
}
