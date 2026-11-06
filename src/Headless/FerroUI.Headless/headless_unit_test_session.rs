//! Port of `HeadlessUnitTestSession.cs`: the session that runs the tests of
//! a test assembly on a dispatcher thread of its own.
//!
//! What differs from the original, and why:
//!
//! - An entry point is a function that builds the application
//!   (`fn() -> AppBuilder`), where the original takes a type and finds its
//!   builder method through reflection. An assembly with its two attributes
//!   is a [`FerroTestAssembly`] value: there are no assembly attributes.
//! - An exception is a panic. The outcome of a dispatch is a
//!   [`HeadlessUnitTestTask`], which a thread other than the session thread
//!   waits for; a panic of the dispatched action, or of the cleanup after
//!   it, is raised again in the thread that observes the task.
//! - The application builder is not `Send` (it holds `Rc`s): it is created
//!   on the session thread, as in the original, and stays there, in the
//!   state of the consumer loop, where the original keeps it in a field of
//!   the session. The synchronization context of the shared application,
//!   its current priority (`Dispatcher::current_priority`), is kept in the
//!   same place.
//! - The session thread is isolated as a unit test thread
//!   ([`Dispatcher::unit_test_scope`]): the dispatcher of the session is the
//!   dispatcher of its thread, and resetting it does not touch the tests of
//!   the process that run on threads of their own. The original resets the
//!   one UI thread dispatcher of the process.
//! - There is no execution context to capture and to run a work item in.
//! - A work item that is still queued when the session is disposed is
//!   dropped, and its task is cancelled; the original leaves such a task
//!   incomplete for ever.
//! - The cleanup after a test of a shared application also processes the
//!   composition batches the test left in flight
//!   ([`finish_composition_in_flight`]), so that they do not hold back the
//!   commits of the next test.

use crate::{
    FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestIsolationAttribute, FerroTestIsolationLevel,
};
use ferroui_base::media::{FontManager, MediaContext};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{CancellationToken, Canceled, Dispatcher, DispatcherFrame, DispatcherPriority};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::{AppBuilder, Application, Control, IToolTipService};
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_system::task::oneshot;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// A panic payload: what the original carries as an exception.
type Exception = Box<dyn Any + Send + 'static>;

/// A test assembly as the session sees it: its name, by which
/// [`HeadlessUnitTestSession::get_or_start_for_assembly`] keeps one session
/// per assembly, and the two assembly attributes of the original. A test
/// crate declares one as a `static`.
#[derive(Debug)]
pub struct FerroTestAssembly {
    /// The name of the assembly: the name of the test crate, or of a part of
    /// it that has an application and an isolation level of its own.
    pub name: &'static str,
    /// The application of the tests; an empty application when `None`.
    pub test_application: Option<FerroTestApplicationAttribute>,
    /// The isolation of the tests; [`FerroTestIsolationLevel::PerTest`] when
    /// `None`.
    pub test_isolation: Option<FerroTestIsolationAttribute>,
}

// --- the task of a dispatch ----------------------------------------------------------------------

/// The completion source of a [`HeadlessUnitTestTask`]: the sender of a one-shot channel
/// ([`oneshot`]) of the outcome, a result or the panic that stands for an exception. A source
/// that is dropped without having completed its task cancels it.
struct TaskCompletionSource<T>(oneshot::Sender<Result<T, Exception>>);

impl<T> TaskCompletionSource<T> {
    fn new() -> (Self, HeadlessUnitTestTask<T>) {
        let (sender, receiver) = oneshot::channel();
        (Self(sender), HeadlessUnitTestTask { receiver })
    }

    // A task that was dropped does not want its outcome: what a send gives back is dropped.

    fn set_result(self, result: T) {
        let _ = self.0.send(Ok(result));
    }

    fn set_exception(self, exception: Exception) {
        let _ = self.0.send(Err(exception));
    }

    fn set_canceled(self) {
        drop(self);
    }
}

