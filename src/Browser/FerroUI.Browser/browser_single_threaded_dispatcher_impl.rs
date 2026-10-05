use ferroui_base::threading::{
    DispatcherImplEvent, IDispatcherImpl, IDispatcherImplWithExplicitBackgroundProcessing, IDispatcherSignal,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::thread::{self, ThreadId};
use std::time::Instant;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    #[wasm_bindgen(js_namespace = SingleThreadedDispatcherHelper, js_name = signal)]
    fn js_signal();

    #[wasm_bindgen(js_namespace = SingleThreadedDispatcherHelper, js_name = requestBackgroundProcessing)]
    fn js_request_background_processing();

    #[wasm_bindgen(js_namespace = SingleThreadedDispatcherHelper, js_name = setTimer)]
    fn js_set_timer(delay_ms: i32);

    #[wasm_bindgen(js_namespace = SingleThreadedDispatcherHelper, js_name = clearTimer)]
    fn js_clear_timer();
}

/// The calls into the page the dispatcher makes; replaced by a recorder in
/// the tests.
trait IScheduling {
    fn signal(&self);
    fn request_background_processing(&self);
    fn set_timer(&self, delay_ms: i32);
    fn clear_timer(&self);
}

struct PageScheduling;

impl IScheduling for PageScheduling {
    fn signal(&self) {
        js_signal();
    }

    fn request_background_processing(&self) {
        js_request_background_processing();
    }

    fn set_timer(&self, delay_ms: i32) {
        js_set_timer(delay_ms);
    }

    fn clear_timer(&self) {
        js_clear_timer();
    }
}

thread_local! {
    static INSTANCE: RefCell<Option<Rc<BrowserSingleThreadedDispatcherImpl>>> = const { RefCell::new(None) };
}

fn instance() -> Option<Rc<BrowserSingleThreadedDispatcherImpl>> {
    INSTANCE.with(|instance| instance.borrow().clone())
}

/// Dispatcher backend for single-threaded WASM. The browser event loop is
/// the only loop: wake-ups are posted as macrotasks and everything runs on
/// the main thread, so no locking is needed. Pending input is deliberately
/// not queried: the browser dispatches input between our tasks on its own,
/// and gating low-priority jobs on a pending-input query starves them for as
/// long as the pointer keeps moving.
pub struct BrowserSingleThreadedDispatcherImpl {
    thread: ThreadId,
    clock: Instant,
    signaled_flag: Cell<bool>,
    background_requested: Cell<bool>,
    timer_set: Cell<bool>,
    signaled: DispatcherImplEvent,
    timer: DispatcherImplEvent,
    ready_for_background_processing: DispatcherImplEvent,
    scheduling: Box<dyn IScheduling>,
}

/// Wakes the dispatcher up. There is one thread, so the handle forwards to
/// the instance of that thread.
struct Signal;

impl IDispatcherSignal for Signal {
    fn signal(&self) {
        if let Some(instance) = instance() {
            IDispatcherImpl::signal(&*instance);
        }
    }
}

impl BrowserSingleThreadedDispatcherImpl {
    /// Creates the dispatcher backend of the current thread and makes it
    /// the receiver of the callbacks of the page.
    pub fn new() -> Rc<Self> {
        Self::with_scheduling(Box::new(PageScheduling))
    }

    fn with_scheduling(scheduling: Box<dyn IScheduling>) -> Rc<Self> {
        let this = Rc::new(Self {
            thread: thread::current().id(),
            clock: Instant::now(),
            signaled_flag: Cell::new(false),
            background_requested: Cell::new(false),
            timer_set: Cell::new(false),
            signaled: DispatcherImplEvent::new(),
            timer: DispatcherImplEvent::new(),
            ready_for_background_processing: DispatcherImplEvent::new(),
            scheduling,
        });
        INSTANCE.with(|instance| *instance.borrow_mut() = Some(this.clone()));
        this
    }
}

impl IDispatcherImpl for BrowserSingleThreadedDispatcherImpl {
    fn current_thread_is_loop_thread(&self) -> bool {
        thread::current().id() == self.thread
    }

    fn signal(&self) {
        if self.signaled_flag.get() {
            return;
        }
        self.signaled_flag.set(true);
        self.scheduling.signal();
    }

    fn signal_handle(&self) -> Arc<dyn IDispatcherSignal> {
        Arc::new(Signal)
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
        if self.timer_set.get() {
            self.scheduling.clear_timer();
            self.timer_set.set(false);
        }

        if let Some(due_time) = due_time_in_ms {
            self.scheduling.set_timer((due_time - self.now()).clamp(0, i32::MAX as i64) as i32);
            self.timer_set.set(true);
        }
    }

    fn as_explicit_background_processing(&self) -> Option<&dyn IDispatcherImplWithExplicitBackgroundProcessing> {
        Some(self)
    }
}

impl IDispatcherImplWithExplicitBackgroundProcessing for BrowserSingleThreadedDispatcherImpl {
    fn ready_for_background_processing(&self) -> &DispatcherImplEvent {
        &self.ready_for_background_processing
    }

