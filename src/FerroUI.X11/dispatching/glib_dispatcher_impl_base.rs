//! Platform-agnostic GLib (GMainLoop/GSource) based dispatcher implementation
//! (the port of `GlibDispatcherImplBase.cs`). It maps the dispatcher model of
//! the framework onto GLib so its code can share a GLib main loop with
//! GLib/GTK based libraries on the UI thread. It owns no platform event
//! source by itself; backends attach their own (e.g. the X11 socket).
//!
//! GLib priorities and the priorities of the framework are a bit different.
//! The framework follows the WPF model when there are "background" and
//! "foreground" priority groups. Foreground jobs are executed before any
//! user input processing, background jobs are executed strictly after user
//! input processing.
//!
//! GLib has numeric priorities that are used in the following way:
//!
//! ```text
//! -100    G_PRIORITY_HIGH - "high" priority sources, not really used by GLib/GTK
//! 0       G_PRIORITY_DEFAULT - polling X11 events (GTK) and default value for g_timeout_add
//! 100     G_PRIORITY_HIGH_IDLE without a clear definition, used as an anchor value of sorts
//! 110     Resize/layout operations (GTK)
//! 120     Render operations (GTK)
//! 200     G_PRIORITY_DEFAULT_IDLE - "idle" priority sources
//! ```
//!
//! So, unlike the framework, GTK puts way higher priority on input
//! processing, then does resize/layout/render.
//!
//! So, to map our model to GLib we do the following:
//! - foreground jobs (including grouped user events) are executed with (-1)
//!   priority (_before_ any normal GLib jobs)
//! - the platform event source is polled with G_PRIORITY_DEFAULT, all events
//!   are read until the socket is empty, we also group input events at that
//!   stage (this matches our epoll-based dispatcher)
//! - background jobs are executed with G_PRIORITY_DEFAULT_IDLE, so they would
//!   have lower priority than GTK foreground jobs
//!
//! Unfortunately we can't detect if there are pending _non-idle_ GLib jobs
//! using g_main_context_pending, since
//! - g_main_context_pending doesn't accept max_priority argument
//! - even if it did, that would still involve a syscall to the kernel to
//!   poll for fds anyway
//!
//! So we just report that we don't support pending input query and let the
//! dispatcher to call RequestBackgroundProcessing every time, which results
//! in g_idle_add call for every background job. Background jobs are expected
//! to be relatively expensive to execute since on Windows
//! MsgWaitForMultipleObjectsEx results isn't really free too.
//!
//! For signaling (aka waking up dispatcher for processing _high_ priority
//! jobs we are using g_idle_add_full with (-1) priority. While the naming
//! suggests that it would enqueue an idle job, it actually adds an
//! always-triggered source that would be called before other sources with
//! lower priority.
//!
//! For timers we are using a simple timeout source and discard the previous
//! one when dispatcher requests an update.
//!
//! Since GLib dispatches event sources in batches, we force-check for
//! "signaled" flag to run high-prio jobs whenever we get control back from
//! GLib. We can still occasionally get GTK code to run before high-prio
//! jobs of the framework, but that should be fine since the point is to keep
//! the jobs of the framework ordered properly and to not have our
//! low-priority jobs to prevent GLib-based code from running its own
//! "foreground" jobs.
//!
//! Another implementation note here is that GLib (just as any other C
//! library) is NOT aware of panics, so a panic is NOT allowed to unwind
//! through a callback GLib made. So the callbacks catch them and try to
//! propagate those to the nearest run loop frame that was initiated by the
//! framework. If there is no such frame, we have no choice but to
//! log/swallow those.

use crate::interop::glib::{panic_message, GMainLoop, Glib, G_PRIORITY_DEFAULT};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::threading::{
    CancellationToken, CancellationTokenSource, DispatcherImplEvent, IControlledDispatcherImpl, IDispatcherImpl,
    IDispatcherImplWithExplicitBackgroundProcessing, IDispatcherImplWithPendingInput, IDispatcherSignal,
};
use ferroui_base::utilities::ThreadBound;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, ThreadId};
use std::time::Instant;

/// What a panic carries.
pub type PanicPayload = Box<dyn Any + Send>;

