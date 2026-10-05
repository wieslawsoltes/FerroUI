use std::any::{Any, TypeId};
use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use super::dispatcher::DispatcherLocal;
use super::dispatcher_priority_queue::NOT_QUEUED;
use super::{
    CancellationTokenSource, Dispatcher, DispatcherFrame, DispatcherPriority, DispatcherTimer,
    FerroSynchronizationContext,
};
use crate::reactive::{Disposable, IDisposable};

/// The payload of a panic raised by a dispatcher callback. This is what the
/// reference implementation models as the exception object.
pub type DispatcherException = Box<dyn Any + Send + 'static>;

/// A callback that can be queued from any thread.
pub(crate) type SendCallback = Box<dyn FnOnce() -> Box<dyn Any + Send> + Send>;

/// A callback that is queued on, and never leaves, the dispatcher thread.
/// Returns `None` when the result is `()`.
pub(crate) type LocalCallback = Box<dyn FnOnce() -> Option<Box<dyn Any>>>;

type EventHandler = Arc<dyn Fn() + Send + Sync>;

/// The state of a [`DispatcherOperation`].
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DispatcherOperationStatus {
    Pending = 0,
    Aborted = 1,
    Completed = 2,
    Executing = 3,
}

impl DispatcherOperationStatus {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Pending,
            1 => Self::Aborted,
            2 => Self::Completed,
            _ => Self::Executing,
        }
    }
}

/// The error produced when the result of an aborted operation is requested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationCanceledError;

impl fmt::Display for OperationCanceledError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("The dispatcher operation was canceled")
    }
}

impl std::error::Error for OperationCanceledError {}

#[derive(Default)]
struct OperationInner {
    callback: Option<SendCallback>,
    result: Option<Box<dyn Any + Send>>,
    panic: Option<DispatcherException>,
    wakers: Vec<Waker>,
    aborted: Vec<(u64, EventHandler)>,
    completed: Vec<(u64, EventHandler)>,
    next_handler_id: u64,
}

static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);

/// Ids are shared by operations and tasks: both key the same thread-local
/// storage.
pub(crate) fn next_operation_id() -> u64 {
    NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed)
}

/// How a wait with a limit ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaitOutcome {
    /// The operation is completed or aborted.
    Finished,
    /// The time ran out first.
    TimedOut,
}

/// The untyped, shareable part of a dispatcher operation. This is what the
/// dispatcher queue stores.
pub(crate) struct OperationCore {
    id: u64,
    dispatcher: Arc<Dispatcher>,
    throw_on_ui_thread: bool,
    /// The callback (and the result, unless it is `()`) is kept in the
    /// dispatcher thread's local storage instead of in `inner`.
    is_local: bool,
    /// Whether local storage may still hold something for this operation.
    local_storage_used: AtomicBool,
    status: AtomicU8,
    priority: AtomicI32,
    queue_slot: AtomicUsize,
    inner: Mutex<OperationInner>,
    finished: Condvar,
}

impl OperationCore {
    fn new(
        dispatcher: &Arc<Dispatcher>,
        priority: DispatcherPriority,
        throw_on_ui_thread: bool,
        callback: Option<SendCallback>,
    ) -> Arc<Self> {
        let is_local = callback.is_none();
        Arc::new(Self {
            id: next_operation_id(),
            dispatcher: dispatcher.clone(),
            throw_on_ui_thread,
            is_local,
            local_storage_used: AtomicBool::new(is_local),
            status: AtomicU8::new(DispatcherOperationStatus::Pending as u8),
            priority: AtomicI32::new(priority.value()),
            queue_slot: AtomicUsize::new(NOT_QUEUED),
            inner: Mutex::new(OperationInner { callback, ..OperationInner::default() }),
            finished: Condvar::new(),
        })
    }

