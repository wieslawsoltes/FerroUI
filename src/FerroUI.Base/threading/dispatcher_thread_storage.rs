use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::thread::{self, ThreadId};

use super::dispatcher::current_thread_id;

use super::dispatcher::DispatcherLocal;
use super::{Dispatcher, DispatcherOperationStatus, DispatcherPriority, FerroSynchronizationContext, IDispatcherImpl};

struct GlobalState {
    ui_thread: Option<Arc<Dispatcher>>,
    dispatchers: Vec<(ThreadId, Weak<Dispatcher>)>,
}

static GLOBAL: Mutex<GlobalState> = Mutex::new(GlobalState { ui_thread: None, dispatchers: Vec::new() });

/// Incremented whenever the UI thread dispatcher changes; validates the
/// per-thread cache used by [`Dispatcher::ui_thread`].
static UI_THREAD_GENERATION: AtomicU64 = AtomicU64::new(1);

fn global() -> MutexGuard<'static, GlobalState> {
    GLOBAL.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Keeps the dispatcher of a thread alive for as long as the thread runs.
struct CurrentThreadDispatcher(Arc<Dispatcher>);

impl Drop for CurrentThreadDispatcher {
    fn drop(&mut self) {
        self.0.abandon();
    }
}

thread_local! {
    static CURRENT_THREAD_DISPATCHER: RefCell<Option<CurrentThreadDispatcher>> = const { RefCell::new(None) };
    static CURRENT_THREAD_LOCAL: RefCell<Option<Rc<DispatcherLocal>>> = const { RefCell::new(None) };
    static UI_THREAD_CACHE: RefCell<(u64, Option<Arc<Dispatcher>>)> = const { RefCell::new((0, None)) };
    static UNIT_TEST_ISOLATED: Cell<bool> = const { Cell::new(false) };
}

fn is_unit_test_isolated() -> bool {
    UNIT_TEST_ISOLATED.try_with(Cell::get).unwrap_or(false)
}

impl Dispatcher {
    pub(crate) fn register_current_thread_dispatcher(dispatcher: &Arc<Dispatcher>, local: Rc<DispatcherLocal>) {
        let thread = current_thread_id();
        {
            let mut global = global();
            if global.dispatchers.iter().any(|(id, weak)| *id == thread && weak.strong_count() > 0) {
                panic!("The current thread already has a dispatcher");
            }

            // The first created dispatcher becomes "UI thread one"
            if global.ui_thread.is_none() && !is_unit_test_isolated() {
                global.ui_thread = Some(dispatcher.clone());
                UI_THREAD_GENERATION.fetch_add(1, Ordering::AcqRel);
            }

            global.dispatchers.retain(|(id, weak)| *id != thread && weak.strong_count() > 0);
            global.dispatchers.push((thread, Arc::downgrade(dispatcher)));
        }
        CURRENT_THREAD_LOCAL.with(|slot| *slot.borrow_mut() = Some(local));
        let previous = CURRENT_THREAD_DISPATCHER
            .with(|slot| slot.borrow_mut().replace(CurrentThreadDispatcher(dispatcher.clone())));
        drop(previous);
    }

    pub(crate) fn current_thread_local() -> Option<Rc<DispatcherLocal>> {
        CURRENT_THREAD_LOCAL.try_with(|slot| slot.borrow().clone()).ok().flatten()
    }

    fn try_current_dispatcher() -> Option<Arc<Dispatcher>> {
        CURRENT_THREAD_DISPATCHER
            .try_with(|slot| slot.borrow().as_ref().map(|current| current.0.clone()))
            .ok()
            .flatten()
    }

    /// The dispatcher of the calling thread. It is created when the thread
    /// does not have one yet and then lives until the thread exits.
    pub fn current_dispatcher() -> Arc<Dispatcher> {
        if let Some(dispatcher) = Self::try_current_dispatcher() {
            return dispatcher;
        }

        Dispatcher::new(None)
    }