/// The outcome of an action dispatched to a [`HeadlessUnitTestSession`];
/// what the original returns as a task object.
pub struct HeadlessUnitTestTask<T> {
    receiver: oneshot::Receiver<Result<T, Exception>>,
}

impl<T> HeadlessUnitTestTask<T> {
    /// Whether the task has finished: completed, failed or cancelled.
    pub fn is_completed(&self) -> bool {
        self.receiver.is_finished()
    }

    /// Blocks until the task has finished and takes its result
    /// (`GetAwaiter().GetResult()`). Not to be called on the session thread,
    /// which is the thread that has to make progress.
    ///
    /// # Panics
    /// Raises the panic of the dispatched action, or of the cleanup after
    /// it, again.
    pub fn wait(self) -> Result<T, Canceled> {
        Self::take(self.receiver.blocking_recv())
    }

    /// Blocks until the task has finished or `timeout` has passed
    /// (`WaitAsync(TimeSpan)`).
    ///
    /// # Panics
    /// Panics when the task has not finished in time (`TimeoutException`);
    /// otherwise as [`wait`](Self::wait).
    pub fn wait_timeout(self, timeout: Duration) -> Result<T, Canceled> {
        let mut receiver = self.receiver;
        match receiver.recv_timeout(timeout) {
            Ok(None) => panic!("The operation has timed out."),
            Ok(Some(outcome)) => Self::take(Ok(outcome)),
            Err(canceled) => Self::take(Err(canceled)),
        }
    }

    fn take(outcome: Result<Result<T, Exception>, Canceled>) -> Result<T, Canceled> {
        match outcome {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(exception)) => resume_unwind(exception),
            Err(canceled) => Err(canceled),
        }
    }
}

// --- the queue of the session --------------------------------------------------------------------

/// What the consumer loop owns and a work item runs with: the fields of the
/// original session that cannot leave the session thread.
struct SessionThread {
    app_builder: AppBuilder,
    /// Only set and used with PerAssembly isolation
    shared_priority: Cell<Option<DispatcherPriority>>,
}

type WorkItem = Box<dyn FnOnce(&SessionThread) + Send + 'static>;

#[derive(Default)]
struct QueueState {
    items: VecDeque<WorkItem>,
    adding_completed: bool,
}

/// `BlockingCollection` of the work items.
#[derive(Default)]
struct BlockingQueue {
    state: Mutex<QueueState>,
    changed: Condvar,
}

impl BlockingQueue {
    fn lock(&self) -> MutexGuard<'_, QueueState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// # Panics
    /// Panics when the adding was completed (`InvalidOperationException`).
    fn add(&self, item: WorkItem) {
        let mut state = self.lock();
        if state.adding_completed {
            drop(state);
            panic!("The collection has been marked as complete with regards to additions.");
        }
        state.items.push_back(item);
        self.changed.notify_all();
    }