    /// Creates an operation whose callback can be queued from any thread.
    pub(crate) fn new_send(
        dispatcher: &Arc<Dispatcher>,
        priority: DispatcherPriority,
        throw_on_ui_thread: bool,
        callback: SendCallback,
    ) -> Arc<Self> {
        Self::new(dispatcher, priority, throw_on_ui_thread, Some(callback))
    }

    /// Creates an operation whose callback stays on the dispatcher thread.
    ///
    /// # Panics
    /// Panics when called from another thread.
    pub(crate) fn new_local(
        dispatcher: &Arc<Dispatcher>,
        priority: DispatcherPriority,
        throw_on_ui_thread: bool,
        callback: LocalCallback,
    ) -> Arc<Self> {
        dispatcher.verify_access();
        let core = Self::new(dispatcher, priority, throw_on_ui_thread, None);
        match dispatcher.try_local() {
            Some(local) => {
                local.jobs.borrow_mut().insert(core.id, callback);
            }
            None => core.local_storage_used.store(false, Ordering::Relaxed),
        }
        core
    }

    fn lock(&self) -> MutexGuard<'_, OperationInner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[inline]
    pub(crate) fn status(&self) -> DispatcherOperationStatus {
        DispatcherOperationStatus::from_u8(self.status.load(Ordering::SeqCst))
    }

    #[inline]
    pub(crate) fn set_status(&self, status: DispatcherOperationStatus) {
        self.status.store(status as u8, Ordering::SeqCst);
    }

    #[inline]
    pub(crate) fn dispatcher(&self) -> &Arc<Dispatcher> {
        &self.dispatcher
    }

    #[inline]
    pub(crate) fn priority(&self) -> DispatcherPriority {
        let value = self.priority.load(Ordering::SeqCst);
        if value == DispatcherPriority::INVALID.value() {
            DispatcherPriority::INVALID
        } else {
            DispatcherPriority::from_value(value)
        }
    }

    pub(crate) fn set_priority(self: &Arc<Self>, value: DispatcherPriority) {
        self.priority.store(value.value(), Ordering::SeqCst);
        self.dispatcher.set_priority(self, value);
    }

    #[inline]
    pub(crate) fn queue_slot(&self) -> usize {
        self.queue_slot.load(Ordering::Relaxed)
    }

    #[inline]
    pub(crate) fn set_queue_slot(&self, slot: usize) {
        self.queue_slot.store(slot, Ordering::Relaxed);
    }

    #[inline]
    pub(crate) fn is_queued(&self) -> bool {
        self.queue_slot() != NOT_QUEUED
    }

    fn add_handler(self: &Arc<Self>, aborted: bool, handler: EventHandler) -> u64 {
        let mut inner = self.lock();
        inner.next_handler_id += 1;
        let id = inner.next_handler_id;
        if aborted {
            inner.aborted.push((id, handler));
        } else {
            inner.completed.push((id, handler));
        }
        id
    }

    fn remove_handler(&self, aborted: bool, id: u64) {
        let removed = {
            let mut inner = self.lock();
            let list = if aborted { &mut inner.aborted } else { &mut inner.completed };
            list.iter().position(|(handler_id, _)| *handler_id == id).map(|index| list.remove(index))
        };
        drop(removed);
    }

    pub(crate) fn add_aborted(self: &Arc<Self>, handler: EventHandler) -> u64 {
        self.add_handler(true, handler)
    }

    pub(crate) fn add_completed(self: &Arc<Self>, handler: EventHandler) -> u64 {
        self.add_handler(false, handler)
    }

    pub(crate) fn remove_aborted(&self, id: u64) {
        self.remove_handler(true, id);
    }

    pub(crate) fn remove_completed(&self, id: u64) {
        self.remove_handler(false, id);
    }

    pub(crate) fn abort(self: &Arc<Self>) -> bool {
        if self.dispatcher.abort(self) {
            self.call_abort_callbacks();
            return true;
        }

        false
    }

