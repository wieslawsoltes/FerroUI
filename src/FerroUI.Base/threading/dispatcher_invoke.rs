use std::any::Any;
use std::fmt;
use std::future::Future;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Duration;

use super::dispatcher_operation::{box_local_callback, OperationCore, SendCallback, WaitOutcome};
use super::dispatcher_task::{wrap_local_future, wrap_send_future};
use super::{
    CancellationToken, Dispatcher, DispatcherOperation, DispatcherOperationStatus, DispatcherPriority,
    DispatcherPriorityAwaitable, DispatcherPriorityTaskAwaitable, DispatcherTask, FerroSynchronizationContext,
    OperationCanceledError,
};

/// The error of a synchronous invoke with a timeout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispatcherInvokeError {
    /// The operation was aborted before it ran: its cancellation token was
    /// canceled, or the dispatcher has shut down.
    Canceled,
    /// The operation did not start within the timeout and was aborted.
    Timeout,
}

impl fmt::Display for DispatcherInvokeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canceled => f.write_str("The dispatcher operation was canceled"),
            Self::Timeout => f.write_str("The dispatcher operation timed out"),
        }
    }
}

impl std::error::Error for DispatcherInvokeError {}

/// Synchronous execution. The callbacks of [`invoke`](Dispatcher::invoke)
/// and its variants can come from any thread and must be `Send`; the
/// `invoke_local` variants accept any callback but can only be called on the
/// dispatcher thread.
impl Dispatcher {
    /// Executes the specified callback synchronously on the thread that the
    /// dispatcher was created on, at [`DispatcherPriority::SEND`].
    ///
    /// Returns `Err` when the operation was aborted before it ran (for
    /// instance because the dispatcher has shut down).
    ///
    /// # Panics
    /// Re-raises the panic of the callback.
    pub fn invoke<R: Send + 'static>(
        &self,
        callback: impl FnOnce() -> R + Send + 'static,
    ) -> Result<R, OperationCanceledError> {
        self.invoke_with_cancellation(callback, DispatcherPriority::SEND, &CancellationToken::none())
    }

    /// Executes the specified callback synchronously with the specified
    /// priority on the thread that the dispatcher was created on.
    pub fn invoke_with_priority<R: Send + 'static>(
        &self,
        callback: impl FnOnce() -> R + Send + 'static,
        priority: DispatcherPriority,
    ) -> Result<R, OperationCanceledError> {
        self.invoke_with_cancellation(callback, priority, &CancellationToken::none())
    }

    /// Executes the specified callback synchronously with the specified
    /// priority on the thread that the dispatcher was created on.
    ///
    /// `cancellation_token` cancels the operation as long as it has not
    /// started. Returns `Err` when the token is already canceled.
    pub fn invoke_with_cancellation<R: Send + 'static>(
        &self,
        callback: impl FnOnce() -> R + Send + 'static,
        priority: DispatcherPriority,
        cancellation_token: &CancellationToken,
    ) -> Result<R, OperationCanceledError> {
        DispatcherPriority::validate(priority, "priority");

        // Fast-Path: if on the same thread, and invoking at Send priority,
        // and the cancellation token is not already canceled, then just
        // call the callback directly.
        if !cancellation_token.is_cancellation_requested() && priority == DispatcherPriority::SEND && self.check_access()
        {
            let _restore = FerroSynchronizationContext::ensure_with_dispatcher(self, priority);
            return Ok(callback());
        }

        // Slow-Path: go through the queue.
        let operation = DispatcherOperation::new_send(&self.to_arc(), priority, callback);
        self.invoke_impl(&operation, cancellation_token, None).map_err(|_| OperationCanceledError)
    }

    /// Executes the specified callback synchronously with the specified
    /// priority on the thread that the dispatcher was created on, giving up
    /// when the operation has not started within `timeout`.
    ///
    /// The timeout only applies to the time the operation spends in the
    /// queue: once it has started, the call waits for it to complete. When
    /// the operation is still pending after `timeout` it is aborted and
    /// [`DispatcherInvokeError::Timeout`] is returned.
    ///
    /// On the dispatcher thread the queue is pumped while waiting and the
    /// timeout is measured with the dispatcher clock.
    pub fn invoke_with_timeout<R: Send + 'static>(
        &self,
        callback: impl FnOnce() -> R + Send + 'static,
        priority: DispatcherPriority,
        cancellation_token: &CancellationToken,
        timeout: Duration,
    ) -> Result<R, DispatcherInvokeError> {
        DispatcherPriority::validate(priority, "priority");

        // Fast-Path, see invoke_with_cancellation.
        if !cancellation_token.is_cancellation_requested() && priority == DispatcherPriority::SEND && self.check_access()
        {
            let _restore = FerroSynchronizationContext::ensure_with_dispatcher(self, priority);
            return Ok(callback());
        }

        // Slow-Path: go through the queue.
        let operation = DispatcherOperation::new_send(&self.to_arc(), priority, callback);
        self.invoke_impl(&operation, cancellation_token, Some(timeout))
    }

    /// Like [`invoke`](Self::invoke) for callbacks that are not `Send`.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn invoke_local<R: 'static>(&self, callback: impl FnOnce() -> R + 'static) -> Result<R, OperationCanceledError> {
        self.invoke_local_with_cancellation(callback, DispatcherPriority::SEND, &CancellationToken::none())
    }

    /// Like [`invoke_with_priority`](Self::invoke_with_priority) for
    /// callbacks that are not `Send`.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn invoke_local_with_priority<R: 'static>(
        &self,
        callback: impl FnOnce() -> R + 'static,
        priority: DispatcherPriority,
    ) -> Result<R, OperationCanceledError> {
        self.invoke_local_with_cancellation(callback, priority, &CancellationToken::none())
    }

    /// Like [`invoke_with_cancellation`](Self::invoke_with_cancellation) for
    /// callbacks that are not `Send`.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn invoke_local_with_cancellation<R: 'static>(
        &self,
        callback: impl FnOnce() -> R + 'static,
        priority: DispatcherPriority,
        cancellation_token: &CancellationToken,
    ) -> Result<R, OperationCanceledError> {
        self.verify_access();
        DispatcherPriority::validate(priority, "priority");

        // Fast-Path, see invoke_with_cancellation.
        if !cancellation_token.is_cancellation_requested() && priority == DispatcherPriority::SEND {
            let _restore = FerroSynchronizationContext::ensure_with_dispatcher(self, priority);
            return Ok(callback());
        }

        // Slow-Path: go through the queue.
        let operation = DispatcherOperation::new_local(&self.to_arc(), priority, callback);
        self.invoke_impl(&operation, cancellation_token, None).map_err(|_| OperationCanceledError)
    }

    /// Like [`invoke_with_timeout`](Self::invoke_with_timeout) for callbacks
    /// that are not `Send`.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn invoke_local_with_timeout<R: 'static>(
        &self,
        callback: impl FnOnce() -> R + 'static,
        priority: DispatcherPriority,
        cancellation_token: &CancellationToken,
        timeout: Duration,
    ) -> Result<R, DispatcherInvokeError> {
        self.verify_access();
        DispatcherPriority::validate(priority, "priority");

        // Fast-Path, see invoke_with_cancellation.
        if !cancellation_token.is_cancellation_requested() && priority == DispatcherPriority::SEND {
            let _restore = FerroSynchronizationContext::ensure_with_dispatcher(self, priority);
            return Ok(callback());
        }

        // Slow-Path: go through the queue.
        let operation = DispatcherOperation::new_local(&self.to_arc(), priority, callback);
        self.invoke_impl(&operation, cancellation_token, Some(timeout))
    }

    fn invoke_impl<R: 'static>(
        &self,
        operation: &DispatcherOperation<R>,
        cancellation_token: &CancellationToken,
        timeout: Option<Duration>,
    ) -> Result<R, DispatcherInvokeError> {
        if cancellation_token.is_cancellation_requested() {
            operation.core().set_status(DispatcherOperationStatus::Aborted);
            operation.core().call_abort_callbacks();
            return Err(DispatcherInvokeError::Canceled);
        }

        debug_assert!(operation.priority() != DispatcherPriority::SEND || !self.check_access()); // should be handled by caller

        // This operation must be queued since it was invoked either to
        // another thread, or at a priority other than Send.
        self.invoke_async_impl(operation.core(), cancellation_token);

        // We have already registered with the cancellation token to abort
        // the operation when it is canceled, and the wait below aborts it
        // when the timeout expires while it is still pending.  If the
        // operation has already started when the timeout expires, we still
        // wait for it to complete.  This is different than simply waiting on
        // the operation with a timeout because we are the ones queuing the
        // dispatcher operation, not the caller.  We can't leave the operation
        // in a state that it might execute if we return that it did not
        // invoke.
        match operation.core().wait_with_limit(timeout, true) {
            // The operation was aborted because of the timeout.
            WaitOutcome::TimedOut => Err(DispatcherInvokeError::Timeout),
            // Completed, or canceled for some other reason.
            WaitOutcome::Finished => operation.take_result().map_err(|_| DispatcherInvokeError::Canceled),
        }
    }

    /// Runs `action` on the dispatcher thread and waits for it. A panic of
    /// the action goes through the unhandled-exception events of the
    /// dispatcher and is re-raised on the dispatcher thread when it is not
    /// handled there.
    ///
    /// `None` stands for [`DispatcherPriority::SEND`].
    pub(crate) fn send(&self, action: impl FnOnce() + Send + 'static, priority: Option<DispatcherPriority>) {
        let priority = priority.unwrap_or(DispatcherPriority::SEND);

        if priority == DispatcherPriority::SEND && self.check_access() {
            self.send_inline(action, priority);
        } else {
            let callback: SendCallback = Box::new(move || {
                action();
                Box::new(()) as Box<dyn Any + Send>
            });
            let operation = OperationCore::new_send(&self.to_arc(), priority, true, callback);
            self.invoke_async_impl(&operation, &CancellationToken::none());
            operation.wait();
        }
    }

    /// Like [`send`](Self::send) for actions that are not `Send`.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub(crate) fn send_local(&self, action: impl FnOnce() + 'static, priority: Option<DispatcherPriority>) {
        self.verify_access();
        let priority = priority.unwrap_or(DispatcherPriority::SEND);

        if priority == DispatcherPriority::SEND {
            self.send_inline(action, priority);
        } else {
            let operation = OperationCore::new_local(&self.to_arc(), priority, true, box_local_callback(action));
            self.invoke_async_impl(&operation, &CancellationToken::none());
            operation.wait();
        }
    }

    fn send_inline(&self, action: impl FnOnce(), priority: DispatcherPriority) {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _restore = FerroSynchronizationContext::ensure_with_dispatcher(self, priority);
            action();
        }));
        if let Err(payload) = result {
            if !self.try_catch_when(&payload) {
                self.forget_exception_if_outermost(&payload);
                resume_unwind(payload);
            }
        }
    }
}