/// The part a backend overrides (the virtual members of the class of the
/// reference).
pub trait GlibDispatcherBackend {
    /// Flush any pending output to the platform event source after jobs of
    /// the framework ran. Called whenever control returns to GLib from a
    /// signaled/timer callback. Backends that own a socket (e.g. X11) flush
    /// their connection.
    fn flush(&self);
}

#[derive(Default)]
struct SignalState {
    signaled: bool,
    signaled_source_added: bool,
}

/// The part other threads reach: the flags under their lock, and the way
/// back to the dispatcher implementation for the source the signal adds,
/// which runs on the thread of the main loop.
struct GlibSignal {
    glib: Glib,
    lock: Mutex<SignalState>,
    owner: ThreadBound<Weak<GlibDispatcherImplBase>>,
    this: std::sync::Weak<GlibSignal>,
}

impl GlibSignal {
    /// `SignalSourceCallback`.
    fn signal_source_callback(&self) -> bool {
        self.lock.lock().unwrap_or_else(PoisonError::into_inner).signaled_source_added = false;
        // Note that we can't use g_main_context_is_owner outside a run loop: the source runs
        // on whatever thread iterates the default context, which is assumed to be the thread
        // the platform was initialized on. On another thread the source does nothing.
        if self.owner.is_on_thread() {
            if let Some(owner) = self.owner.get().upgrade() {
                owner.check_signaled();
            }
        }
        false
    }
}

impl IDispatcherSignal for GlibSignal {
    fn signal(&self) {
        {
            let mut state = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
            if state.signaled {
                return;
            }
            state.signaled = true;
            if state.signaled_source_added {
                return;
            }
            state.signaled_source_added = true;
        }
        let Some(this) = self.this.upgrade() else {
            return;
        };
        self.glib.g_idle_add_full(G_PRIORITY_DEFAULT - 1, Box::new(move || this.signal_source_callback()));
    }
}

/// One run of the main loop the framework started (`ManagedLoopFrame`).
struct ManagedLoopFrame {
    external_token: CancellationToken,
    internal_token_source: RefCell<Option<CancellationTokenSource>>,
    cancelled: RefCell<CancellationToken>,
    // Shared with the callback of the cancellation, which may come from another thread:
    // the loop lives as long as either holds it, where the reference guards a disposed
    // flag with a lock.
    main_loop: Arc<GMainLoop>,
    exceptions: RefCell<Vec<PanicPayload>>,
}

impl ManagedLoopFrame {
    fn new(glib: Glib, token: CancellationToken) -> Self {
        Self {
            external_token: token,
            internal_token_source: RefCell::new(None),
            cancelled: RefCell::new(CancellationToken::none()),
            main_loop: Arc::new(glib.g_main_loop_new(true)),
            exceptions: RefCell::new(Vec::new()),
        }
    }

    fn stop(&self) {
        let source = self.internal_token_source.borrow().clone();
        if let Some(source) = source {
            source.cancel();
        }
    }

    fn run(&self) {
        if self.external_token.is_cancellation_requested() {
            return;
        }
        // The token of the frame is canceled by the token of the caller and by `stop` (the
        // linked token source of the reference).
        let internal = CancellationTokenSource::new();
        *self.internal_token_source.borrow_mut() = Some(internal.clone());
        let link = internal.clone();
        let external_registration = self.external_token.register(move || link.cancel());
        *self.cancelled.borrow_mut() = internal.token();
        let main_loop = self.main_loop.clone();
        let registration = internal.token().register(move || main_loop.quit());

        self.main_loop.run();

        registration.dispose();
        external_registration.dispose();
        *self.internal_token_source.borrow_mut() = None;
    }
}

/// The dispatcher implementation over the main loop of GLib.
pub struct GlibDispatcherImplBase {
    glib: Glib,
    // The app author is assumed to initialize the framework on the intended UI thread and
    // not to migrate the default run loop to a different thread.
    main_thread: ThreadId,
    external_exception_logger: Option<Rc<dyn Fn(&(dyn Any + Send))>>,
    signal: Arc<GlibSignal>,
    run_loop_stack: RefCell<Vec<Rc<ManagedLoopFrame>>>,
    stopwatch: Instant,
    glib_timer_source_tag: Cell<Option<u32>>,
    backend: RefCell<Option<Weak<dyn GlibDispatcherBackend>>>,
    signaled: DispatcherImplEvent,
    timer: DispatcherImplEvent,
    ready_for_background_processing: DispatcherImplEvent,
    this: Weak<GlibDispatcherImplBase>,
}