    /// Wakes everything waiting for the operation and raises `Aborted`.
    pub(crate) fn call_abort_callbacks(&self) {
        debug_assert_eq!(self.status(), DispatcherOperationStatus::Aborted);
        let (callback, wakers, handlers) = {
            let mut inner = self.lock();
            let result = (inner.callback.take(), std::mem::take(&mut inner.wakers), inner.aborted.clone());
            self.finished.notify_all();
            result
        };
        // The callback will never run; release whatever it captured.
        drop(callback);
        self.discard_local_storage();
        for waker in wakers {
            waker.wake();
        }
        for (_, handler) in handlers {
            handler();
        }
    }

    fn call_completed_callbacks(&self) {
        let (wakers, handlers) = {
            let mut inner = self.lock();
            let result = (std::mem::take(&mut inner.wakers), inner.completed.clone());
            self.finished.notify_all();
            result
        };
        for waker in wakers {
            waker.wake();
        }
        for (_, handler) in handlers {
            handler();
        }
    }

    fn discard_local_storage(&self) {
        if self.is_local && self.local_storage_used.swap(false, Ordering::SeqCst) {
            self.dispatcher.discard_local_operation(self.id);
        }
    }

    /// Runs the callback. Called by the dispatcher, on its thread, after the
    /// status has been changed to `Executing`.
    pub(crate) fn execute(&self) {
        debug_assert_eq!(self.status(), DispatcherOperationStatus::Executing);
        let local = self.dispatcher.try_local();
        if let Some(local) = &local {
            local.executing_depth.set(local.executing_depth.get() + 1);
        }

        let outcome = catch_unwind(AssertUnwindSafe(|| {
            // The current context tells asynchronous code started by the
            // callback to continue at the priority of this operation.
            let _restore = FerroSynchronizationContext::ensure_with_dispatcher(&self.dispatcher, self.priority());
            self.invoke_core(local.as_deref())
        }));

        // finally
        let completed = catch_unwind(AssertUnwindSafe(|| self.call_completed_callbacks()));

        if let Some(local) = &local {
            let depth = local.executing_depth.get() - 1;
            local.executing_depth.set(depth);
            if depth == 0 {
                // No dispatcher code is left on the stack that could see one
                // of the already-filtered panics again.
                local.seen_exceptions.borrow_mut().clear();
            }
        }

        match outcome {
            Ok(None) => {}
            Ok(Some(payload)) | Err(payload) => resume_unwind(payload),
        }
        if let Err(payload) = completed {
            resume_unwind(payload);
        }
    }

    /// Returns the panic payload that has to be re-raised on the dispatcher
    /// thread, if any.
    fn invoke_core(&self, local: Option<&DispatcherLocal>) -> Option<DispatcherException> {
        enum Value {
            Send(Box<dyn Any + Send>),
            Local(Option<Box<dyn Any>>),
            Missing,
        }

        let result = if self.is_local {
            let job = local.and_then(|local| local.jobs.borrow_mut().remove(&self.id));
            match job {
                Some(job) => catch_unwind(AssertUnwindSafe(|| Value::Local(job()))),
                None => Ok(Value::Missing),
            }
        } else {
            let job = self.lock().callback.take();
            match job {
                Some(job) => catch_unwind(AssertUnwindSafe(|| Value::Send(job()))),
                None => Ok(Value::Missing),
            }
        };

        match result {
            Ok(value) => {
                let mut local_result = None;
                {
                    let mut inner = self.lock();
                    self.set_status(DispatcherOperationStatus::Completed);
                    match value {
                        Value::Send(value) => inner.result = Some(value),
                        Value::Local(value) => local_result = value,
                        Value::Missing => {}
                    }
                }
                match (local_result, local) {
                    (Some(value), Some(local)) => {
                        local.results.borrow_mut().insert(self.id, value);
                    }
                    _ => self.local_storage_used.store(false, Ordering::SeqCst),
                }
                None
            }
            Err(payload) => {
                self.local_storage_used.store(false, Ordering::SeqCst);
                if self.throw_on_ui_thread {
                    self.set_status(DispatcherOperationStatus::Completed);
                    if self.dispatcher.try_catch_when(&payload) {
                        None
                    } else {
                        Some(payload)
                    }
                } else {
                    let mut inner = self.lock();
                    self.set_status(DispatcherOperationStatus::Completed);
                    inner.panic = Some(payload);
                    None
                }
            }
        }
    }

