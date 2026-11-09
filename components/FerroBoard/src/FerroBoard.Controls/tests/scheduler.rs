use ferroboard_controls::services::DispatcherKeyboardScheduler;
use ferroboard_core::input::IKeyboardScheduler;
use ferroui_base::threading::{
    Dispatcher, DispatcherImplEvent, IDispatcherImpl, IDispatcherSignal,
};
use std::time::Duration;
use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
struct Signal(Arc<AtomicBool>);
impl IDispatcherSignal for Signal {
    fn signal(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
}
struct Platform {
    now: Cell<i64>,
    due: Cell<Option<i64>>,
    signal: Arc<AtomicBool>,
    signaled: DispatcherImplEvent,
    timer: DispatcherImplEvent,
}
impl Platform {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            now: Cell::new(0),
            due: Cell::new(None),
            signal: Arc::new(AtomicBool::new(false)),
            signaled: DispatcherImplEvent::new(),
            timer: DispatcherImplEvent::new(),
        })
    }
    fn advance_to(&self, now: i64) {
        self.now.set(now);
        if self.due.get().is_some_and(|due| due <= now) {
            self.timer.raise();
        }
        Dispatcher::current_dispatcher().run_jobs(None);
    }
}
impl IDispatcherImpl for Platform {
    fn current_thread_is_loop_thread(&self) -> bool {
        true
    }
    fn signal(&self) {
        self.signal.store(true, Ordering::Relaxed);
    }
    fn signal_handle(&self) -> Arc<dyn IDispatcherSignal> {
        Arc::new(Signal(self.signal.clone()))
    }
    fn signaled(&self) -> &DispatcherImplEvent {
        &self.signaled
    }
    fn timer(&self) -> &DispatcherImplEvent {
        &self.timer
    }
    fn now(&self) -> i64 {
        self.now.get()
    }
    fn update_timer(&self, due: Option<i64>) {
        self.due.set(due);
    }
}
#[test]
fn dispatcher_scheduler_is_one_shot_and_final_handle_drop_cancels_work() {
    let _scope = Dispatcher::unit_test_scope();
    let platform = Platform::new();
    Dispatcher::initialize_ui_thread_dispatcher(platform.clone());
    let scheduler = DispatcherKeyboardScheduler::default();
    let fired = Rc::new(Cell::new(0));
    let count = fired.clone();
    let handle = scheduler.schedule(
        Duration::from_millis(300),
        Rc::new(move || count.set(count.get() + 1)),
    );
    platform.advance_to(299);
    assert_eq!(fired.get(), 0);
    platform.advance_to(300);
    assert_eq!(fired.get(), 1);
    assert!(platform.due.get().is_none());
    platform.advance_to(900);
    assert_eq!(fired.get(), 1);
    handle.cancel();
    handle.cancel();
    let count = fired.clone();
    let handle = scheduler.schedule(
        Duration::from_millis(1),
        Rc::new(move || count.set(count.get() + 1)),
    );
    handle.cancel();
    platform.advance_to(901);
    assert_eq!(fired.get(), 1);
    let count = fired.clone();
    drop(scheduler.schedule(
        Duration::from_millis(1),
        Rc::new(move || count.set(count.get() + 1)),
    ));
    platform.advance_to(902);
    assert_eq!(fired.get(), 1);
    assert!(platform.due.get().is_none());
    let count = fired.clone();
    let handle = scheduler.schedule(
        Duration::from_millis(1),
        Rc::new(move || count.set(count.get() + 1)),
    );
    let retained = handle.clone();
    drop(handle);
    platform.advance_to(903);
    assert_eq!(fired.get(), 2);
    drop(retained);
}
#[test]
fn dispatcher_scheduler_clamps_zero_delay_and_uses_its_own_dispatcher() {
    let _scope = Dispatcher::unit_test_scope();
    let platform = Platform::new();
    Dispatcher::initialize_ui_thread_dispatcher(platform.clone());
    let scheduler = DispatcherKeyboardScheduler::new(Some(Dispatcher::current_dispatcher()));
    let fired = Rc::new(Cell::new(0));
    let mut handles = Vec::new();
    for _ in 0..3 {
        let count = fired.clone();
        handles
            .push(scheduler.schedule(Duration::ZERO, Rc::new(move || count.set(count.get() + 1))));
    }
    platform.advance_to(0);
    assert_eq!(fired.get(), 0);
    platform.advance_to(1);
    assert_eq!(fired.get(), 3);
    assert!(platform.due.get().is_none());
    let handle = scheduler.schedule(
        Duration::new(922_337_203_685, 477_580_700),
        Rc::new(|| panic!("not due")),
    );
    assert!(platform.due.get().unwrap() > 1);
    handle.cancel();
}

