use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::thread::{self, ThreadId};

use super::dispatcher_operation::LocalCallback;
use super::dispatcher_priority_queue::DispatcherPriorityQueue;
use super::dispatcher_task::LocalTask;
use super::i_dispatcher_impl::{DetachedDispatcherImpl, DetachedDispatcherSignal};
use super::{
    DispatcherFrame, DispatcherPriority, DispatcherTimer, DispatcherUnhandledExceptionEventArgs,
    DispatcherUnhandledExceptionFilterEventArgs, FerroSynchronizationContext, IDispatcher, IDispatcherImpl,
    IDispatcherSignal,
};
use crate::utilities::HandlerList;

/// Handler of [`Dispatcher::unhandled_exception`].
pub type DispatcherUnhandledExceptionEventHandler = dyn for<'a> Fn(&DispatcherUnhandledExceptionEventArgs<'a>);

/// Handler of [`Dispatcher::unhandled_exception_filter`].
pub type DispatcherUnhandledExceptionFilterEventHandler =
    dyn for<'a> Fn(&DispatcherUnhandledExceptionFilterEventArgs<'a>);

/// The part of a dispatcher that never leaves its thread: everything that
/// holds `Rc`-based or otherwise non-`Send` values. It lives in thread-local
/// storage, so it needs no locking and cannot be reached from other threads.
pub(crate) struct DispatcherLocal {
    pub(crate) dispatcher_id: u64,
    /// The platform implementation; it belongs to this thread.
    pub(crate) impl_: RefCell<Rc<dyn IDispatcherImpl>>,
    attached: Cell<bool>,
    timer_token: Cell<u64>,
    signaled_token: Cell<u64>,
    background_token: Cell<Option<u64>>,
    /// The per-priority synchronization contexts, created on demand.
    pub(crate) priority_contexts: RefCell<Vec<Option<Arc<FerroSynchronizationContext>>>>,
    /// The futures running on this dispatcher.
    pub(crate) tasks: RefCell<HashMap<u64, LocalTask>>,
    /// Callbacks of operations queued with the `*_local` methods.
    pub(crate) jobs: RefCell<HashMap<u64, LocalCallback>>,
    /// Results of completed local operations that have not been taken yet.
    pub(crate) results: RefCell<HashMap<u64, Box<dyn Any>>>,
    pub(crate) timers: RefCell<Vec<Rc<DispatcherTimer>>>,
    pub(crate) timers_version: Cell<i64>,
    pub(crate) shutdown_started: HandlerList<dyn Fn(&Arc<Dispatcher>)>,
    pub(crate) shutdown_finished: HandlerList<dyn Fn(&Arc<Dispatcher>)>,
    pub(crate) unhandled_exception: HandlerList<DispatcherUnhandledExceptionEventHandler>,
    pub(crate) unhandled_exception_filter: HandlerList<DispatcherUnhandledExceptionFilterEventHandler>,
    /// Addresses of panic payloads this dispatcher has already filtered and
    /// that are still propagating through its frames.
    pub(crate) seen_exceptions: RefCell<Vec<usize>>,
    /// Number of operations currently executing (nested frames).
    pub(crate) executing_depth: Cell<u32>,
}

impl DispatcherLocal {
    fn new(dispatcher_id: u64) -> Self {
        Self {
            dispatcher_id,
            impl_: RefCell::new(Rc::new(DetachedDispatcherImpl::new())),
            attached: Cell::new(false),
            timer_token: Cell::new(0),
            signaled_token: Cell::new(0),
            background_token: Cell::new(None),
            priority_contexts: RefCell::new(Vec::new()),
            tasks: RefCell::new(HashMap::new()),
            jobs: RefCell::new(HashMap::new()),
            results: RefCell::new(HashMap::new()),
            timers: RefCell::new(Vec::new()),
            timers_version: Cell::new(0),
            shutdown_started: HandlerList::new(),
            shutdown_finished: HandlerList::new(),
            unhandled_exception: HandlerList::new(),
            unhandled_exception_filter: HandlerList::new(),
            seen_exceptions: RefCell::new(Vec::new()),
            executing_depth: Cell::new(0),
        }
    }
}

/// Everything guarded by the dispatcher's instance lock.
pub(crate) struct DispatcherState {
    /// Wakes the dispatcher thread up; the only part of the platform
    /// implementation that other threads touch.
    pub(crate) signal: Arc<dyn IDispatcherSignal>,
    pub(crate) supports_run_loops: bool,
    pub(crate) initialized: bool,

    // Queue
    pub(crate) queue: DispatcherPriorityQueue,
    pub(crate) signaled: bool,
    pub(crate) explicit_background_processing_requested: bool,
    pub(crate) maximum_input_starvation_time: i64,
    /// Local operations that were discarded on another thread; their
    /// thread-local storage is released by the dispatcher thread.
    pub(crate) dead_local_operations: Vec<u64>,