    /// Blocks the calling (non-dispatcher) thread until the operation is
    /// completed or aborted.
    fn block_until_finished(&self) {
        let mut inner = self.lock();
        while matches!(self.status(), DispatcherOperationStatus::Pending | DispatcherOperationStatus::Executing) {
            inner = self.finished.wait(inner).unwrap_or_else(PoisonError::into_inner);
        }
    }

    fn is_finished(&self) -> bool {
        matches!(self.status(), DispatcherOperationStatus::Completed | DispatcherOperationStatus::Aborted)
    }

    pub(crate) fn wait(self: &Arc<Self>) {
        self.wait_with_limit(None, false);
    }

    /// Waits for the operation to finish, for at most `timeout`.
    ///
    /// On the dispatcher thread this pumps the queue, measuring time with
    /// the dispatcher clock; on any other thread it blocks. With
    /// `abort_on_timeout` the operation is aborted when the time runs out
    /// while it is still pending; when it has already started by then, the
    /// wait continues until it is done.
    pub(crate) fn wait_with_limit(self: &Arc<Self>, timeout: Option<Duration>, abort_on_timeout: bool) -> WaitOutcome {
        let dispatcher = &self.dispatcher;
        let on_dispatcher_thread = dispatcher.check_access();

        if matches!(self.status(), DispatcherOperationStatus::Pending | DispatcherOperationStatus::Executing)
            && timeout != Some(Duration::ZERO)
            && on_dispatcher_thread
        {
            if self.status() == DispatcherOperationStatus::Executing {
                // We are the dispatching thread, and the current operation state is
                // executing, which means that the operation is in the middle of
                // executing (on this thread) and is trying to wait for the execution
                // to complete.  Unfortunately, the thread will now deadlock, so
                // we panic instead.
                panic!("A thread cannot wait on operations already running on the same thread.");
            }

            let deadline = timeout.map(|timeout| dispatcher.now().saturating_add(duration_to_ms(timeout)));
            let cts = CancellationTokenSource::new();
            let cancel = cts.clone();
            let finished_handler: EventHandler = Arc::new(move || cancel.cancel());
            let completed = self.add_completed(finished_handler.clone());
            let aborted = self.add_aborted(finished_handler);

            let result = catch_unwind(AssertUnwindSafe(|| {
                while self.status() == DispatcherOperationStatus::Pending {
                    let remaining = match deadline {
                        Some(deadline) => {
                            let remaining = deadline - dispatcher.now();
                            if remaining <= 0 {
                                break;
                            }
                            Some(remaining)
                        }
                        None => None,
                    };
                    if dispatcher.supports_run_loops() {
                        if self.priority() >= DispatcherPriority::MINIMUM_FOREGROUND_PRIORITY {
                            dispatcher.run_jobs_with_cancellation(Some(self.priority()), &cts.token());
                        } else {
                            self.push_operation_frame(remaining);
                        }
                    } else {
                        dispatcher.run_jobs_with_cancellation(
                            Some(DispatcherPriority::MINIMUM_ACTIVE_VALUE),
                            &cts.token(),
                        );
                    }
                }
            }));

            // finally
            self.remove_completed(completed);
            self.remove_aborted(aborted);
            if let Err(payload) = result {
                resume_unwind(payload);
            }
        }

        if self.is_finished() {
            return WaitOutcome::Finished;
        }

        if on_dispatcher_thread {
            // Only reachable with a timeout: blocking here could never end.
            if abort_on_timeout {
                self.abort();
            }
            return WaitOutcome::TimedOut;
        }

        match timeout {
            None => {
                self.block_until_finished();
                WaitOutcome::Finished
            }
            Some(timeout) => {
                if self.block_until_finished_or_timeout(timeout) {
                    return WaitOutcome::Finished;
                }
                if !abort_on_timeout {
                    return WaitOutcome::TimedOut;
                }
                if self.abort() {
                    return WaitOutcome::TimedOut;
                }
                // The operation has already started: we can't leave it in a
                // state that it might execute if we report that it did not.
                self.block_until_finished();
                WaitOutcome::Finished
            }
        }
    }