    /// `Take(CancellationToken)`: `None` is the cancellation of the token.
    fn take(&self, cancellation_token: &CancellationToken) -> Option<WorkItem> {
        let mut state = self.lock();
        loop {
            if cancellation_token.is_cancelled() {
                return None;
            }
            if let Some(item) = state.items.pop_front() {
                return Some(item);
            }
            state = self.changed.wait(state).unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Wakes the consumer so that it sees a cancellation.
    fn wake(&self) {
        let _state = self.lock();
        self.changed.notify_all();
    }

    fn complete_adding(&self) {
        let dropped = {
            let mut state = self.lock();
            state.adding_completed = true;
            std::mem::take(&mut state.items)
        };
        // Outside of the lock: dropping a work item cancels its task.
        drop(dropped);
    }
}

// --- the session ---------------------------------------------------------------------------------

/// Headless unit test session that needs to be used by the actual testing
/// framework. All UI tests are supposed to be executed from one of the
/// `dispatch` methods to keep execution flow on the UI thread. Disposing
/// unit test session stops internal dispatcher loop.
pub struct HeadlessUnitTestSession {
    cancellation_token: CancellationToken,
    queue: Arc<BlockingQueue>,
    dispatch_task: Mutex<Option<JoinHandle<()>>>,
    isolated: bool,
}

fn sessions() -> &'static Mutex<HashMap<&'static str, Arc<HeadlessUnitTestSession>>> {
    static SESSIONS: OnceLock<Mutex<HashMap<&'static str, Arc<HeadlessUnitTestSession>>>> = OnceLock::new();
    SESSIONS.get_or_init(Default::default)
}

/// The assembly of the session itself, which stands for an assembly that is
/// not given.
static HEADLESS_ASSEMBLY: FerroTestAssembly =
    FerroTestAssembly { name: env!("CARGO_PKG_NAME"), test_application: None, test_isolation: None };

/// Drops the compositor of the headless platform when the thread of a
/// session ends (see `FerroHeadlessPlatform::release_compositor`).
struct ReleaseCompositorOnDrop;

impl Drop for ReleaseCompositorOnDrop {
    fn drop(&mut self) {
        crate::ferro_headless_platform::FerroHeadlessPlatform::release_compositor();
    }
}

impl HeadlessUnitTestSession {
    /// Queues an action on the dispatcher thread; see
    /// [`dispatch_task`](Self::dispatch_task).
    pub fn dispatch(
        &self,
        action: impl FnOnce() + Send + 'static,
        cancellation_token: CancellationToken,
    ) -> HeadlessUnitTestTask<()> {
        self.dispatch_core(
            move || {
                action();
                std::future::ready(())
            },
            cancellation_token,
        )
    }

    /// Queues an action with a result on the dispatcher thread; see
    /// [`dispatch_task`](Self::dispatch_task).
    pub fn dispatch_result<TResult: Send + 'static>(
        &self,
        action: impl FnOnce() -> TResult + Send + 'static,
        cancellation_token: CancellationToken,
    ) -> HeadlessUnitTestTask<TResult> {
        self.dispatch_core(move || std::future::ready(action()), cancellation_token)
    }

    /// Dispatch method queues an async operation on the dispatcher thread,
    /// creates a new application instance, setting the application services,
    /// and runs `action` parameter.
    ///
    /// `action` is the action to execute on the dispatcher thread with the
    /// services; the future it returns never leaves the dispatcher thread.
    /// `cancellation_token` is the cancellation token to cancel execution.
    ///
    /// # Panics
    /// If global session was already cancelled and thread killed, it's not
    /// possible to dispatch any actions again (`ObjectDisposedException`).
    pub fn dispatch_task<F, Fut>(&self, action: F, cancellation_token: CancellationToken) -> HeadlessUnitTestTask<Fut::Output>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send + 'static,
    {
        self.dispatch_core(action, cancellation_token)
    }

    fn dispatch_core<F, Fut>(&self, action: F, cancellation_token: CancellationToken) -> HeadlessUnitTestTask<Fut::Output>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future + 'static,
        Fut::Output: Send + 'static,
    {
        if self.cancellation_token.is_cancelled() {
            panic!("Cannot access a disposed object.\nObject name: 'Session was already disposed.'.");
        }

        let token = self.cancellation_token.clone();
        let isolated = self.isolated;

        let (tcs, task) = TaskCompletionSource::<Fut::Output>::new();
        self.queue.add(Box::new(move |session: &SessionThread| {
            let cts = CancellationToken::new();
            let global_cts = {
                let cts = cts.clone();
                token.register(move || cts.cancel())
            };
            let local_cts = {
                let cts = cts.clone();
                cancellation_token.register(move || cts.cancel())
            };

            let application = match catch_unwind(AssertUnwindSafe(|| {
                if isolated {
                    session.ensure_isolated_application()
                } else {
                    session.ensure_shared_application()
                }
            })) {
                Ok(application) => application,
                Err(ex) => {
                    tcs.set_exception(ex);
                    local_cts.dispose();
                    global_cts.dispose();
                    return; // Exit the dispatcher action if application initialization fails
                }
            };

            let mut should_cancel = false;
            let mut caught: Option<Exception> = None;
            let mut result: Option<Fut::Output> = None;

            match catch_unwind(AssertUnwindSafe(|| {
                // The future is started as a call of an asynchronous method is: it runs to its
                // first pending await before `start_local` returns. The guard is the continuation
                // of the original, which cancels the token when the task has finished: when the
                // future has run to its end, has panicked or was dropped.
                let future = action();
                let guard = cts.clone().drop_guard();
                let mut task = Dispatcher::ui_thread().start_local(async move {
                    let _guard = guard;
                    future.await
                });
                if task.is_finished() {
                    return Some(result_of(&mut task));
                }

                if cts.is_cancelled() {
                    should_cancel = true;
                    None
                } else {
                    let frame = DispatcherFrame::new();
                    let inner_cts = {
                        let frame = frame.clone();
                        cts.register(move || frame.set_continue(false))
                    };
                    Dispatcher::ui_thread().push_frame(&frame);
                    inner_cts.dispose();
                    Some(result_of(&mut task))
                }
            })) {
                Ok(value) => result = value,
                Err(ex) => caught = Some(ex),
            }

            if let Err(ex) = catch_unwind(AssertUnwindSafe(|| application.dispose())) {
                // Cleanup runs before the TCS is completed, so its failure must be
                // reported by this work item instead of escaping the consumer loop.
                caught = Some(ex);
            }

            local_cts.dispose();
            global_cts.dispose();

            if let Some(caught) = caught {
                tcs.set_exception(caught);
            } else if should_cancel {
                tcs.set_canceled();
            } else if let Some(result) = result {
                tcs.set_result(result);
            }
        }));
        task
    }

    pub fn dispose(&self) {
        self.cancellation_token.cancel();
        self.queue.wake();
        let dispatch_task = self.dispatch_task.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(dispatch_task) = dispatch_task {
            // `_dispatchTask.Wait()`.
            if let Err(ex) = dispatch_task.join() {
                resume_unwind(ex);
            }
        }
        self.queue.complete_adding();
    }

    /// The asynchronous form of [`dispose`](Self::dispose). The dispatch
    /// thread is joined before the future is returned to its first poll:
    /// there is no task to await in its place.
    pub async fn dispose_async(&self) {
        self.dispose();
    }

    /// Creates instance of [`HeadlessUnitTestSession`] with the
    /// [`FerroTestIsolationLevel::PerTest`] isolation.
    ///
    /// `entry_point_type` is the parameter from which [`AppBuilder`] should
    /// be created: the function that builds the application
    /// (`AppBuilder::configure::<TApp>` for an application class).
    pub fn start_new(entry_point_type: fn() -> AppBuilder) -> Arc<HeadlessUnitTestSession> {
        Self::start_new_with_isolation(entry_point_type, FerroTestIsolationLevel::PerTest)
    }

    /// Creates instance of [`HeadlessUnitTestSession`].
    ///
    /// `entry_point_type` is the parameter from which [`AppBuilder`] should
    /// be created. `isolation_level` defines the isolation level for
    /// headless unit tests.
    ///
    /// # Panics
    /// Raises the panic of the entry point again.
    pub fn start_new_with_isolation(
        entry_point_type: fn() -> AppBuilder,
        isolation_level: FerroTestIsolationLevel,
    ) -> Arc<HeadlessUnitTestSession> {
        let (tcs, started) = mpsc::channel::<Result<(), Exception>>();
        let cancellation_token = CancellationToken::new();
        let queue = Arc::new(BlockingQueue::default());

        let task = {
            let cancellation_token = cancellation_token.clone();
            let queue = queue.clone();
            thread::Builder::new()
                .name("HeadlessUnitTestSession".to_owned())
                .spawn(move || {
                    // See the module documentation: the dispatcher of the session is the
                    // dispatcher of this thread.
                    let _dispatcher = Dispatcher::unit_test_scope();
                    // Dropped before the scope of the dispatcher, after the session.
                    let _compositor = ReleaseCompositorOnDrop;
                    let session = match catch_unwind(|| {
                        let mut app_builder = entry_point_type();

                        // If windowing subsystem wasn't initialized by user, force headless with default parameters.
                        if app_builder.windowing_subsystem_name().as_deref() != Some("Headless") {
                            app_builder = app_builder.use_headless(FerroHeadlessPlatformOptions::default());
                        }

                        if app_builder.text_shaping_subsystem_initializer().is_none() {
                            app_builder = app_builder.use_harfbuzz();
                        }

                        SessionThread { app_builder, shared_priority: Cell::new(None) }
                    }) {
                        Ok(session) => {
                            let _ = tcs.send(Ok(()));
                            session
                        }
                        Err(e) => {
                            let _ = tcs.send(Err(e));
                            return;
                        }
                    };

                    while !cancellation_token.is_cancelled() {
                        // `None` is the `OperationCanceledException` of the original.
                        if let Some(action) = queue.take(&cancellation_token) {
                            action(&session);
                        }
                    }
                })
                .expect("the thread of the headless unit test session")
        };

        match started.recv() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => resume_unwind(e),
            Err(_) => panic!("The thread of the headless unit test session ended before the session was created."),
        }

        let run_isolated = isolation_level == FerroTestIsolationLevel::PerTest;
        Arc::new(HeadlessUnitTestSession {
            cancellation_token,
            queue,
            dispatch_task: Mutex::new(Some(task)),
            isolated: run_isolated,
        })
    }