    // Timers
    pub(crate) due_time_found: bool,
    pub(crate) due_time_in_ms: i64,
    pub(crate) due_time_for_timers: Option<i64>,
    pub(crate) due_time_for_background_processing: Option<i64>,
    pub(crate) os_timer_set_to: Option<i64>,

    // Main loop
    pub(crate) disabled_processing_count: i32,
    pub(crate) has_shutdown_finished: bool,
    pub(crate) starting_shutdown: bool,
    pub(crate) frames: Vec<Arc<DispatcherFrame>>,
}

static NEXT_DISPATCHER_ID: AtomicU64 = AtomicU64::new(1);

/// Provides services for managing work items on a thread.
///
/// # Threading
///
/// The object model is single-threaded and `Rc`-based; the dispatcher is the
/// one entry point that can be used from other threads. A dispatcher is
/// therefore handed out as `Arc<Dispatcher>` and is `Send + Sync`, and its
/// API is split in two:
///
/// * [`post`](Self::post), [`invoke`](Self::invoke),
///   [`invoke_async`](Self::invoke_async) and their variants can be called
///   from **any thread**. Their callbacks (and results) must be `Send`.
/// * [`post_local`](Self::post_local), [`invoke_local`](Self::invoke_local),
///   [`invoke_async_local`](Self::invoke_async_local) and their variants
///   accept callbacks that are not `Send` (they may capture `Rc`s, `Ref`s,
///   ...). They can only be called **on the dispatcher thread**, which is
///   checked at run time: calling them from another thread panics. The
///   callback is kept in thread-local storage and never leaves the thread.
///
/// Timers ([`DispatcherTimer`]), the shutdown and unhandled-exception events
/// and everything else that takes non-`Send` handlers is likewise bound to
/// the dispatcher thread. So is the platform implementation
/// ([`IDispatcherImpl`]): other threads only ever use its
/// [`IDispatcherSignal`].
///
/// # Async
///
/// The dispatcher is also an executor for futures, which takes the place of
/// the synchronization context of the reference implementation:
/// [`invoke_async_task`](Self::invoke_async_task) and
/// [`DispatcherTaskScheduler`](super::DispatcherTaskScheduler) run a future
/// on the dispatcher thread, polling it from jobs queued at a fixed priority;
/// [`resume`](Self::resume), [`yield_now`](Self::yield_now) and
/// [`await_with_priority`](Self::await_with_priority) change the priority at
/// which an `async` block continues. No async runtime is involved.
///
/// # Panics in callbacks
///
/// What the reference implementation does with exceptions is done here with
/// panics: a panic in a posted callback goes through
/// [`unhandled_exception_filter`](Self::unhandled_exception_filter) and
/// [`unhandled_exception`](Self::unhandled_exception) and is re-raised on the
/// dispatcher thread unless a handler marks it as handled; a panic in an
/// `invoke`/`invoke_async` callback is captured and re-raised to whoever
/// waits for the operation. On targets that abort on panic none of this
/// applies.
pub struct Dispatcher {
    pub(crate) id: u64,
    thread: ThreadId,
    pub(crate) weak_self: Weak<Dispatcher>,
    state: Mutex<DispatcherState>,
    pub(crate) exit_all_frames_requested: AtomicBool,
    pub(crate) has_shutdown_started: AtomicBool,
}

impl Dispatcher {
    /// Creates the dispatcher of the current thread.
    ///
    /// # Panics
    /// Panics when the thread already has a dispatcher or when `impl_`
    /// belongs to a different thread.
    pub(crate) fn new(impl_: Option<Rc<dyn IDispatcherImpl>>) -> Arc<Dispatcher> {
        #[cfg(debug_assertions)]
        {
            use crate::LocatorExtensions;
            let locator = crate::FerroLocator::current();
            if locator.get_service::<dyn IDispatcherImpl>().is_some()
                || locator.get_service::<dyn crate::platform::IPlatformThreadingInterface>().is_some()
            {
                panic!("Registering IDispatcherImpl or IPlatformThreadingInterface via locator is no longer valid");
            }
        }

        if let Some(impl_) = &impl_ {
            if !impl_.current_thread_is_loop_thread() {
                panic!("IDispatcherImpl belongs to a different thread");
            }
        }

        let id = NEXT_DISPATCHER_ID.fetch_add(1, Ordering::Relaxed);
        let dispatcher = Arc::new_cyclic(|weak_self| Dispatcher {
            id,
            thread: current_thread_id(),
            weak_self: weak_self.clone(),
            state: Mutex::new(DispatcherState {
                // Set by replace_implementation
                signal: Arc::new(DetachedDispatcherSignal),
                supports_run_loops: false,
                initialized: false,
                queue: DispatcherPriorityQueue::new(),
                signaled: false,
                explicit_background_processing_requested: false,
                maximum_input_starvation_time: Self::MAXIMUM_INPUT_STARVATION_TIME_IN_FALLBACK_MODE,
                dead_local_operations: Vec::new(),
                due_time_found: false,
                due_time_in_ms: 0,
                due_time_for_timers: None,
                due_time_for_background_processing: None,
                os_timer_set_to: None,
                disabled_processing_count: 0,
                has_shutdown_finished: false,
                starting_shutdown: false,
                frames: Vec::new(),
            }),
            exit_all_frames_requested: AtomicBool::new(false),
            has_shutdown_started: AtomicBool::new(false),
        });

        // The first created dispatcher becomes "UI thread one"
        Self::register_current_thread_dispatcher(&dispatcher, Rc::new(DispatcherLocal::new(id)));

        dispatcher.replace_implementation(impl_);
        dispatcher
    }