    /// Blocks the calling (non-dispatcher) thread; returns whether the
    /// operation finished in time.
    fn block_until_finished_or_timeout(&self, timeout: Duration) -> bool {
        let inner = self.lock();
        let (_inner, _result) = self
            .finished
            .wait_timeout_while(inner, timeout, |_| {
                matches!(self.status(), DispatcherOperationStatus::Pending | DispatcherOperationStatus::Executing)
            })
            .unwrap_or_else(PoisonError::into_inner);
        self.is_finished()
    }

    /// Pumps a nested frame until the operation is completed or aborted, or
    /// until `timeout_ms` of dispatcher time have passed.
    ///
    /// The frame does not exit when the dispatcher asks its frames to exit,
    /// because operations may have to be invoked during the shutdown process.
    fn push_operation_frame(self: &Arc<Self>, timeout_ms: Option<i64>) {
        let frame = DispatcherFrame::with_dispatcher(&self.dispatcher, false);

        // We will exit this frame once the operation is completed or aborted.
        let exit_frame = frame.clone();
        let handler: EventHandler = Arc::new(move || exit_frame.set_continue(false));
        let aborted = self.add_aborted(handler.clone());
        let completed = self.add_completed(handler);

        // We will exit the frame if the operation is not completed within
        // the requested timeout.
        let wait_timer = timeout_ms.filter(|timeout| *timeout > 0).map(|timeout| {
            let exit_frame = frame.clone();
            DispatcherTimer::with_callback_and_dispatcher(
                Duration::from_millis(timeout.min(i32::MAX as i64) as u64),
                DispatcherPriority::SEND,
                &self.dispatcher,
                move |timer| {
                    timer.stop();
                    exit_frame.set_continue(false);
                },
            )
        });

        // Some other thread could have aborted the operation while we were
        // setting up the handlers.  We check the state again and mark the
        // frame as "should not continue" if this happened.
        if self.status() != DispatcherOperationStatus::Pending {
            frame.set_continue(false);
        }

        let result = catch_unwind(AssertUnwindSafe(|| self.dispatcher.push_frame(&frame)));
        if let Some(wait_timer) = wait_timer {
            wait_timer.stop();
        }
        self.remove_aborted(aborted);
        self.remove_completed(completed);
        if let Err(payload) = result {
            resume_unwind(payload);
        }
    }

    /// Takes the outcome of a finished operation: re-raises the callback's
    /// panic, or returns the boxed result.
    fn take_outcome<R: 'static>(&self) -> Result<R, OperationCanceledError> {
        debug_assert!(self.is_finished());
        if self.status() == DispatcherOperationStatus::Aborted {
            return Err(OperationCanceledError);
        }

        let (panic, result) = {
            let mut inner = self.lock();
            (inner.panic.take(), inner.result.take())
        };
        if let Some(payload) = panic {
            resume_unwind(payload);
        }

