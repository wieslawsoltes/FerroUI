//! The dispatcher platform implementation over the native run loop.

use crate::interop::*;
use ferroui_base::threading::{
    CancellationToken, CancellationTokenSource, DispatcherImplEvent, IControlledDispatcherImpl, IDispatcherImpl,
    IDispatcherImplWithExplicitBackgroundProcessing, IDispatcherImplWithPendingInput, IDispatcherSignal,
};
use ferroui_microcom::ComPtr;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::panic::resume_unwind;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, ThreadId};
use std::time::Instant;

thread_local! {
    /// The dispatcher implementation that belongs to this thread; what
    /// callbacks hand their panics to.
    static CURRENT: RefCell<Weak<DispatcherImpl>> = const { RefCell::new(Weak::new()) };
}

/// The cross-thread wake-up handle.
struct NativeSignal(ComPtr<IFrnPlatformThreadingInterface>);

// SAFETY: the only member used through this handle is `Signal`, which the
// native side implements for any thread (it takes a lock, queues a block on
// the main queue and wakes the main run loop). Reference counting of native
// objects is atomic.
unsafe impl Send for NativeSignal {}
// SAFETY: see `Send`.
unsafe impl Sync for NativeSignal {}

impl IDispatcherSignal for NativeSignal {
    fn signal(&self) {
        self.0.signal();
    }
}

/// The native loop cancellation, shared with the cancellation callback that
/// may run on any thread.
struct LoopCancellation(Mutex<LoopCancellationState>);

struct LoopCancellationState {
    exited: bool,
    cancel: Option<ComPtr<IFrnLoopCancellation>>,
}

// SAFETY: `IFrnLoopCancellation::Cancel` may be called from any thread (off
// the main thread the native side re-posts itself to the main queue), and
// the state is only touched under the mutex.
unsafe impl Send for LoopCancellation {}
// SAFETY: see `Send`.
unsafe impl Sync for LoopCancellation {}

impl LoopCancellation {
    fn cancel(&self) {
        let state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if !state.exited {
            if let Some(cancel) = &state.cancel {
                cancel.cancel();
            }
        }
    }

    /// Marks the loop as exited and hands the native object back so that it
    /// is released on the loop thread.
    fn exit(&self) -> Option<ComPtr<IFrnLoopCancellation>> {
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        state.exited = true;
        state.cancel.take()
    }
}

/// One nested run of the native loop.
struct RunLoopFrame {
    exception: RefCell<Option<Box<dyn Any + Send>>>,
    cancellation_token_source: CancellationTokenSource,
}

/// The dispatcher platform implementation of the macOS backend.
pub struct DispatcherImpl {
    native: ComPtr<IFrnPlatformThreadingInterface>,
    signal: Arc<NativeSignal>,
    loop_thread: Cell<Option<ThreadId>>,
    clock: Instant,
    managed_frames: RefCell<Vec<Rc<RunLoopFrame>>>,
    signaled: DispatcherImplEvent,
    timer: DispatcherImplEvent,
    ready_for_background_processing: DispatcherImplEvent,
}

/// Receives the events of the native threading interface.
struct Events {
    parent: Weak<DispatcherImpl>,
}

impl IFrnPlatformThreadingInterfaceEventsImpl for Events {
    fn signaled(&self) {
        crate::callback_base::guard((), || {
            if let Some(parent) = self.parent.upgrade() {
                parent.signaled.raise();
            }
        })
    }

    fn timer(&self) {
        crate::callback_base::guard((), || {
            if let Some(parent) = self.parent.upgrade() {
                parent.timer.raise();
            }
        })
    }

    fn ready_for_background_processing(&self) {
        crate::callback_base::guard((), || {
            if let Some(parent) = self.parent.upgrade() {
                parent.ready_for_background_processing.raise();
            }
        })
    }
}

impl DispatcherImpl {
    /// Creates the implementation for the thread that owns `native` (the
    /// calling thread) and subscribes to the native events.
    pub fn new(native: ComPtr<IFrnPlatformThreadingInterface>) -> Rc<DispatcherImpl> {
        let this = Rc::new(DispatcherImpl {
            signal: Arc::new(NativeSignal(native.clone())),
            native,
            loop_thread: Cell::new(None),
            clock: Instant::now(),
            managed_frames: RefCell::new(Vec::new()),
            signaled: DispatcherImplEvent::new(),
            timer: DispatcherImplEvent::new(),
            ready_for_background_processing: DispatcherImplEvent::new(),
        });
        let events = IFrnPlatformThreadingInterfaceEvents::from_impl(Events { parent: Rc::downgrade(&this) });
        this.native.set_events(Some(&events));
        CURRENT.with(|current| *current.borrow_mut() = Rc::downgrade(&this));
        this
    }