    /// Creates a session from the test application attribute of the
    /// assembly or reuses any existing. If the attribute doesn't exist,
    /// empty application is used.
    ///
    /// `None` is the assembly of the session itself.
    pub fn get_or_start_for_assembly(assembly: Option<&FerroTestAssembly>) -> Arc<HeadlessUnitTestSession> {
        let assembly = assembly.unwrap_or(&HEADLESS_ASSEMBLY);

        let mut sessions = sessions().lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(session) = sessions.get(assembly.name) {
            return session.clone();
        }

        let app_builder_entry_point_type =
            assembly.test_application.map(|attribute| attribute.app_builder_entry_point_type());

        let isolation_level = assembly
            .test_isolation
            .map_or(FerroTestIsolationLevel::PerTest, |attribute| attribute.isolation_level());

        let session = match app_builder_entry_point_type {
            Some(app_builder_entry_point_type) => {
                Self::start_new_with_isolation(app_builder_entry_point_type, isolation_level)
            }
            None => Self::start_new_with_isolation(AppBuilder::configure::<Application>, isolation_level),
        };

        sessions.insert(assembly.name, session.clone());

        session
    }
}

impl IDisposable for HeadlessUnitTestSession {
    fn dispose(&self) {
        HeadlessUnitTestSession::dispose(self)
    }
}