        let value: Option<Box<dyn Any>> = if !self.is_local {
            result.map(|value| value as Box<dyn Any>)
        } else if TypeId::of::<R>() == TypeId::of::<()>() {
            Some(Box::new(()))
        } else {
            if !self.dispatcher.check_access() {
                panic!("The result of a local dispatcher operation can only be read on the dispatcher thread.");
            }
            let value = self.dispatcher.try_local().and_then(|local| local.results.borrow_mut().remove(&self.id));
            self.local_storage_used.store(false, Ordering::SeqCst);
            value
        };

        match value.and_then(|value| value.downcast::<R>().ok()) {
            Some(value) => Ok(*value),
            None => panic!("The result of the dispatcher operation has already been taken."),
        }
    }
}

impl Drop for OperationCore {
    fn drop(&mut self) {
        self.discard_local_storage();
    }
}

/// Represents an object used to interact with a job that has been posted to
/// the [`Dispatcher`] queue.
///
/// The handle can be cloned and sent to other threads. It is also a
/// [`Future`] that resolves when the operation has completed
/// (`Ok(result)`) or has been aborted (`Err(OperationCanceledError)`).
///
/// A panic raised by the callback is the equivalent of an exception in the
/// reference implementation: it is captured when the callback runs and
/// re-raised by [`wait`](Self::wait), [`result`](Self::result) and when the
/// future is polled.
pub struct DispatcherOperation<R = ()> {
    core: Arc<OperationCore>,
    _result: PhantomData<fn() -> R>,
}

impl<R> Clone for DispatcherOperation<R> {
    fn clone(&self) -> Self {
        Self { core: self.core.clone(), _result: PhantomData }
    }
}

impl<R> fmt::Debug for DispatcherOperation<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DispatcherOperation #{} [{}] {:?}", self.core.id, self.core.priority(), self.core.status())
    }
}

