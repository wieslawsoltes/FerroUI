//! The completion of a send: the port's counterpart of the `Task` the
//! upstream `Send` returns and of the `TaskCompletionSource` behind it.
//!
//! The library has no asynchronous runtime (see the threading contract in
//! `i_transport.rs`): a task is a result that another thread may still be
//! producing, and waiting for it blocks the calling thread. The result
//! travels through a one-shot channel of `ferroui_system::task::oneshot`,
//! whose shared receiver the tasks are, waited for with its blocking
//! receives.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use ferroui_system::task::oneshot;

use crate::error::Error;

/// The result of an operation that completes on another thread. A clone is
/// another handle of the same operation.
#[derive(Clone)]
pub struct Task {
    receiver: oneshot::SharedReceiver<Result<(), Error>>,
}

impl Task {
    /// `Task.CompletedTask`.
    pub fn completed() -> Task {
        Task::from_result(Ok(()))
    }

    /// A task that is already completed with the given result.
    pub fn from_result(result: Result<(), Error>) -> Task {
        let (sender, receiver) = oneshot::channel();
        let task = Task { receiver: receiver.shared() };
        // The task keeps a receiver: the result cannot come back.
        let _ = sender.send(result);
        task
    }

    /// `Task.IsCompleted`.
    pub fn is_completed(&self) -> bool {
        self.receiver.is_finished()
    }

    /// `Task.Wait()`: blocks until the operation is finished and returns its
    /// result.
    pub fn wait(&self) -> Result<(), Error> {
        self.receiver.blocking_recv().unwrap_or_else(|_| Err(Task::canceled()))
    }

    /// `Task.Wait(timeout)`: `None` when the operation is not finished after
    /// the given time.
    pub fn wait_timeout(&self, timeout: Duration) -> Option<Result<(), Error>> {
        self.receiver.recv_timeout(timeout).unwrap_or_else(|_| Some(Err(Task::canceled())))
    }

    /// The result of a task whose every source was dropped without a result:
    /// the message of `TaskCanceledException`.
    // Deviation (DEVIATIONS.md, Remote protocol): upstream's task would never
    // complete.
    fn canceled() -> Error {
        Error::InvalidOperation("A task was canceled.".to_string())
    }
}

/// `TaskCompletionSource<int>` of the upstream code: the producing side of a
/// [`Task`]. A clone is another handle of the same source; the first result
/// set completes the task. When every clone is dropped without a result, the
/// task completes with an error (where upstream's would never complete).
#[derive(Clone)]
pub struct TaskCompletionSource {
    sender: Arc<Mutex<Option<oneshot::Sender<Result<(), Error>>>>>,
    task: Task,
}

impl TaskCompletionSource {
    pub fn new() -> TaskCompletionSource {
        let (sender, receiver) = oneshot::channel();
        TaskCompletionSource { sender: Arc::new(Mutex::new(Some(sender))), task: Task { receiver: receiver.shared() } }
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
        let sender = self.sender.lock().unwrap_or_else(PoisonError::into_inner).take();
        match sender {
            Some(sender) => {
                // The source keeps a receiver (its task): the result cannot come back.
                let _ = sender.send(value);
                true
            }
            None => false,
        }
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

    #[test]
    fn a_task_whose_sources_are_all_dropped_completes_as_canceled() {
        let source = TaskCompletionSource::new();
        let task = source.task();
        let other = source.clone();
        drop(source);
        assert!(!task.is_completed());
        let thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            drop(other);
        });
        assert!(matches!(task.wait(), Err(Error::InvalidOperation(message)) if message == "A task was canceled."));
        assert!(task.is_completed());
        assert!(matches!(task.wait_timeout(Duration::from_millis(10)), Some(Err(Error::InvalidOperation(_)))));
        thread.join().unwrap();
    }
}
