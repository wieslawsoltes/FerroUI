//! The dispatcher implementation of the platform over the main loop of
//! GLib (the port of `GLibDispatcherImpl.cs`): the connection to the server
//! is a file descriptor source of the default main context. It replaces
//! `X11PlatformThreading` when the option `use_g_lib_main_loop` is set, for
//! applications that use GLib-based libraries on the main thread.

use super::glib_dispatcher_impl_base::{GlibDispatcherBackend, GlibDispatcherImplBase};
use super::{IX11PlatformDispatcher, X11EventDispatcher};
use crate::interop::glib::{GIOCondition, Glib, G_PRIORITY_DEFAULT};
use crate::x11_platform::FerroX11Platform;
use ferroui_base::threading::{
    CancellationToken, DispatcherImplEvent, IControlledDispatcherImpl, IDispatcherImpl,
    IDispatcherImplWithExplicitBackgroundProcessing, IDispatcherImplWithPendingInput, IDispatcherSignal,
};
use ferroui_base::utilities::ThreadBound;
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// The dispatcher implementation of the X11 platform on the GLib main
/// loop.
pub struct GlibDispatcherImpl {
    base: Rc<GlibDispatcherImplBase>,
    platform: Weak<FerroX11Platform>,
    x11_events: Rc<X11EventDispatcher>,
    /// Whether a source that hands out the events Xlib has queued is
    /// waiting to run.
    queued_events_source_added: Cell<bool>,
    this: Weak<GlibDispatcherImpl>,
}

impl GlibDispatcherImpl {
    /// # Panics
    /// When the libraries of GLib cannot be loaded: the option asks for a
    /// main loop that does not exist (the reference fails with the
    /// exception of the first function of the library it calls).
    pub fn new(platform: &Rc<FerroX11Platform>) -> Rc<Self> {
        let glib = match Glib::try_get() {
            Ok(glib) => glib,
            Err(error) => panic!("X11PlatformOptions::use_g_lib_main_loop needs GLib: {error}"),
        };
        let base =
            GlibDispatcherImplBase::new(glib, platform.options().external_g_lib_main_loop_exception_logger.clone());
        let x11_events = X11EventDispatcher::new(platform);
        let this = Rc::new_cyclic(|this| Self {
            base,
            platform: Rc::downgrade(platform),
            x11_events,
            queued_events_source_added: Cell::new(false),
            this: this.clone(),
        });
        let backend: Rc<dyn GlibDispatcherBackend> = this.clone();
        this.base.set_backend(Rc::downgrade(&backend));

        // The source runs on the thread that iterates the default main context, which is
        // this one; anywhere else it does nothing.
        let bound = ThreadBound::new(Rc::downgrade(&this));
        let unix_fd_id = glib.g_unix_fd_add_full(
            G_PRIORITY_DEFAULT,
            this.x11_events.fd(),
            GIOCondition::G_IO_IN,
            Box::new(move |fd, condition| {
                if !bound.is_on_thread() {
                    return true;
                }
                match bound.get().upgrade() {
                    Some(this) => this.x11_source_callback(fd, condition),
                    None => false,
                }
            }),
        );
        // We can trigger a nested event loop when handling X11 events, so we need to mark the source as recursive
        glib.g_source_set_can_recurse(unix_fd_id, true);
        this
    }

    fn x11_source_callback(&self, _fd: i32, _gio_condition: GIOCondition) -> bool {
        self.base.check_signaled();
        let token = self.base.current_loop_cancellation();
        let Some(platform) = self.platform.upgrade() else {
            return false;
        };
        let queue = platform.event_grouper_dispatch_queue().clone();
        let result = catch_unwind(AssertUnwindSafe(|| {
            // Completely drain X11 socket while we are at it
            while self.x11_events.is_pending() {
                // If we don't actually drain our X11 socket, GLib _will_ call us again even if
                // we request the run loop to quit
                self.x11_events.dispatch_x11_events(&CancellationToken::none());
                if !token.is_cancellation_requested() {
                    while queue.has_jobs() {
                        self.base.check_signaled();
                        queue.dispatch_next();
                    }

                    self.x11_events.flush();
                }
            }
        }));
        if let Err(e) = result {
            self.base.handle_exception(e);
        }

        true
    }
}

impl GlibDispatcherBackend for GlibDispatcherImpl {
    fn flush(&self) {
        self.x11_events.flush();
        self.dispatch_queued_events_later();
    }
}

impl GlibDispatcherImpl {
    /// Not in the reference (DEVIATIONS.md). The source of the connection
    /// runs when the socket can be read. A job of the framework that waits
    /// for an answer of the server (`XSync`, a property that is read) makes
    /// Xlib read the socket and queue the events that came before the
    /// answer: the socket is then empty, the source does not run, and the
    /// events stay in the queue until something else arrives. So after
    /// jobs ran, events that are queued get a source of their own, at the
    /// priority of the source of the connection.
    fn dispatch_queued_events_later(&self) {
        if self.queued_events_source_added.get() || !self.x11_events.is_pending() {
            return;
        }
        self.queued_events_source_added.set(true);
        let bound = ThreadBound::new(self.this.clone());
        self.base.glib().g_idle_add_full(
            G_PRIORITY_DEFAULT,
            Box::new(move || {
                if bound.is_on_thread() {
                    if let Some(this) = bound.get().upgrade() {
                        this.queued_events_source_added.set(false);
                        this.x11_source_callback(this.x11_events.fd(), GIOCondition::G_IO_IN);
                    }
                }
                false
            }),
        );
    }
}

impl IDispatcherImpl for GlibDispatcherImpl {
    fn current_thread_is_loop_thread(&self) -> bool {
        self.base.current_thread_is_loop_thread()
    }

    fn signal(&self) {
        self.base.signal();
    }

    fn signal_handle(&self) -> Arc<dyn IDispatcherSignal> {
        self.base.signal_handle()
    }

    fn signaled(&self) -> &DispatcherImplEvent {
        self.base.signaled()
    }

    fn timer(&self) -> &DispatcherImplEvent {
        self.base.timer()
    }

    fn now(&self) -> i64 {
        self.base.now()
    }

    fn update_timer(&self, due_time_in_ms: Option<i64>) {
        self.base.update_timer(due_time_in_ms);
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

impl IDispatcherImplWithExplicitBackgroundProcessing for GlibDispatcherImpl {
    fn ready_for_background_processing(&self) -> &DispatcherImplEvent {
        self.base.ready_for_background_processing()
    }

    fn request_background_processing(&self) {
        self.base.request_background_processing();
    }
}

impl IDispatcherImplWithPendingInput for GlibDispatcherImpl {
    fn can_query_pending_input(&self) -> bool {
        self.base.can_query_pending_input()
    }

    fn has_pending_input(&self) -> bool {
        self.platform.upgrade().is_some_and(|platform| platform.event_grouper_dispatch_queue().has_jobs())
            || self.x11_events.is_pending()
    }
}

impl IControlledDispatcherImpl for GlibDispatcherImpl {
    fn run_loop(&self, token: CancellationToken) {
        self.base.run_loop(token);
    }
}

impl IX11PlatformDispatcher for GlibDispatcherImpl {
    fn event_dispatcher(&self) -> &Rc<X11EventDispatcher> {
        &self.x11_events
    }
}