impl<R: 'static> DispatcherOperation<R> {
    pub(crate) fn from_core(core: Arc<OperationCore>) -> Self {
        Self { core, _result: PhantomData }
    }

    pub(crate) fn core(&self) -> &Arc<OperationCore> {
        &self.core
    }

    /// Creates an operation that can be queued from any thread.
    pub(crate) fn new_send(
        dispatcher: &Arc<Dispatcher>,
        priority: DispatcherPriority,
        callback: impl FnOnce() -> R + Send + 'static,
    ) -> Self
    where
        R: Send,
    {
        let callback: SendCallback = Box::new(move || Box::new(callback()) as Box<dyn Any + Send>);
        Self::from_core(OperationCore::new_send(dispatcher, priority, false, callback))
    }

    /// Creates an operation whose callback and result stay on the dispatcher
    /// thread.
    pub(crate) fn new_local(
        dispatcher: &Arc<Dispatcher>,
        priority: DispatcherPriority,
        callback: impl FnOnce() -> R + 'static,
    ) -> Self {
        Self::from_core(OperationCore::new_local(dispatcher, priority, false, box_local_callback(callback)))
    }

    /// The current status of the operation.
    pub fn status(&self) -> DispatcherOperationStatus {
        self.core.status()
    }

    /// The dispatcher the operation was queued on.
    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        self.core.dispatcher()
    }

    pub fn priority(&self) -> DispatcherPriority {
        self.core.priority()
    }

    /// Changes the priority of the operation. A queued operation keeps its
    /// position relative to the other operations of the new priority.
    pub fn set_priority(&self, value: DispatcherPriority) {
        self.core.set_priority(value);
    }

    /// An event that is raised when the operation is aborted or canceled.
    ///
    /// The handler runs on the thread that aborts the operation.
    pub fn aborted(&self, handler: impl Fn() + Send + Sync + 'static) -> Rc<dyn IDisposable> {
        let id = self.core.add_aborted(Arc::new(handler));
        let core = self.core.clone();
        Disposable::create(move || core.remove_aborted(id))
    }

    /// An event that is raised when the operation completes.
    ///
    /// The handler runs on the dispatcher thread.
    pub fn completed(&self, handler: impl Fn() + Send + Sync + 'static) -> Rc<dyn IDisposable> {
        let id = self.core.add_completed(Arc::new(handler));
        let core = self.core.clone();
        Disposable::create(move || core.remove_completed(id))
    }

    /// Removes a pending operation from the queue. Returns `false` when the
    /// operation is no longer pending.
    pub fn abort(&self) -> bool {
        self.core.abort()
    }

    /// Waits for this operation to complete.
    ///
    /// On the dispatcher thread this pumps the queue (nesting a frame when
    /// the operation has a background priority and the platform supports run
    /// loops). On any other thread it blocks.
    ///
    /// Returns `Err` when the operation was aborted.
    ///
    /// # Panics
    /// Re-raises the panic of the callback. Panics when called on the
    /// dispatcher thread for the operation that is currently executing.
    pub fn wait(&self) -> Result<(), OperationCanceledError> {
        self.core.wait();
        if self.core.status() == DispatcherOperationStatus::Aborted {
            return Err(OperationCanceledError);
        }
        let panic = self.core.lock().panic.take();
        if let Some(payload) = panic {
            resume_unwind(payload);
        }
        Ok(())
    }

    /// Waits for this operation to complete, for at most `timeout`.
    ///
    /// Returns `Ok(true)` when the operation completed in time, `Ok(false)`
    /// when it is still pending or executing after `timeout`, and `Err` when
    /// it was aborted.
    ///
    /// On the dispatcher thread this pumps the queue like
    /// [`wait`](Self::wait) and measures the timeout with the dispatcher
    /// clock; a nested frame (background priorities, where the platform
    /// supports run loops) is left by a dispatcher timer. On any other
    /// thread it blocks. A zero timeout only reports the current state.
    ///
    /// # Panics
    /// Re-raises the panic of the callback. Panics when called on the
    /// dispatcher thread for the operation that is currently executing.
    pub fn wait_with_timeout(&self, timeout: Duration) -> Result<bool, OperationCanceledError> {
        if self.core.wait_with_limit(Some(timeout), false) == WaitOutcome::TimedOut {
            return Ok(false);
        }
        self.wait().map(|()| true)
    }

    /// Takes the result of the operation.
    ///
    /// When the operation has not finished yet this blocks, which is only
    /// supported on non-dispatcher threads.
    ///
    /// # Panics
    /// Re-raises the panic of the callback. Panics when the operation is not
    /// finished and the calling thread is the dispatcher thread, when the
    /// result has already been taken, and when the result of a local
    /// operation is requested from another thread.
    pub fn result(&self) -> Result<R, OperationCanceledError> {
        if !self.core.is_finished() {
            if self.core.dispatcher.check_access() {
                panic!("Synchronous wait is only supported on non-UI threads");
            }
            self.core.block_until_finished();
        }
        self.core.take_outcome()
    }

    /// Takes the result of a finished operation.
    pub(crate) fn take_result(&self) -> Result<R, OperationCanceledError> {
        self.core.take_outcome()
    }
}

impl<R: 'static> Future for DispatcherOperation<R> {
    type Output = Result<R, OperationCanceledError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        {
            let mut inner = self.core.lock();
            // Checked under the lock: the status is published before the
            // wakers are drained under the same lock.
            if !self.core.is_finished() {
                if !inner.wakers.iter().any(|waker| waker.will_wake(cx.waker())) {
                    inner.wakers.push(cx.waker().clone());
                }
                return Poll::Pending;
            }
        }
        Poll::Ready(self.core.take_outcome())
    }
}

pub(crate) fn box_local_callback<R: 'static>(callback: impl FnOnce() -> R + 'static) -> LocalCallback {
    Box::new(move || {
        let value = callback();
        if TypeId::of::<R>() == TypeId::of::<()>() {
            None
        } else {
            Some(Box::new(value) as Box<dyn Any>)
        }
    })
}

/// Milliseconds of a duration on the dispatcher clock.
pub(crate) fn duration_to_ms(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}
