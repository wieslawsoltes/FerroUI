//! The dispatcher of the main looper.
//!
//! The reference holds a `Handler` of the main looper, three runnables and
//! an idle handler of the message queue. They are Java objects, held here
//! by one object of the Java layer (`MainLooperBridge`); the decisions are
//! made in this file.

use crate::interop::java::{
    call_boolean, call_static_boolean, call_void, new_object, JavaClass, JavaObject, JavaValue,
};
use crate::interop::natives::MAIN_LOOPER_BRIDGE;
use ferroui_base::threading::{
    DispatcherImplEvent, IDispatcherImpl, IDispatcherImplWithExplicitBackgroundProcessing,
    IDispatcherImplWithPendingInput, IDispatcherSignal,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

thread_local! {
    static IS_UI_THREAD: Cell<Option<bool>> = const { Cell::new(None) };
    static INSTANCE: RefCell<Weak<AndroidDispatcherImpl>> = const { RefCell::new(Weak::new()) };
}

/// The part of the dispatcher other threads use: posting the signal.
struct Signal {
    bridge: JavaObject,
    signaled: AtomicBool,
}

impl IDispatcherSignal for Signal {
    fn signal(&self) {
        if self.signaled.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return;
        }
        // A handler may be posted to from any thread; a thread the virtual machine does not
        // know is attached by the call.
        call_void(&self.bridge, "postSignal", "()V", &[]);
    }
}

pub struct AndroidDispatcherImpl {
    signal: Arc<Signal>,
    background_processing_requested: Cell<bool>,
    clock: Instant,
    timer: DispatcherImplEvent,
    signaled: DispatcherImplEvent,
    ready_for_background_processing: DispatcherImplEvent,
}

fn main_looper_is_current_thread() -> bool {
    call_static_boolean(&JavaClass::find(MAIN_LOOPER_BRIDGE), "isMainThread", "()Z", &[])
}

impl AndroidDispatcherImpl {
    /// # Panics
    /// Panics on another thread than the main thread of the application.
    pub fn new() -> Rc<AndroidDispatcherImpl> {
        if !main_looper_is_current_thread() {
            panic!("This class should be instanciated from the UI thread");
        }
        let bridge = new_object(&JavaClass::find(MAIN_LOOPER_BRIDGE), "()V", &[]).to_global();

        let this = Rc::new(AndroidDispatcherImpl {
            signal: Arc::new(Signal { bridge, signaled: AtomicBool::new(false) }),
            background_processing_requested: Cell::new(false),
            clock: Instant::now(),
            timer: DispatcherImplEvent::new(),
            signaled: DispatcherImplEvent::new(),
            ready_for_background_processing: DispatcherImplEvent::new(),
        });
        INSTANCE.with(|instance| *instance.borrow_mut() = Rc::downgrade(&this));
        this
    }

    fn instance() -> Option<Rc<AndroidDispatcherImpl>> {
        INSTANCE.with(|instance| instance.borrow().upgrade())
    }

    /// The timer runnable ran.
    pub(crate) fn on_timer() {
        if let Some(this) = Self::instance() {
            this.timer.raise();
        }
    }

    /// The signal runnable ran.
    pub(crate) fn on_signaled() {
        if let Some(this) = Self::instance() {
            this.signal.signaled.store(false, Ordering::SeqCst);
            this.signaled.raise();
        }
    }

    /// The message queue of the main looper is idle.
    pub(crate) fn on_idle() {
        let Some(this) = Self::instance() else {
            return;
        };

        loop {
            if this.background_processing_requested.replace(false) {
                this.ready_for_background_processing.raise();
            }

            if this.background_processing_requested.get() {
                // Dispatcher requested background processing again, however if the queue is empty and we
                // just return here, Android's Looper will go to sleep and won't call us again and we'll have
                // "background" jobs not being processed
                // So we need to examine the queue state to prevent that scenario

                if this.signal.signaled.load(Ordering::SeqCst) {
                    return;
                }

                if this.can_query_pending_input() {
                    if !this.has_pending_input() {
                        // There are no events in the queue, so if we just return here, Looper will go to sleep,
                        // so just run our logic again
                        continue;
                    }
                    // Nothing to do otherwise, we'll be called again after higher priority events get processed
                } else {
                    // On this API level we can't check if there is pending input,
                    // so we explicitly wake up the Looper to make sure that it will call idle hooks again
                    // before going to sleep
                    call_void(&this.signal.bridge, "postWakeup", "()V", &[]);
                }
            }
            return;
        }
    }
}

impl IDispatcherImpl for AndroidDispatcherImpl {
    fn current_thread_is_loop_thread(&self) -> bool {
        IS_UI_THREAD.with(|is_ui_thread| match is_ui_thread.get() {
            Some(value) => value,
            None => {
                let ui_thread = main_looper_is_current_thread();
                is_ui_thread.set(Some(ui_thread));
                ui_thread
            }
        })
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
        self.clock.elapsed().as_millis() as i64
    }

    fn update_timer(&self, due_time_in_ms: Option<i64>) {
        // A negative delay removes the timer, zero posts it at once.
        let delay = match due_time_in_ms {
            Some(due_time_in_ms) => (due_time_in_ms - self.now()).max(0),
            None => -1,
        };
        call_void(&self.signal.bridge, "updateTimer", "(J)V", &[JavaValue::Long(delay)]);
    }

    fn as_pending_input(&self) -> Option<&dyn IDispatcherImplWithPendingInput> {
        Some(self)
    }

    fn as_explicit_background_processing(&self) -> Option<&dyn IDispatcherImplWithExplicitBackgroundProcessing> {
        Some(self)
    }
}

impl IDispatcherImplWithExplicitBackgroundProcessing for AndroidDispatcherImpl {
    fn ready_for_background_processing(&self) -> &DispatcherImplEvent {
        &self.ready_for_background_processing
    }

    fn request_background_processing(&self) {
        self.background_processing_requested.set(true);
    }
}

impl IDispatcherImplWithPendingInput for AndroidDispatcherImpl {
    fn can_query_pending_input(&self) -> bool {
        // The message queue can be asked since API 23; the library is built for API 26.
        true
    }

    fn has_pending_input(&self) -> bool {
        !call_boolean(&self.signal.bridge, "isIdle", "()Z", &[])
    }
}