    /// The dispatcher of `thread`, if that thread has one.
    pub fn from_thread(thread: ThreadId) -> Option<Arc<Dispatcher>> {
        global().dispatchers.iter().find(|(id, _)| *id == thread).and_then(|(_, weak)| weak.upgrade())
    }

    /// Gets the dispatcher for the UI thread: the first dispatcher that was
    /// created in the process.
    ///
    /// Control and library authors are encouraged to use
    /// [`current_dispatcher`](Self::current_dispatcher) and the dispatcher of
    /// the object at hand instead.
    pub fn ui_thread() -> Arc<Dispatcher> {
        if is_unit_test_isolated() {
            return Self::current_dispatcher();
        }

        let generation = UI_THREAD_GENERATION.load(Ordering::Acquire);
        let cached = UI_THREAD_CACHE
            .try_with(|cache| {
                let cache = cache.borrow();
                if cache.0 == generation {
                    cache.1.clone()
                } else {
                    None
                }
            })
            .ok()
            .flatten();
        if let Some(dispatcher) = cached {
            return dispatcher;
        }

        Self::get_ui_thread_dispatcher_slow()
    }

    #[cold]
    fn get_ui_thread_dispatcher_slow() -> Arc<Dispatcher> {
        let existing = global().ui_thread.clone();
        let current = match existing {
            Some(_) => None,
            // Creating the dispatcher of this thread makes it the UI thread
            // one unless another thread got there first.
            None => Some(Self::current_dispatcher()),
        };

        let global = global();
        match &global.ui_thread {
            Some(dispatcher) => {
                let generation = UI_THREAD_GENERATION.load(Ordering::Acquire);
                let _ = UI_THREAD_CACHE.try_with(|cache| *cache.borrow_mut() = (generation, Some(dispatcher.clone())));
                dispatcher.clone()
            }
            None => {
                drop(global);
                current.unwrap_or_else(Self::current_dispatcher)
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn try_get_ui_thread() -> Option<Arc<Dispatcher>> {
        if is_unit_test_isolated() {
            return Self::try_current_dispatcher();
        }
        global().ui_thread.clone()
    }

    /// Installs the platform implementation of the UI thread dispatcher,
    /// adapting a legacy threading interface.
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    pub fn initialize_ui_thread_dispatcher_legacy(impl_: Rc<dyn crate::platform::IPlatformThreadingInterface>) {
        Self::initialize_ui_thread_dispatcher(Rc::new(super::i_dispatcher_impl::LegacyDispatcherImpl::new(impl_)));
    }

    /// Installs the platform implementation of the UI thread dispatcher.
    ///
    /// # Panics
    /// Panics when not called on the UI thread or when the UI thread
    /// dispatcher has already been initialized.
    pub fn initialize_ui_thread_dispatcher(impl_: Rc<dyn IDispatcherImpl>) {
        let ui_thread = Self::ui_thread();
        ui_thread.verify_access();
        if ui_thread.lock().initialized {
            panic!("UI thread dispatcher is already initialized");
        }
        ui_thread.replace_implementation(Some(impl_));
    }

    /// Forgets the dispatcher of the calling thread.
    fn reset_global_state() {
        let thread = current_thread_id();
        let current = CURRENT_THREAD_DISPATCHER.try_with(|slot| slot.borrow_mut().take()).ok().flatten();
        if let Some(current) = &current {
            // While the thread-local part is still reachable.
            current.0.abandon();
        }
        let local = CURRENT_THREAD_LOCAL.try_with(|slot| slot.borrow_mut().take()).ok().flatten();
        FerroSynchronizationContext::set_current(None);
        let _ = UI_THREAD_CACHE.try_with(|cache| *cache.borrow_mut() = (0, None));
        let ui_thread = {
            let mut global = global();
            global.dispatchers.retain(|(id, weak)| *id != thread && weak.strong_count() > 0);
            let is_ui_thread = match (&global.ui_thread, &current) {
                (Some(ui_thread), Some(current)) => Arc::ptr_eq(ui_thread, &current.0),
                (Some(ui_thread), None) => ui_thread.thread() == thread,
                _ => false,
            };
            if is_ui_thread {
                UI_THREAD_GENERATION.fetch_add(1, Ordering::AcqRel);
                global.ui_thread.take()
            } else {
                None
            }
        };
        // Dropped outside of the lock: releasing the dispatcher aborts
        // whatever is still queued.
        drop(ui_thread);
        drop(current);
        drop(local);
    }

    /// Detaches the dispatcher of the calling thread without running its
    /// pending jobs. Counterpart of [`reset_for_unit_tests`](Self::reset_for_unit_tests)
    /// for use before a test.
    #[doc(hidden)]
    pub fn reset_before_unit_tests() {
        Self::reset_global_state();
    }

    /// Runs the remaining active jobs of the calling thread's dispatcher,
    /// shuts it down and forgets it, so that the next use of the thread gets
    /// a fresh dispatcher.
    ///
    /// The reference implementation resets the process-wide UI dispatcher;
    /// tests here run in parallel on separate threads, so the reset is
    /// per-thread.
    #[doc(hidden)]
    pub fn reset_for_unit_tests() {
        let Some(dispatcher) = Self::try_current_dispatcher() else {
            Self::reset_global_state();
            return;
        };

        let mut executed = 0u32;
        loop {
            let job = {
                let local = dispatcher.try_local();
                let mut st = dispatcher.lock();
                Self::detach_from_platform(&mut st, local.as_deref());
                st.queue.peek()
            };
            executed += 1;
            if executed > 1_000_000 {
                panic!("You've caused dispatcher loop");
            }

            match job {
                Some(job) if job.priority() > DispatcherPriority::INACTIVE => dispatcher.execute_job(&job),
                _ => {
                    dispatcher.shutdown_impl();
                    Self::reset_global_state();
                    return;
                }
            }
        }
    }

    /// Makes the calling thread self-contained for the duration of a unit
    /// test: while the returned scope is alive,
    /// [`ui_thread`](Self::ui_thread) resolves to the dispatcher of the
    /// calling thread and dispatchers created on it never become the
    /// process-wide UI thread dispatcher. Dropping the scope resets the
    /// thread's dispatcher.
    #[doc(hidden)]
    pub fn unit_test_scope() -> UnitTestDispatcherScope {
        Self::reset_for_unit_tests();
        let previous = UNIT_TEST_ISOLATED.with(|isolated| isolated.replace(true));
        UnitTestDispatcherScope { previous }
    }

    /// Called when the owning thread exits (or the dispatcher is reset):
    /// nothing queued can run anymore, so everything is aborted, which also
    /// releases threads blocked on those operations.
    pub(crate) fn abandon(&self) {
        let local = self.try_local();
        let operations = {
            let mut st = self.lock();
            st.has_shutdown_finished = true;
            Self::detach_from_platform(&mut st, local.as_deref());
            st.queue.clear()
        };
        for operation in operations {
            if operation.status() == DispatcherOperationStatus::Pending {
                operation.set_status(DispatcherOperationStatus::Aborted);
                operation.call_abort_callbacks();
            }
        }
        self.cancel_tasks();
    }
}

/// See [`Dispatcher::unit_test_scope`].
#[doc(hidden)]
pub struct UnitTestDispatcherScope {
    previous: bool,
}

impl Drop for UnitTestDispatcherScope {
    fn drop(&mut self) {
        if thread::panicking() {
            // Do not run more user code while unwinding.
            Dispatcher::reset_global_state();
        } else {
            Dispatcher::reset_for_unit_tests();
        }
        let _ = UNIT_TEST_ISOLATED.try_with(|isolated| isolated.set(self.previous));
    }
}