    /// Whether the platform implementation can run nested event loops. When
    /// this is `false`, [`push_frame`](Self::push_frame) and
    /// [`main_loop`](Self::main_loop) are not available.
    pub fn supports_run_loops(&self) -> bool {
        self.lock().supports_run_loops
    }

    /// Checks that the current thread is the dispatcher thread.
    #[inline]
    pub fn check_access(&self) -> bool {
        current_thread_id() == self.thread
    }

    /// Checks that the current thread is the dispatcher thread and panics if
    /// not.
    #[inline]
    #[track_caller]
    pub fn verify_access(&self) {
        if !self.check_access() {
            Self::throw_verify_access();
        }
    }

    #[cold]
    #[inline(never)]
    #[track_caller]
    fn throw_verify_access() -> ! {
        panic!("The calling thread cannot access this object because a different thread owns it.");
    }

    /// The thread this dispatcher belongs to.
    pub fn thread(&self) -> ThreadId {
        self.thread
    }

    /// The platform implementation currently in use.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread:
    /// the implementation belongs to that thread.
    pub fn platform_impl(&self) -> Rc<dyn IDispatcherImpl> {
        self.verify_access();
        self.local_impl().unwrap_or_else(|| Rc::new(DetachedDispatcherImpl::new()))
    }

    /// The platform implementation, when called on the dispatcher thread
    /// and the dispatcher has not been reset.
    pub(crate) fn local_impl(&self) -> Option<Rc<dyn IDispatcherImpl>> {
        self.try_local().map(|local| local.impl_.borrow().clone())
    }

    /// Takes the instance lock.
    pub(crate) fn lock(&self) -> MutexGuard<'_, DispatcherState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A strong handle to this dispatcher.
    pub(crate) fn to_arc(&self) -> Arc<Dispatcher> {
        self.weak_self.upgrade().expect("the dispatcher is alive while it is borrowed")
    }

    /// The thread-local part of this dispatcher, when called on its thread
    /// and it has not been reset.
    pub(crate) fn try_local(&self) -> Option<Rc<DispatcherLocal>> {
        if !self.check_access() {
            return None;
        }
        Self::current_thread_local().filter(|local| local.dispatcher_id == self.id)
    }

    /// Releases the thread-local storage of a local operation that will not
    /// run or whose result will not be read.
    pub(crate) fn discard_local_operation(&self, operation_id: u64) {
        if self.check_access() {
            if let Some(local) = self.try_local() {
                let job = local.jobs.borrow_mut().remove(&operation_id);
                let result = local.results.borrow_mut().remove(&operation_id);
                drop(job);
                drop(result);
            }
        } else {
            self.lock().dead_local_operations.push(operation_id);
        }
    }