/// Processes the composition batches the test has left in flight, so that
/// the next test of a shared application starts with none.
///
/// Not in the original (DEVIATIONS.md, Headless platform), whose cleanup of
/// a shared application only runs the jobs. A batch is processed by a tick
/// of the render timer, and the timer of the headless platform ticks on the
/// dispatcher of the session, which does not run between two dispatches: a
/// batch the jobs after a test commit (a test that closes a window ends so:
/// the window hides before its renderer is disposed, which asks for a
/// commit) stays in flight into the next test, whose commits the media
/// context then holds back until a tick. A window the next test shows is
/// not composed by its jobs, and its composition visual is not attached to
/// its target (the snapshot test of the rendering tests failed so, in the
/// order of tests in which the leak test ran before it). An isolated
/// application is set up afresh for every test and has nothing in flight.
///
/// A tick processes the batches and posts their completion; the jobs after
/// it run the completion and the render pass it schedules, which may commit
/// once more. As many rounds as the headless window extensions render at
/// most in one go.
fn finish_composition_in_flight() {
    let Some(media_context) = FerroLocator::current().get_service::<MediaContext>() else {
        return;
    };
    for _ in 0..10 {
        if !media_context.has_composition_batches_in_flight() {
            return;
        }
        FerroHeadlessPlatform::force_render_timer_tick(1);
        Dispatcher::ui_thread().run_jobs(None);
    }
}