#[test]
fn callback_captures_are_released_on_completion_and_cancellation_with_handles_retained() {
    let _scope = Dispatcher::unit_test_scope();
    let platform = Platform::new();
    Dispatcher::initialize_ui_thread_dispatcher(platform.clone());
    let scheduler = DispatcherKeyboardScheduler::default();
    let marker = Rc::new(());
    let weak = Rc::downgrade(&marker);
    let handle = scheduler.schedule(
        Duration::from_millis(1),
        Rc::new(move || {
            std::hint::black_box(&marker);
        }),
    );
    platform.advance_to(1);
    assert!(
        weak.upgrade().is_none(),
        "completed one-shot must release its payload while handle is retained"
    );
    handle.cancel();
    let marker = Rc::new(());
    let weak = Rc::downgrade(&marker);
    let handle = scheduler.schedule(
        Duration::new(922_337_203_685, 477_580_700),
        Rc::new(move || {
            std::hint::black_box(&marker);
        }),
    );
    handle.cancel();
    assert!(
        weak.upgrade().is_none(),
        "cancelled payload must not wait for an aborted dispatcher operation to be collected"
    );
    // The dispatcher can retain a separate background-queue wakeup after timer abort.
    platform.advance_to(2);
    assert!(platform.due.get().is_none());
    handle.cancel();
}

#[test]
fn cancellation_inside_callback_and_reentrant_capture_drop_do_not_hold_state_borrows() {
    let _scope = Dispatcher::unit_test_scope();
    let platform = Platform::new();
    Dispatcher::initialize_ui_thread_dispatcher(platform.clone());
    let scheduler = DispatcherKeyboardScheduler::default();
    type HandleSlot = std::cell::RefCell<Option<ferroboard_core::input::ScheduledAction>>;
    let slot = Rc::new(HandleSlot::new(None));
    let weak = Rc::downgrade(&slot);
    let handle = scheduler.schedule(
        Duration::from_millis(1),
        Rc::new(move || {
            let slot = weak.upgrade().unwrap();
            slot.borrow().as_ref().unwrap().cancel();
        }),
    );
    *slot.borrow_mut() = Some(handle.clone());
    platform.advance_to(1);
    assert!(platform.due.get().is_none());
    slot.borrow_mut().take();
    drop(handle);
    struct CaptureDrop(std::rc::Weak<HandleSlot>);
    impl Drop for CaptureDrop {
        fn drop(&mut self) {
            if let Some(slot) = self.0.upgrade() {
                let handle = slot.borrow().clone();
                if let Some(handle) = handle {
                    handle.cancel();
                }
            }
        }
    }
    let marker = CaptureDrop(Rc::downgrade(&slot));
    let handle = scheduler.schedule(
        Duration::new(922_337_203_685, 477_580_700),
        Rc::new(move || {
            std::hint::black_box(&marker);
        }),
    );
    *slot.borrow_mut() = Some(handle.clone());
    handle.cancel();
    // The dispatcher can retain a separate background-queue wakeup after timer abort.
    platform.advance_to(2);
    assert!(platform.due.get().is_none());
    slot.borrow_mut().take();
    drop(handle);
}

#[test]
fn reentrant_contact_cancelled_before_timer_install_is_stopped_on_controller_drop() {
    use ferroboard_core::{
        editor::{ITextInputTarget, TextBufferTarget},
        engine::KeyboardEngine,
        input::KeyboardTouchProcessor,
    };
    let _scope = Dispatcher::unit_test_scope();
    let platform = Platform::new();
    Dispatcher::initialize_ui_thread_dispatcher(platform.clone());
    let engine = KeyboardEngine::with_defaults().unwrap();
    engine.settings().set_glide_typing(false);
    engine.settings().set_show_suggestions(false);
    engine.settings().set_auto_capitalization(false);
    let target = Rc::new(TextBufferTarget::default());
    engine.attach(target.clone()).unwrap();
    engine.update_layout(1000.0, 240.0, None).unwrap();
    let scheduler = Rc::new(DispatcherKeyboardScheduler::default());
    let touch = KeyboardTouchProcessor::new(engine.clone(), scheduler);
    let weak = Rc::downgrade(&touch);
    touch.key_pressed().add(Rc::new(move |_| {
        weak.upgrade().unwrap().cancel_all().unwrap()
    }));
    let before = Dispatcher::timers_for_unit_tests();
    let bounds = engine
        .keyboard()
        .unwrap()
        .find_key(i32::from(b'e'))
        .unwrap()
        .unwrap()
        .touch_bounds();
    touch
        .pointer_down(1, bounds.center_x(), bounds.center_y())
        .unwrap();
    let timers: Vec<_> = Dispatcher::timers_for_unit_tests()
        .into_iter()
        .filter(|timer| !before.iter().any(|old| Rc::ptr_eq(old, timer)))
        .collect();
    assert_eq!(timers.len(), 1);
    assert!(timers[0].is_enabled());
    assert!(touch.pressed_keys().iter().next().is_none());
    let text = target.text();
    drop(touch);
    assert!(
        !timers[0].is_enabled(),
        "retired contact still owns a live native timer"
    );
    Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);
    platform.advance_to(1000);
    assert_eq!(target.text(), text);
}
