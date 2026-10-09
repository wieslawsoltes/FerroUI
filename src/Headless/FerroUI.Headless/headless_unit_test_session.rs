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
//!   the session. The synchronization context of the shared application is
//!   kept in the same place.
//! - The session thread is isolated as a unit test thread
//!   ([`Dispatcher::unit_test_scope`]): the dispatcher of the session is the
//!   dispatcher of its thread, and resetting it does not touch the tests of
//!   the process that run on threads of their own. The original resets the
//!   one UI thread dispatcher of the process.
//! - There is no execution context to capture and to run a work item in.
//! - A work item that is still queued when the session is disposed is
//!   dropped, and its task is cancelled; the original leaves such a task
//!   incomplete for ever.

use crate::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestIsolationAttribute, FerroTestIsolationLevel,
};
use ferroui_base::media::FontManager;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{
    CancellationToken, CancellationTokenSource, Dispatcher, DispatcherFrame, FerroSynchronizationContext,
    OperationCanceledError,
};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::{AppBuilder, Application, Control, IToolTipService};
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use std::any::Any;
use std::cell::RefCell;
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

enum TaskState<T> {
    Running,
    RanToCompletion(Option<T>),
    Faulted(Option<Exception>),
    Canceled,
}

struct TaskShared<T> {
    state: Mutex<TaskState<T>>,
    finished: Condvar,
}

impl<T> TaskShared<T> {
    fn lock(&self) -> MutexGuard<'_, TaskState<T>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn try_set(&self, state: TaskState<T>) -> bool {
        let mut current = self.lock();
        if !matches!(*current, TaskState::Running) {
            return false;
        }
        *current = state;
        self.finished.notify_all();
        true
    }
}

/// The completion source of a [`HeadlessUnitTestTask`]. A source that is
/// dropped without having completed its task cancels it.
struct TaskCompletionSource<T>(Arc<TaskShared<T>>);

impl<T> TaskCompletionSource<T> {
    fn new() -> (Self, HeadlessUnitTestTask<T>) {
        let shared = Arc::new(TaskShared { state: Mutex::new(TaskState::Running), finished: Condvar::new() });
        (Self(shared.clone()), HeadlessUnitTestTask { shared })
    }

    fn try_set_result(&self, result: T) -> bool {
        self.0.try_set(TaskState::RanToCompletion(Some(result)))
    }

    fn try_set_exception(&self, exception: Exception) -> bool {
        self.0.try_set(TaskState::Faulted(Some(exception)))
    }

    fn try_set_canceled(&self) -> bool {
        self.0.try_set(TaskState::Canceled)
    }
}

impl<T> Drop for TaskCompletionSource<T> {
    fn drop(&mut self) {
        self.try_set_canceled();
    }
}

/// The outcome of an action dispatched to a [`HeadlessUnitTestSession`];
/// what the original returns as a task object.
pub struct HeadlessUnitTestTask<T> {
    shared: Arc<TaskShared<T>>,
}

impl<T> HeadlessUnitTestTask<T> {
    /// Whether the task has finished: completed, failed or cancelled.
    pub fn is_completed(&self) -> bool {
        !matches!(*self.shared.lock(), TaskState::Running)
    }

    /// Blocks until the task has finished and takes its result
    /// (`GetAwaiter().GetResult()`). Not to be called on the session thread,
    /// which is the thread that has to make progress.
    ///
    /// # Panics
    /// Raises the panic of the dispatched action, or of the cleanup after
    /// it, again. Panics when the outcome has already been taken.
    pub fn wait(self) -> Result<T, OperationCanceledError> {
        let mut state = self.shared.lock();
        while matches!(*state, TaskState::Running) {
            state = self.shared.finished.wait(state).unwrap_or_else(PoisonError::into_inner);
        }
        Self::take(&mut state)
    }

    /// Blocks until the task has finished or `timeout` has passed
    /// (`WaitAsync(TimeSpan)`).
    ///
    /// # Panics
    /// Panics when the task has not finished in time (`TimeoutException`);
    /// otherwise as [`wait`](Self::wait).
    pub fn wait_timeout(self, timeout: Duration) -> Result<T, OperationCanceledError> {
        let state = self.shared.lock();
        let (mut state, _) = self
            .shared
            .finished
            .wait_timeout_while(state, timeout, |state| matches!(*state, TaskState::Running))
            .unwrap_or_else(PoisonError::into_inner);
        if matches!(*state, TaskState::Running) {
            drop(state);
            panic!("The operation has timed out.");
        }
        Self::take(&mut state)
    }