    fn request_background_processing(&self) {
        if self.background_requested.get() {
            return;
        }
        self.background_requested.set(true);
        self.scheduling.request_background_processing();
    }
}

/// The task posted by a signal runs.
#[wasm_bindgen(js_name = BrowserSingleThreadedDispatcherImpl_OnSignaled)]
pub fn on_signaled() {
    let Some(impl_) = instance() else { return };
    impl_.signaled_flag.set(false);
    impl_.signaled.raise();
}

/// The task posted by a background processing request runs.
#[wasm_bindgen(js_name = BrowserSingleThreadedDispatcherImpl_OnReadyForBackgroundProcessing)]
pub fn on_ready_for_background_processing() {
    let Some(impl_) = instance() else { return };
    impl_.background_requested.set(false);
    impl_.ready_for_background_processing.raise();
}

/// The timer of the dispatcher is due.
#[wasm_bindgen(js_name = BrowserSingleThreadedDispatcherImpl_OnTimer)]
pub fn on_timer() {
    let Some(impl_) = instance() else { return };
    impl_.timer_set.set(false);
    impl_.timer.raise();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder {
        calls: Rc<RefCell<Vec<String>>>,
    }

    impl IScheduling for Recorder {
        fn signal(&self) {
            self.calls.borrow_mut().push("signal".to_string());
        }

        fn request_background_processing(&self) {
            self.calls.borrow_mut().push("background".to_string());
        }

        fn set_timer(&self, delay_ms: i32) {
            self.calls.borrow_mut().push(format!("setTimer({})", delay_ms.min(1_000_000) / 1000));
        }

        fn clear_timer(&self) {
            self.calls.borrow_mut().push("clearTimer".to_string());
        }
    }

    fn create() -> (Rc<BrowserSingleThreadedDispatcherImpl>, Rc<RefCell<Vec<String>>>) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let impl_ = BrowserSingleThreadedDispatcherImpl::with_scheduling(Box::new(Recorder { calls: calls.clone() }));
        (impl_, calls)
    }

    fn counter(event: &DispatcherImplEvent) -> Rc<Cell<u32>> {
        let count = Rc::new(Cell::new(0));
        let seen = count.clone();
        event.add(Rc::new(move |_| seen.set(seen.get() + 1)));
        count
    }

    #[test]
    fn the_creating_thread_is_the_loop_thread() {
        let (impl_, _) = create();

        assert!(impl_.current_thread_is_loop_thread());
    }

    #[test]
    fn signals_are_coalesced_until_the_posted_task_runs() {
        let (impl_, calls) = create();
        let signaled = counter(impl_.signaled());

        IDispatcherImpl::signal(&*impl_);
        IDispatcherImpl::signal(&*impl_);
        assert_eq!(vec!["signal"], *calls.borrow());
        assert_eq!(0, signaled.get());

        on_signaled();
        assert_eq!(1, signaled.get());

        IDispatcherImpl::signal(&*impl_);
        assert_eq!(vec!["signal", "signal"], *calls.borrow());
    }

    #[test]
    fn the_signal_handle_wakes_the_instance_of_the_thread() {
        let (impl_, calls) = create();

        impl_.signal_handle().signal();
        impl_.signal_handle().signal();

        assert_eq!(vec!["signal"], *calls.borrow());
    }

    #[test]
    fn background_requests_are_coalesced_until_the_posted_task_runs() {
        let (impl_, calls) = create();
        let ready = counter(impl_.ready_for_background_processing());

        impl_.request_background_processing();
        impl_.request_background_processing();
        assert_eq!(vec!["background"], *calls.borrow());

        on_ready_for_background_processing();
        assert_eq!(1, ready.get());

        impl_.request_background_processing();
        assert_eq!(vec!["background", "background"], *calls.borrow());
    }

    #[test]
    fn the_timer_is_set_relative_to_now_and_replaced_by_the_next_update() {
        let (impl_, calls) = create();

        impl_.update_timer(Some(impl_.now() + 5_500));
        impl_.update_timer(Some(impl_.now() + 2_500));
        impl_.update_timer(None);
        impl_.update_timer(None);

        assert_eq!(vec!["setTimer(5)", "clearTimer", "setTimer(2)", "clearTimer"], *calls.borrow());
    }

    #[test]
    fn a_due_time_in_the_past_fires_immediately() {
        let (impl_, calls) = create();

        impl_.update_timer(Some(-1_000));

        assert_eq!(vec!["setTimer(0)"], *calls.borrow());
    }

    #[test]
    fn a_fired_timer_is_not_cleared_by_the_next_update() {
        let (impl_, calls) = create();
        let timer = counter(impl_.timer());

        impl_.update_timer(Some(impl_.now() + 1_500));
        on_timer();
        assert_eq!(1, timer.get());
        impl_.update_timer(Some(impl_.now() + 3_500));

        assert_eq!(vec!["setTimer(1)", "setTimer(3)"], *calls.borrow());
    }

    #[test]
    fn explicit_background_processing_is_supported_and_a_loop_is_not() {
        let (impl_, _) = create();

        assert!(impl_.as_explicit_background_processing().is_some());
        assert!(impl_.as_pending_input().is_none());
        assert!(impl_.as_controlled().is_none());
    }
}
