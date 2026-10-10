//! The dispatcher on the run loop of the main thread.
//!
//! Work is signalled through the main dispatch queue, the timer is a timer
//! of the run loop, and an observer of the run loop gives background
//! processing its turn before the loop waits.

use crate::interop;
use ferroui_base::threading::{
    DispatcherImplEvent, IDispatcherImpl, IDispatcherImplWithExplicitBackgroundProcessing, IDispatcherSignal,
};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

// The documentation of `CFRunLoopTimerSetNextFireDate` recommends to
// "create a repeating timer with an initial firing time in the distant
// future (or the initial firing time) and a very large repeat interval, on
// the order of decades or more".
const DISTANT_FUTURE_INTERVAL: f64 = 50.0 * 365.0 * 24.0 * 3600.0;

thread_local! {
    /// The dispatcher of the main thread. The callbacks of the run loop and
    /// of the main queue run on the main thread and find it here.
    static INSTANCE: RefCell<Option<Rc<DispatcherImpl>>> = const { RefCell::new(None) };
}

/// What another thread needs to wake the dispatcher: the flag, the run loop
/// of the main thread and the main queue.
struct MainLoopSignal {
    signaled: AtomicBool,
    main_loop: *mut c_void,
    main_queue: *const interop::DispatchQueue,
}

// SAFETY: the two pointers are the run loop of the main thread and the main
// dispatch queue, which live as long as the process, and the only calls
// made with them from here are `CFRunLoopWakeUp` and `dispatch_async_f`,
// both documented as callable from any thread.
unsafe impl Send for MainLoopSignal {}
// SAFETY: see `Send`; the flag is atomic.
unsafe impl Sync for MainLoopSignal {}

impl IDispatcherSignal for MainLoopSignal {
    fn signal(&self) {
        if self.signaled.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return;
        }
        // SAFETY: the queue and the run loop are valid for the life of the
        // process, the function is a plain function that takes no context,
        // and both calls may be made on any thread.
        unsafe {
            interop::dispatch_async_f(self.main_queue, std::ptr::null_mut(), check_signaled_callback);
            interop::CFRunLoopWakeUp(self.main_loop);
        }
    }
}

/// The dispatcher implementation of the platform.
pub struct DispatcherImpl {
    clock: Instant,
    timer: *mut c_void,
    signal: Arc<MainLoopSignal>,
    background_processing_requested: Cell<bool>,
    signaled: DispatcherImplEvent,
    timer_event: DispatcherImplEvent,
    ready_for_background_processing: DispatcherImplEvent,
}

impl DispatcherImpl {
    /// The dispatcher of the main thread, created by the first call.
    ///
    /// # Panics
    /// Panics when called on another thread than the main thread: the
    /// dispatcher is an object of the thread whose run loop it sits on.
    pub fn instance() -> Rc<DispatcherImpl> {
        if !is_main_thread() {
            panic!("The dispatcher of the platform belongs to the main thread.");
        }
        if let Some(instance) = INSTANCE.with(|instance| instance.borrow().clone()) {
            return instance;
        }
        let instance = Rc::new(Self::new());
        INSTANCE.with(|slot| *slot.borrow_mut() = Some(instance.clone()));
        instance
    }

    fn new() -> DispatcherImpl {
        // SAFETY: plain calls into Core Foundation on the main thread. The
        // observer and the timer are created without a context (the
        // callbacks find the dispatcher in the thread-local), are added to
        // the run loop of the main thread, which retains them, and are
        // never removed: the dispatcher lives as long as the process.
        let (main_loop, timer) = unsafe {
            let main_loop = interop::CFRunLoopGetMain();

            let observer = interop::CFRunLoopObserverCreate(
                std::ptr::null(),
                interop::kCFRunLoopAfterWaiting | interop::kCFRunLoopBeforeSources | interop::kCFRunLoopBeforeWaiting,
                1,
                0,
                observer_callback,
                std::ptr::null_mut(),
            );
            interop::CFRunLoopAddObserver(main_loop, observer, interop::kCFRunLoopDefaultMode);

            let timer = interop::CFRunLoopTimerCreate(
                std::ptr::null(),
                interop::CFAbsoluteTimeGetCurrent() + DISTANT_FUTURE_INTERVAL,
                DISTANT_FUTURE_INTERVAL,
                0,
                0,
                timer_callback,
                std::ptr::null_mut(),
            );
            interop::CFRunLoopAddTimer(main_loop, timer, interop::kCFRunLoopDefaultMode);
            (main_loop, timer)
        };

        DispatcherImpl {
            clock: Instant::now(),
            timer,
            signal: Arc::new(MainLoopSignal {
                signaled: AtomicBool::new(false),
                main_loop,
                main_queue: interop::dispatch_get_main_queue(),
            }),
            background_processing_requested: Cell::new(false),
            signaled: DispatcherImplEvent::new(),
            timer_event: DispatcherImplEvent::new(),
            ready_for_background_processing: DispatcherImplEvent::new(),
        }
    }