impl SessionThread {
    fn ensure_shared_application(&self) -> Box<dyn IDisposable> {
        let old_priority = Dispatcher::current_priority();
        if Application::current().is_none() {
            self.app_builder.setup_unsafe();
            self.shared_priority.set(Dispatcher::current_priority());
        } else {
            Dispatcher::set_current_priority(self.shared_priority.get());
        }

        Box::new(Cleanup::new(move || {
            let jobs = catch_unwind(|| {
                Dispatcher::ui_thread().run_jobs(None);
                finish_composition_in_flight();
            });
            Dispatcher::set_current_priority(old_priority);
            if let Err(ex) = jobs {
                resume_unwind(ex);
            }
        }))
    }

    fn ensure_isolated_application(&self) -> Box<dyn IDisposable> {
        let scope = FerroLocator::enter_scope();
        let old_priority = Dispatcher::current_priority();
        if let Err(ex) = catch_unwind(AssertUnwindSafe(|| {
            Dispatcher::reset_before_unit_tests();
            self.app_builder.setup_unsafe();
        })) {
            scope.dispose();
            resume_unwind(ex);
        }

        Box::new(Cleanup::new(move || {
            let services = catch_unwind(|| {
                let tool_tip_service = FerroLocator::current().get_service::<dyn IToolTipService>();
                if let Some(tool_tip_service) =
                    tool_tip_service.as_ref().and_then(|service| service.as_tool_tip_service())
                {
                    tool_tip_service.dispose();
                }
                if let Some(font_manager) = FerroLocator::current().get_service::<FontManager>() {
                    font_manager.dispose();
                }
                Dispatcher::reset_for_unit_tests();
            });

            // Cleanup jobs can throw, but the ambient state still belongs to this dispatch.
            let scope = catch_unwind(AssertUnwindSafe(|| scope.dispose()));
            Dispatcher::reset_before_unit_tests();
            Dispatcher::set_current_priority(old_priority);
            // Not in the original: the queue of the controls that wait for their loaded event is
            // a static of the thread, which the reset dispatcher will not run (as the scope of
            // the unit test application of the controls does).
            Control::reset_loaded_queue_for_unit_tests();

            // An exception of the inner `finally` replaces the one that was in flight.
            if let Err(ex) = scope {
                resume_unwind(ex);
            }
            if let Err(ex) = services {
                resume_unwind(ex);
            }
        }))
    }
}

/// `Disposable.Create`.
struct Cleanup(RefCell<Option<Box<dyn FnOnce()>>>);

impl Cleanup {
    fn new(action: impl FnOnce() + 'static) -> Self {
        Self(RefCell::new(Some(Box::new(action))))
    }
}

impl IDisposable for Cleanup {
    fn dispose(&self) {
        let action = self.0.borrow_mut().take();
        if let Some(action) = action {
            action();
        }
    }
}

/// The output of the task of a test, which has ended: a panic of its future is raised again, and a
/// cancellation is raised as [`Canceled`], as the outcome of the task was before it became a
/// `JoinHandle`.
fn result_of<T>(task: &mut ferroui_system::task::JoinHandle<T>) -> T {
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    match std::pin::Pin::new(task).poll(&mut context) {
        std::task::Poll::Ready(Ok(result)) => result,
        std::task::Poll::Ready(Err(ferroui_system::task::JoinError::Panicked(payload))) => resume_unwind(payload),
        std::task::Poll::Ready(Err(ferroui_system::task::JoinError::Canceled)) => std::panic::panic_any(Canceled),
        std::task::Poll::Pending => panic!("Synchronous wait is only supported on non-UI threads"),
    }
}