    fn take(state: &mut TaskState<T>) -> Result<T, OperationCanceledError> {
        match state {
            TaskState::RanToCompletion(result) => match result.take() {
                Some(result) => Ok(result),
                None => panic!("The result of the task has already been taken."),
            },
            TaskState::Faulted(exception) => match exception.take() {
                Some(exception) => resume_unwind(exception),
                None => panic!("The failure of the task has already been observed."),
            },
            TaskState::Canceled => Err(OperationCanceledError),
            TaskState::Running => unreachable!("the task has finished"),
        }
    }
}

// --- the queue of the session --------------------------------------------------------------------

/// What the consumer loop owns and a work item runs with: the fields of the
/// original session that cannot leave the session thread.
struct SessionThread {
    app_builder: AppBuilder,
    /// Only set and used with PerAssembly isolation
    shared_context: RefCell<Option<Arc<FerroSynchronizationContext>>>,
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
    fn take(&self, cancellation_token_source: &CancellationTokenSource) -> Option<WorkItem> {
        let mut state = self.lock();
        loop {
            if cancellation_token_source.is_cancellation_requested() {
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
    cancellation_token_source: CancellationTokenSource,
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

/// Cancels the source when dropped: when the future it is a local of has
/// run to its end, has panicked or was dropped.
struct CancelOnDrop(CancellationTokenSource);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

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
        if self.cancellation_token_source.is_cancellation_requested() {
            panic!("Cannot access a disposed object.\nObject name: 'Session was already disposed.'.");
        }

        let token = self.cancellation_token_source.token();
        let isolated = self.isolated;

        let (tcs, task) = TaskCompletionSource::<Fut::Output>::new();
        self.queue.add(Box::new(move |session: &SessionThread| {
            let cts = CancellationTokenSource::new();
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
                    tcs.try_set_exception(ex);
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
                // of the original, which cancels the source when the task has finished.
                let future = action();
                let guard = CancelOnDrop(cts.clone());
                let task = Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
                    let _guard = guard;
                    future.await
                });
                if task.is_completed() {
                    return match task.result() {
                        Ok(result) => Some(result),
                        Err(canceled) => std::panic::panic_any(canceled),
                    };
                }

                if cts.is_cancellation_requested() {
                    should_cancel = true;
                    None
                } else {
                    let frame = DispatcherFrame::new();
                    let inner_cts = {
                        let frame = frame.clone();
                        cts.token().register(move || frame.set_continue(false))
                    };
                    Dispatcher::ui_thread().push_frame(&frame);
                    inner_cts.dispose();
                    match task.result() {
                        Ok(result) => Some(result),
                        Err(canceled) => std::panic::panic_any(canceled),
                    }
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
                tcs.try_set_exception(caught);
            } else if should_cancel {
                tcs.try_set_canceled();
            } else if let Some(result) = result {
                tcs.try_set_result(result);
            }
        }));
        task
    }

    pub fn dispose(&self) {
        self.cancellation_token_source.cancel();
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
        let cancellation_token_source = CancellationTokenSource::new();
        let queue = Arc::new(BlockingQueue::default());

        let task = {
            let cancellation_token_source = cancellation_token_source.clone();
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

                        SessionThread { app_builder, shared_context: RefCell::new(None) }
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

                    while !cancellation_token_source.is_cancellation_requested() {
                        // `None` is the `OperationCanceledException` of the original.
                        if let Some(action) = queue.take(&cancellation_token_source) {
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
            cancellation_token_source,
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

impl SessionThread {
    fn ensure_shared_application(&self) -> Box<dyn IDisposable> {
        let old_context = FerroSynchronizationContext::current();
        if Application::current().is_none() {
            self.app_builder.setup_unsafe();
            *self.shared_context.borrow_mut() = FerroSynchronizationContext::current();
        } else {
            FerroSynchronizationContext::set_current(self.shared_context.borrow().clone());
        }

        Box::new(Cleanup::new(move || {
            let jobs = catch_unwind(|| Dispatcher::ui_thread().run_jobs(None));
            FerroSynchronizationContext::set_current(old_context);
            if let Err(ex) = jobs {
                resume_unwind(ex);
            }
        }))
    }

    fn ensure_isolated_application(&self) -> Box<dyn IDisposable> {
        let scope = FerroLocator::enter_scope();
        let old_context = FerroSynchronizationContext::current();
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
            FerroSynchronizationContext::set_current(old_context);
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