/// Asynchronous execution; see [`Dispatcher`] for the `Send`/local split.
impl Dispatcher {
    /// Executes the specified callback asynchronously on the thread that the
    /// dispatcher was created on, at [`DispatcherPriority::DEFAULT`].
    ///
    /// The returned operation can be awaited, waited for, aborted and
    /// re-prioritized.
    pub fn invoke_async<R: Send + 'static>(
        &self,
        callback: impl FnOnce() -> R + Send + 'static,
    ) -> DispatcherOperation<R> {
        self.invoke_async_with_cancellation(callback, DispatcherPriority::DEFAULT, &CancellationToken::none())
    }

    /// Executes the specified callback asynchronously with the specified
    /// priority on the thread that the dispatcher was created on.
    pub fn invoke_async_with_priority<R: Send + 'static>(
        &self,
        callback: impl FnOnce() -> R + Send + 'static,
        priority: DispatcherPriority,
    ) -> DispatcherOperation<R> {
        self.invoke_async_with_cancellation(callback, priority, &CancellationToken::none())
    }

    /// Executes the specified callback asynchronously with the specified
    /// priority on the thread that the dispatcher was created on.
    ///
    /// `cancellation_token` aborts the operation as long as it has not
    /// started.
    pub fn invoke_async_with_cancellation<R: Send + 'static>(
        &self,
        callback: impl FnOnce() -> R + Send + 'static,
        priority: DispatcherPriority,
        cancellation_token: &CancellationToken,
    ) -> DispatcherOperation<R> {
        DispatcherPriority::validate(priority, "priority");

        let operation = DispatcherOperation::new_send(&self.to_arc(), priority, callback);
        self.invoke_async_impl(operation.core(), cancellation_token);

        operation
    }

    /// Like [`invoke_async`](Self::invoke_async) for callbacks that are not
    /// `Send`.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn invoke_async_local<R: 'static>(&self, callback: impl FnOnce() -> R + 'static) -> DispatcherOperation<R> {
        self.invoke_async_local_with_cancellation(callback, DispatcherPriority::DEFAULT, &CancellationToken::none())
    }

    /// Like [`invoke_async_with_priority`](Self::invoke_async_with_priority)
    /// for callbacks that are not `Send`.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn invoke_async_local_with_priority<R: 'static>(
        &self,
        callback: impl FnOnce() -> R + 'static,
        priority: DispatcherPriority,
    ) -> DispatcherOperation<R> {
        self.invoke_async_local_with_cancellation(callback, priority, &CancellationToken::none())
    }

    /// Like
    /// [`invoke_async_with_cancellation`](Self::invoke_async_with_cancellation)
    /// for callbacks that are not `Send`.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn invoke_async_local_with_cancellation<R: 'static>(
        &self,
        callback: impl FnOnce() -> R + 'static,
        priority: DispatcherPriority,
        cancellation_token: &CancellationToken,
    ) -> DispatcherOperation<R> {
        self.verify_access();
        DispatcherPriority::validate(priority, "priority");

        let operation = DispatcherOperation::new_local(&self.to_arc(), priority, callback);
        self.invoke_async_impl(operation.core(), cancellation_token);

        operation
    }

    pub(crate) fn invoke_async_impl(&self, operation: &Arc<OperationCore>, cancellation_token: &CancellationToken) {
        let mut succeeded = false;

        // Could be a non-dispatcher thread, lock to read
        {
            let mut st = self.lock();
            if !cancellation_token.is_cancellation_requested() && !st.has_shutdown_finished {
                // Add the operation to the work queue
                st.queue.enqueue(operation.priority(), operation.clone());

                // Make sure we will wake up to process this operation.
                succeeded = self.request_processing_locked(&mut st);

                if !succeeded {
                    // Dequeue the item since we failed to request
                    // processing for it.  Note we will mark it aborted
                    // below.
                    st.queue.remove_item(operation);
                }
            }
        }

        if succeeded {
            // We have enqueued the operation.  Register a callback
            // with the cancellation token to abort the operation
            // when cancellation is requested.
            if cancellation_token.can_be_canceled() {
                let to_abort = operation.clone();
                let registration = Arc::new(cancellation_token.register(move || {
                    to_abort.abort();
                }));

                // Revoke the cancellation when the operation is done.
                let revoke = registration.clone();
                operation.add_aborted(Arc::new(move || revoke.dispose()));
                operation.add_completed(Arc::new(move || registration.dispose()));
            }
        } else {
            // We failed to enqueue the operation, and the caller that
            // created the operation does not expose it before we return,
            // so it is safe to modify the operation outside of the lock.
            // Just mark the operation as aborted, which we can safely
            // return to the user.
            operation.set_status(DispatcherOperationStatus::Aborted);
            operation.call_abort_callbacks();
        }
    }

    /// Executes the specified callback on the dispatcher thread and then
    /// runs the future it returns there, at
    /// [`DispatcherPriority::DEFAULT`].
    ///
    /// The returned task completes when the future has completed; see
    /// [`invoke_async_task_with_priority`](Self::invoke_async_task_with_priority).
    pub fn invoke_async_task<F, Fut>(&self, callback: F) -> DispatcherTask<Fut::Output>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send + 'static,
    {
        self.invoke_async_task_with_priority(callback, DispatcherPriority::DEFAULT)
    }

    /// Executes the specified callback with the specified priority on the
    /// dispatcher thread and then runs the future it returns there.
    ///
    /// The callback can come from any thread; the future it creates never
    /// leaves the dispatcher thread and does not have to be `Send`. The
    /// future is polled for the first time right after the callback, inside
    /// the same job, and afterwards from jobs of the same priority.
    ///
    /// The returned task completes when the future has completed. It is
    /// canceled when the operation is aborted before the callback ran or the
    /// dispatcher shuts down before the future is done, and it fails with
    /// the panic of the callback or the future.
    pub fn invoke_async_task_with_priority<F, Fut>(
        &self,
        callback: F,
        priority: DispatcherPriority,
    ) -> DispatcherTask<Fut::Output>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send + 'static,
    {
        DispatcherPriority::validate(priority, "priority");
        let dispatcher = self.to_arc();
        let (task, completion, wrap) = wrap_send_future::<Fut>(&dispatcher);
        let target = dispatcher.clone();
        // When the operation is aborted its callback is dropped, which
        // cancels the task.
        drop(self.invoke_async_with_priority(
            move || match catch_unwind(AssertUnwindSafe(callback)) {
                Ok(future) => target.start_task(priority, wrap(future), completion, true),
                Err(payload) => completion.fail(payload),
            },
            priority,
        ));
        task
    }

    /// Like [`invoke_async_task`](Self::invoke_async_task) for callbacks and
    /// results that are not `Send`.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn invoke_async_task_local<F, Fut>(&self, callback: F) -> DispatcherTask<Fut::Output>
    where
        F: FnOnce() -> Fut + 'static,
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        self.invoke_async_task_local_with_priority(callback, DispatcherPriority::DEFAULT)
    }

    /// Like
    /// [`invoke_async_task_with_priority`](Self::invoke_async_task_with_priority)
    /// for callbacks and results that are not `Send`.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn invoke_async_task_local_with_priority<F, Fut>(
        &self,
        callback: F,
        priority: DispatcherPriority,
    ) -> DispatcherTask<Fut::Output>
    where
        F: FnOnce() -> Fut + 'static,
        Fut: Future + 'static,
        Fut::Output: 'static,
    {
        self.verify_access();
        DispatcherPriority::validate(priority, "priority");
        let dispatcher = self.to_arc();
        let (task, completion, wrap) = wrap_local_future::<Fut>(&dispatcher);
        let target = dispatcher.clone();
        drop(self.invoke_async_local_with_priority(
            move || match catch_unwind(AssertUnwindSafe(callback)) {
                Ok(future) => target.start_task(priority, wrap(future), completion, true),
                Err(payload) => completion.fail(payload),
            },
            priority,
        ));
        task
    }

    /// Posts an action that will be invoked on the dispatcher thread.
    ///
    /// Can be called from any thread. A panic raised by the action goes
    /// through the unhandled-exception events of the dispatcher.
    pub fn post(&self, action: impl FnOnce() + Send + 'static, priority: DispatcherPriority) {
        let callback: SendCallback = Box::new(move || {
            action();
            Box::new(()) as Box<dyn Any + Send>
        });
        let operation = OperationCore::new_send(&self.to_arc(), priority, true, callback);
        self.invoke_async_impl(&operation, &CancellationToken::none());
    }

    /// Posts an action that is not `Send` to be invoked on the dispatcher
    /// thread.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn post_local(&self, action: impl FnOnce() + 'static, priority: DispatcherPriority) {
        let operation = OperationCore::new_local(&self.to_arc(), priority, true, box_local_callback(action));
        self.invoke_async_impl(&operation, &CancellationToken::none());
    }

    /// Returns a future that awaits `task` and then resumes the awaiting
    /// code from a job of this dispatcher with the given priority, yielding
    /// the output of `task`.
    pub fn await_with_priority<F: Future>(
        &self,
        task: F,
        priority: DispatcherPriority,
    ) -> DispatcherPriorityTaskAwaitable<F> {
        DispatcherPriority::validate(priority, "priority");
        DispatcherPriorityTaskAwaitable::new(self.to_arc(), task, priority)
    }

    /// Returns a future that resumes on this dispatcher's thread at
    /// [`DispatcherPriority::BACKGROUND`] when awaited.
    pub fn resume(&self) -> DispatcherPriorityAwaitable {
        self.resume_with_priority(DispatcherPriority::BACKGROUND)
    }

    /// Returns a future that resumes on this dispatcher's thread at the
    /// given priority when awaited.
    pub fn resume_with_priority(&self, priority: DispatcherPriority) -> DispatcherPriorityAwaitable {
        DispatcherPriority::validate(priority, "priority");
        DispatcherPriorityAwaitable::new(self.to_arc(), priority)
    }

    /// Returns a future that yields to the current dispatcher, letting it
    /// process other work, and resumes at [`DispatcherPriority::BACKGROUND`].
    pub fn yield_now() -> DispatcherPriorityAwaitable {
        Self::yield_with_priority(DispatcherPriority::BACKGROUND)
    }

    /// Returns a future that yields to the current dispatcher and resumes at
    /// the given priority.
    pub fn yield_with_priority(priority: DispatcherPriority) -> DispatcherPriorityAwaitable {
        Self::current_dispatcher().resume_with_priority(priority)
    }
}