impl GlibDispatcherImplBase {
    pub fn new(glib: Glib, external_exception_logger: Option<Rc<dyn Fn(&(dyn Any + Send))>>) -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<Self>| Self {
            glib,
            main_thread: thread::current().id(),
            external_exception_logger,
            signal: Arc::new_cyclic(|signal| GlibSignal {
                glib,
                lock: Mutex::new(SignalState::default()),
                owner: ThreadBound::new(this.clone()),
                this: signal.clone(),
            }),
            run_loop_stack: RefCell::new(Vec::new()),
            stopwatch: Instant::now(),
            glib_timer_source_tag: Cell::new(None),
            backend: RefCell::new(None),
            signaled: DispatcherImplEvent::new(),
            timer: DispatcherImplEvent::new(),
            ready_for_background_processing: DispatcherImplEvent::new(),
            this: this.clone(),
        })
    }

    /// GLib.
    pub fn glib(&self) -> Glib {
        self.glib
    }

    /// Sets the object that has the overrides of a backend.
    pub fn set_backend(&self, backend: Weak<dyn GlibDispatcherBackend>) {
        *self.backend.borrow_mut() = Some(backend);
    }

    /// A callback for a source of GLib that reaches this object on its
    /// thread and does nothing anywhere else, or when the object is gone.
    fn callback(&self, call: impl Fn(&GlibDispatcherImplBase) + 'static) -> Box<dyn FnOnce() + Send> {
        let bound = ThreadBound::new((self.this.clone(), call));
        Box::new(move || {
            if bound.is_on_thread() {
                let (this, call) = bound.get();
                if let Some(this) = this.upgrade() {
                    call(&this);
                }
            }
        })
    }

    pub fn check_signaled(&self) {
        {
            let mut state = self.signal.lock.lock().unwrap_or_else(PoisonError::into_inner);
            if !state.signaled {
                return;
            }
            state.signaled = false;
        }

        if let Err(e) = catch_unwind(AssertUnwindSafe(|| self.signaled.raise())) {
            self.handle_exception(e);
        }
        self.flush();
    }

    fn timer_callback(&self) {
        // The source removes itself after this call. The reference keeps its tag and
        // removes it once more at the next update.
        self.glib_timer_source_tag.set(None);
        if let Err(e) = catch_unwind(AssertUnwindSafe(|| self.timer.raise())) {
            self.handle_exception(e);
        }
        self.flush();
    }

    fn flush(&self) {
        let backend = self.backend.borrow().as_ref().and_then(Weak::upgrade);
        if let Some(backend) = backend {
            backend.flush();
        }
    }

    /// The cancellation token of the innermost run loop frame the
    /// framework controls, or one that is never canceled when no frame is
    /// running. Backends use it to stop draining their event source once
    /// a loop frame asked to quit.
    pub fn current_loop_cancellation(&self) -> CancellationToken {
        match self.run_loop_stack.borrow().last() {
            Some(frame) => frame.cancelled.borrow().clone(),
            None => CancellationToken::none(),
        }
    }

    pub fn handle_exception(&self, e: PanicPayload) {
        let frame = self.run_loop_stack.borrow().last().cloned();
        if let Some(frame) = frame {
            frame.exceptions.borrow_mut().push(e);
            frame.stop();
        } else if let Some(logger) = &self.external_exception_logger {
            logger(&*e);
        } else if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Control") {
            logger.log_with_values(None, "Unhandled exception: {exception}", &[&panic_message(&*e)]);
        }
    }

    fn elapsed_milliseconds(&self) -> i64 {
        self.stopwatch.elapsed().as_millis() as i64
    }
}

impl IDispatcherImpl for GlibDispatcherImplBase {
    fn current_thread_is_loop_thread(&self) -> bool {
        self.main_thread == thread::current().id()
    }

    fn signal(&self) {
        self.signal.signal();
    }

    fn signal_handle(&self) -> Arc<dyn IDispatcherSignal> {
        self.signal.clone()
    }