    /// The implementation that belongs to the calling thread, if any.
    pub(crate) fn current() -> Option<Rc<DispatcherImpl>> {
        CURRENT.try_with(|current| current.borrow().upgrade()).ok().flatten()
    }

    /// The timer interval the native side is asked for: `-1` clears the
    /// timer, otherwise the time left until `due_time_in_ms`, at least one
    /// millisecond.
    fn timer_interval(due_time_in_ms: Option<i64>, now: i64) -> i32 {
        match due_time_in_ms {
            None => -1,
            Some(due_time) => (due_time - now).max(1).min((i32::MAX - 10) as i64) as i32,
        }
    }

    /// Stops the innermost run loop and makes it re-raise `capture`, the
    /// payload of a panic caught in a native callback.
    pub fn propagate_callback_exception(&self, capture: Box<dyn Any + Send>) {
        let frame = self.managed_frames.borrow().last().cloned();
        let Some(frame) = frame else {
            debug_assert!(false, "We should never get here");
            return;
        };

        *frame.exception.borrow_mut() = Some(capture);
        frame.cancellation_token_source.cancel();
    }
}

impl IDispatcherImpl for DispatcherImpl {
    fn current_thread_is_loop_thread(&self) -> bool {
        if let Some(loop_thread) = self.loop_thread.get() {
            return thread::current().id() == loop_thread;
        }
        if !self.native.get_current_thread_is_loop_thread() {
            return false;
        }
        self.loop_thread.set(Some(thread::current().id()));
        true
    }

    fn signal(&self) {
        self.native.signal();
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
        self.clock.elapsed().as_millis() as i64
    }

    fn update_timer(&self, due_time_in_ms: Option<i64>) {
        self.native.update_timer(Self::timer_interval(due_time_in_ms, self.now()));
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

impl IDispatcherImplWithPendingInput for DispatcherImpl {
    fn can_query_pending_input(&self) -> bool {
        false
    }

    fn has_pending_input(&self) -> bool {
        false
    }
}

impl IDispatcherImplWithExplicitBackgroundProcessing for DispatcherImpl {
    fn ready_for_background_processing(&self) -> &DispatcherImplEvent {
        &self.ready_for_background_processing
    }

    fn request_background_processing(&self) {
        self.native.request_background_processing();
    }
}

impl IControlledDispatcherImpl for DispatcherImpl {
    fn run_loop(&self, token: CancellationToken) {
        if token.is_cancellation_requested() {
            return;
        }

        // The frame's source is linked to `token` and can also be canceled
        // by `propagate_callback_exception`.
        let frame = Rc::new(RunLoopFrame {
            exception: RefCell::new(None),
            cancellation_token_source: CancellationTokenSource::new(),
        });
        let linked = frame.cancellation_token_source.clone();
        let link_registration = token.register(move || linked.cancel());

        let cancel = self.native.create_loop_cancellation();
        let shared = Arc::new(LoopCancellation(Mutex::new(LoopCancellationState { exited: false, cancel: cancel.clone() })));
        let on_cancel = shared.clone();
        let cancel_registration = frame.cancellation_token_source.token().register(move || on_cancel.cancel());

        self.managed_frames.borrow_mut().push(frame.clone());
        self.native.run_loop(cancel.as_deref());

        // finally
        drop(shared.exit());
        self.managed_frames.borrow_mut().pop();
        cancel_registration.dispose();
        link_registration.dispose();
        let exception = frame.exception.borrow_mut().take();
        if let Some(exception) = exception {
            resume_unwind(exception);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_interval_matches_the_reference_clamping() {
        assert_eq!(DispatcherImpl::timer_interval(None, 100), -1);
        assert_eq!(DispatcherImpl::timer_interval(Some(150), 100), 50);
        // Due or overdue timers fire as soon as possible, never "now".
        assert_eq!(DispatcherImpl::timer_interval(Some(100), 100), 1);
        assert_eq!(DispatcherImpl::timer_interval(Some(10), 100), 1);
        assert_eq!(DispatcherImpl::timer_interval(Some(i64::MAX), 0), i32::MAX - 10);
    }
}