    pub(crate) fn replace_implementation(&self, impl_: Option<Rc<dyn IDispatcherImpl>>) {
        if let Some(impl_) = &impl_ {
            if !impl_.current_thread_is_loop_thread() {
                panic!("IDispatcherImpl belongs to a different thread");
            }
        }
        self.verify_access();
        let Some(local) = self.try_local() else {
            return;
        };

        let mut time_shift = None;
        {
            let mut st = self.lock();
            let mut old_now = None;

            if local.attached.get() {
                // Not attached in the constructor
                old_now = Some(local.impl_.borrow().now());
                Self::detach_implementation(&local);
            }

            let impl_ = match impl_ {
                Some(impl_) => {
                    st.initialized = true;
                    impl_
                }
                None => default_dispatcher_impl(),
            };
            *local.impl_.borrow_mut() = impl_.clone();
            st.signal = impl_.signal_handle();
            st.supports_run_loops = impl_.as_controlled().is_some();

            let weak = self.weak_self.clone();
            local.timer_token.set(impl_.timer().add(Rc::new(move |()| {
                if let Some(dispatcher) = weak.upgrade() {
                    dispatcher.on_os_timer();
                }
            })));
            let weak = self.weak_self.clone();
            local.signaled_token.set(impl_.signaled().add(Rc::new(move |()| {
                if let Some(dispatcher) = weak.upgrade() {
                    dispatcher.signaled();
                }
            })));
            local.attached.set(true);

            let background = impl_.as_explicit_background_processing();
            st.maximum_input_starvation_time = if background.is_none() {
                Self::MAXIMUM_INPUT_STARVATION_TIME_IN_FALLBACK_MODE
            } else {
                Self::MAXIMUM_INPUT_STARVATION_TIME_IN_EXPLICIT_PROCESSING_EXPLICIT_MODE
            };
            if let Some(background) = background {
                let weak = self.weak_self.clone();
                local.background_token.set(Some(background.ready_for_background_processing().add(Rc::new(
                    move |()| {
                        if let Some(dispatcher) = weak.upgrade() {
                            dispatcher.on_ready_for_explicit_background_processing();
                        }
                    },
                ))));
            }

            // All time comes from the implementation, so due times computed
            // with the previous implementation's clock are moved to the new
            // clock.
            if let Some(old_now) = old_now {
                let shift = impl_.now().wrapping_sub(old_now);
                if shift != 0 {
                    st.due_time_in_ms = st.due_time_in_ms.wrapping_add(shift);
                    st.due_time_for_timers = st.due_time_for_timers.map(|due| due.wrapping_add(shift));
                    st.due_time_for_background_processing =
                        st.due_time_for_background_processing.map(|due| due.wrapping_add(shift));
                    time_shift = Some(shift);
                }
            }

            if st.signaled {
                impl_.signal();
            }
            if st.explicit_background_processing_requested {
                match background {
                    Some(background) => background.request_background_processing(),
                    None => {
                        // The request cannot be repeated to an implementation
                        // without explicit background processing; fall back to
                        // the timer so that queued background jobs still run.
                        st.explicit_background_processing_requested = false;
                        self.request_background_processing_locked(&mut st);
                    }
                }
            }

            st.os_timer_set_to = None;
            self.update_os_timer_locked(&mut st);
        }

        if let Some(shift) = time_shift {
            for timer in local.timers.borrow().iter() {
                timer.shift_due_time(shift);
            }
        }
    }

    /// Unsubscribes from the current implementation's events.
    pub(crate) fn detach_implementation(local: &DispatcherLocal) {
        if !local.attached.get() {
            return;
        }
        let impl_ = local.impl_.borrow().clone();
        impl_.timer().remove(local.timer_token.get());
        impl_.signaled().remove(local.signaled_token.get());
        if let Some(token) = local.background_token.take() {
            if let Some(background) = impl_.as_explicit_background_processing() {
                background.ready_for_background_processing().remove(token);
            }
        }
        local.attached.set(false);
    }

    /// Replaces the implementation with one that ignores everything, without
    /// touching the queue. Used while tearing a dispatcher down. `local` is
    /// absent when the thread-local part is already gone.
    pub(crate) fn detach_from_platform(st: &mut DispatcherState, local: Option<&DispatcherLocal>) {
        if let Some(local) = local {
            Self::detach_implementation(local);
            let previous = local.impl_.replace(Rc::new(DetachedDispatcherImpl::new()));
            drop(previous);
        }
        st.signal = Arc::new(DetachedDispatcherSignal);
        st.supports_run_loops = false;
    }
}

impl IDispatcher for Dispatcher {
    fn check_access(&self) -> bool {
        Dispatcher::check_access(self)
    }

    fn verify_access(&self) {
        Dispatcher::verify_access(self)
    }

    fn post(&self, action: Box<dyn FnOnce() + Send>, priority: DispatcherPriority) {
        Dispatcher::post(self, action, priority)
    }
}

impl std::fmt::Debug for Dispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dispatcher").field("id", &self.id).field("thread", &self.thread).finish()
    }
}

/// The implementation used until the platform installs its own one.
fn default_dispatcher_impl() -> Rc<dyn IDispatcherImpl> {
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    {
        Rc::new(crate::platform::ManagedDispatcherImpl::new(None))
    }
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    {
        // No clock and no blocking primitives: queue work until the platform
        // implementation is installed, which then receives the pending
        // signal and timer requests.
        Rc::new(DetachedDispatcherImpl::new())
    }
}

thread_local! {
    static THREAD_ID: ThreadId = thread::current().id();
}

/// The id of the calling thread, without the cost of `thread::current()`.
#[inline]
pub(crate) fn current_thread_id() -> ThreadId {
    THREAD_ID.try_with(|id| *id).unwrap_or_else(|_| thread::current().id())
}