    fn signaled(&self) -> &DispatcherImplEvent {
        &self.signaled
    }

    fn timer(&self) -> &DispatcherImplEvent {
        &self.timer
    }

    fn now(&self) -> i64 {
        self.elapsed_milliseconds()
    }

    fn update_timer(&self, due_time_in_ms: Option<i64>) {
        if let Some(tag) = self.glib_timer_source_tag.take() {
            self.glib.g_source_remove(tag);
        }

        let Some(due_time_in_ms) = due_time_in_ms else {
            return;
        };

        let interval = (due_time_in_ms - self.now()).clamp(0, i32::MAX as i64) as u32;
        let tag = self.glib.g_timeout_add_once(interval, self.callback(Self::timer_callback));
        self.glib_timer_source_tag.set(Some(tag));
    }

    fn as_pending_input(&self) -> Option<&dyn IDispatcherImplWithPendingInput> {
        Some(self)
    }

    fn as_explicit_background_processing(&self) -> Option<&dyn IDispatcherImplWithExplicitBackgroundProcessing> {
        Some(self)
    }

    fn as_controlled(&self) -> Option<&dyn IControlledDispatcherImpl> {
        Some(self)
    }
}

impl IDispatcherImplWithExplicitBackgroundProcessing for GlibDispatcherImplBase {
    fn ready_for_background_processing(&self) -> &DispatcherImplEvent {
        &self.ready_for_background_processing
    }

    fn request_background_processing(&self) {
        // The reference raises the event without a handler for what it throws; here a panic
        // goes where the panics of the other callbacks go.
        self.glib.g_idle_add_once(self.callback(|this| {
            if let Err(e) = catch_unwind(AssertUnwindSafe(|| this.ready_for_background_processing.raise())) {
                this.handle_exception(e);
            }
        }));
    }
}

impl IDispatcherImplWithPendingInput for GlibDispatcherImplBase {
    fn can_query_pending_input(&self) -> bool {
        false
    }

    fn has_pending_input(&self) -> bool {
        false
    }
}