    fn check_signaled(&self) {
        if self.signal.signaled.swap(false, Ordering::SeqCst) {
            self.signaled.raise();
        }
    }
}

fn is_main_thread() -> bool {
    // SAFETY: the function takes nothing and reads a property of the
    // calling thread.
    unsafe { libc::pthread_main_np() != 0 }
}

/// The dispatcher, for a callback of the main thread. The handle is taken
/// out of the thread-local before the handlers run, so that a handler may
/// reach the dispatcher again.
fn with_instance(f: impl FnOnce(&DispatcherImpl)) {
    let instance = INSTANCE.with(|instance| instance.borrow().clone());
    if let Some(instance) = instance {
        f(&instance);
    }
}

extern "C" fn check_signaled_callback(_context: *mut c_void) {
    with_instance(|instance| instance.check_signaled());
}

extern "C" fn wake_up_callback(_context: *mut c_void) {}

extern "C" fn observer_callback(_observer: *mut c_void, activity: interop::CFOptionFlags, _info: *mut c_void) {
    with_instance(|instance| {
        if activity == interop::kCFRunLoopBeforeWaiting {
            let trigger_processing = instance.background_processing_requested.replace(false);
            if trigger_processing {
                instance.ready_for_background_processing.raise();
            }
        }

        instance.check_signaled();
    });
}

extern "C" fn timer_callback(_timer: *mut c_void, _info: *mut c_void) {
    with_instance(|instance| instance.timer_event.raise());
}

/// The time until the timer of the run loop is due, in seconds, for a due
/// time of the dispatcher and its clock (both in milliseconds).
pub(crate) fn timer_interval(due_time_in_ms: Option<i64>, now: i64) -> f64 {
    let ms = match due_time_in_ms {
        None => -1,
        Some(due_time) => (due_time - now).clamp(1, i64::from(i32::MAX - 10)) as i32,
    };
    if ms < 0 {
        DISTANT_FUTURE_INTERVAL
    } else {
        f64::from(ms) / 1000.0
    }
}

impl IDispatcherImpl for DispatcherImpl {
    fn current_thread_is_loop_thread(&self) -> bool {
        is_main_thread()
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
        &self.timer_event
    }

    fn now(&self) -> i64 {
        self.clock.elapsed().as_millis() as i64
    }

    fn update_timer(&self, due_time_in_ms: Option<i64>) {
        let interval = timer_interval(due_time_in_ms, self.now());
        // SAFETY: the timer was created by this object and is retained by
        // the run loop of the main thread; the dispatcher calls this on the
        // main thread.
        unsafe {
            interop::CFRunLoopTimerSetTolerance(self.timer, 0.0);
            interop::CFRunLoopTimerSetNextFireDate(self.timer, interop::CFAbsoluteTimeGetCurrent() + interval);
        }
    }

    fn as_explicit_background_processing(&self) -> Option<&dyn IDispatcherImplWithExplicitBackgroundProcessing> {
        Some(self)
    }
}

impl IDispatcherImplWithExplicitBackgroundProcessing for DispatcherImpl {
    fn ready_for_background_processing(&self) -> &DispatcherImplEvent {
        &self.ready_for_background_processing
    }

    fn request_background_processing(&self) {
        if self.background_processing_requested.replace(true) {
            return;
        }
        // SAFETY: as in `MainLoopSignal::signal`; the function does nothing
        // and exists to make the run loop take a turn.
        unsafe { interop::dispatch_async_f(self.signal.main_queue, std::ptr::null_mut(), wake_up_callback) };
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file. The loop
    // itself is driven by `tests/dispatcher_main_loop.rs`, on the main
    // thread.
    use super::*;

    #[test]
    fn no_due_time_puts_the_timer_in_the_distant_future() {
        assert_eq!(DISTANT_FUTURE_INTERVAL, timer_interval(None, 100));
    }

    #[test]
    fn a_due_time_is_the_time_left_in_seconds() {
        assert_eq!(0.25, timer_interval(Some(350), 100));
    }

    #[test]
    fn a_due_time_that_has_passed_fires_in_one_millisecond() {
        assert_eq!(0.001, timer_interval(Some(100), 100));
        assert_eq!(0.001, timer_interval(Some(10), 100));
    }

    #[test]
    fn a_due_time_beyond_the_range_of_the_timer_is_clamped() {
        assert_eq!(f64::from(i32::MAX - 10) / 1000.0, timer_interval(Some(i64::MAX), 0));
    }

    #[test]
    #[should_panic(expected = "belongs to the main thread")]
    fn the_dispatcher_is_refused_off_the_main_thread() {
        // The tests of the harness run on threads of their own.
        let _ = DispatcherImpl::instance();
    }
}