impl IControlledDispatcherImpl for GlibDispatcherImplBase {
    fn run_loop(&self, token: CancellationToken) {
        if token.is_cancellation_requested() {
            return;
        }

        let frame = Rc::new(ManagedLoopFrame::new(self.glib, token));
        self.run_loop_stack.borrow_mut().push(frame.clone());
        frame.run();
        self.run_loop_stack.borrow_mut().pop();

        // Propagate any panics that we've captured from this frame. The reference throws
        // several as one aggregate; here the first goes on and the others are logged.
        let mut exceptions = std::mem::take(&mut *frame.exceptions.borrow_mut());
        if exceptions.is_empty() {
            return;
        }
        let first = exceptions.remove(0);
        for e in exceptions {
            if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Control") {
                logger.log_with_values(None, "Unhandled exception: {exception}", &[&panic_message(&*e)]);
            }
        }
        resume_unwind(first);
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file. They need GLib and return
    // at once without it (the development machine).
    use super::*;
    use crate::interop::glib::tests::main_context_lock;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn events(dispatcher: &Rc<GlibDispatcherImplBase>) -> Rc<RefCell<Vec<&'static str>>> {
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        dispatcher.signaled().add(Rc::new(move |()| l.borrow_mut().push("signaled")));
        let l = log.clone();
        dispatcher.timer().add(Rc::new(move |()| l.borrow_mut().push("timer")));
        let l = log.clone();
        dispatcher.ready_for_background_processing().add(Rc::new(move |()| l.borrow_mut().push("background")));
        log
    }

    #[test]
    fn signals_timers_and_background_requests_run_in_the_loop_in_their_order() {
        let Ok(glib) = Glib::try_get() else {
            return;
        };
        let _lock = main_context_lock();
        let dispatcher = GlibDispatcherImplBase::new(glib, None);
        let log = events(&dispatcher);
        assert!(dispatcher.current_thread_is_loop_thread());
        assert!(!dispatcher.can_query_pending_input() && !dispatcher.has_pending_input());

        let source = CancellationTokenSource::new();
        let (l, s) = (log.clone(), source.clone());
        dispatcher.timer().add(Rc::new(move |()| {
            // The timer is the last of the three: it ends the loop.
            l.borrow_mut().push("stop");
            s.cancel();
        }));

        // Background work is asked for first and runs after the signal, which has the
        // higher priority; the timer is due later.
        dispatcher.request_background_processing();
        dispatcher.signal();
        dispatcher.signal();
        dispatcher.update_timer(Some(dispatcher.now() + 30));
        assert!(log.borrow().is_empty());

        dispatcher.run_loop(source.token());
        assert_eq!(*log.borrow(), ["signaled", "background", "timer", "stop"]);

        // A loop with a canceled token does not run at all.
        dispatcher.signal();
        dispatcher.run_loop(source.token());
        assert_eq!(log.borrow().len(), 4);
    }

    #[test]
    fn a_signal_from_another_thread_wakes_the_loop() {
        let Ok(glib) = Glib::try_get() else {
            return;
        };
        let _lock = main_context_lock();
        let dispatcher = GlibDispatcherImplBase::new(glib, None);
        let source = CancellationTokenSource::new();
        let s = source.clone();
        let ui_thread = thread::current().id();
        dispatcher.signaled().add(Rc::new(move |()| {
            assert_eq!(thread::current().id(), ui_thread);
            s.cancel();
        }));
        let handle = dispatcher.signal_handle();
        let other = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(20));
            handle.signal();
        });
        dispatcher.run_loop(source.token());
        other.join().unwrap();
        assert!(source.is_cancellation_requested());
    }

    #[test]
    fn a_timer_that_is_replaced_or_cleared_does_not_fire() {
        let Ok(glib) = Glib::try_get() else {
            return;
        };
        let _lock = main_context_lock();
        let dispatcher = GlibDispatcherImplBase::new(glib, None);
        let log = events(&dispatcher);
        let source = CancellationTokenSource::new();

        dispatcher.update_timer(Some(dispatcher.now() + 5));
        dispatcher.update_timer(None);
        dispatcher.update_timer(Some(dispatcher.now() + 10));
        dispatcher.update_timer(Some(dispatcher.now() + 40));
        let started = dispatcher.now();
        let s = source.clone();
        dispatcher.timer().add(Rc::new(move |()| s.cancel()));
        dispatcher.run_loop(source.token());
        assert_eq!(*log.borrow(), ["timer"]);
        assert!(dispatcher.now() - started >= 35, "the timer fired early");
    }

    #[test]
    fn a_panic_in_a_callback_ends_the_frame_and_goes_on_from_run_loop() {
        let Ok(glib) = Glib::try_get() else {
            return;
        };
        let _lock = main_context_lock();
        let dispatcher = GlibDispatcherImplBase::new(glib, None);
        dispatcher.signaled().add(Rc::new(|()| panic!("from a job")));
        dispatcher.signal();
        let d = dispatcher.clone();
        let result = catch_unwind(AssertUnwindSafe(move || d.run_loop(CancellationToken::none())));
        let payload = result.expect_err("the panic of the job leaves the loop");
        assert_eq!(panic_message(&*payload), "from a job");
        assert!(dispatcher.run_loop_stack.borrow().is_empty());
    }

    #[test]
    fn without_a_frame_a_panic_goes_to_the_logger_of_the_options() {
        let Ok(glib) = Glib::try_get() else {
            return;
        };
        let _lock = main_context_lock();
        let seen = Rc::new(RefCell::new(None));
        let s = seen.clone();
        let dispatcher = GlibDispatcherImplBase::new(
            glib,
            Some(Rc::new(move |payload: &(dyn Any + Send)| *s.borrow_mut() = Some(panic_message(payload)))),
        );
        dispatcher.signaled().add(Rc::new(|()| panic!("outside a frame")));
        dispatcher.signal();

        // A main loop that is not a frame of the dispatcher: an application's own.
        let main_loop = Arc::new(glib.g_main_loop_new(true));
        let l = main_loop.clone();
        let done = Arc::new(AtomicBool::new(false));
        let d = done.clone();
        glib.g_timeout_add_once(
            20,
            Box::new(move || {
                d.store(true, Ordering::SeqCst);
                l.quit();
            }),
        );
        main_loop.run();
        assert!(done.load(Ordering::SeqCst));
        assert_eq!(seen.borrow().as_deref(), Some("outside a frame"));
    }
}
