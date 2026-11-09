//! Component-owned timer tests. Call the real handler directly to isolate it from the framework's
//! additional routed holding/tapping recognizer, which legitimately owns pointer/press state.
//! Existing touch_button integration tests exercise normal routed/raw input separately.
use super::TouchButton;
use ferroboard_core::input::IKeyboardScheduler;
use ferroboard_core::input::ScheduledAction;
use ferroui_base::{
    Point, Ref,
    input::{KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType},
};
use std::time::Duration;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Work {
    canceled: Cell<bool>,
    action: Rc<dyn Fn()>,
}
impl Work {
    fn cancel(&self) {
        self.canceled.set(true);
    }
}
#[derive(Default)]
struct SavedScheduler {
    work: RefCell<Vec<Rc<Work>>>,
}
impl IKeyboardScheduler for SavedScheduler {
    fn schedule(&self, _: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        let work = Rc::new(Work {
            canceled: Cell::new(false),
            action,
        });
        self.work.borrow_mut().push(work.clone());
        ScheduledAction::new(move || work.cancel())
    }
}
fn press(button: &Ref<TouchButton>, pointer: &Rc<Pointer>) {
    let args = PointerPressedEventArgs::new(
        button,
        pointer.clone(),
        button,
        Point::new(5.0, 5.0),
        0,
        PointerPointProperties {
            is_left_button_pressed: true,
            ..Default::default()
        },
        KeyModifiers::empty(),
        1,
    );
    button.pressed(&args);
    assert!(args.handled());
}
#[test]
fn a_missing_pointer_up_does_not_leave_timer_work_owning_the_button() {
    crate::test_support::run(|| {
        for repeat in [false, true] {
            let scheduler = Rc::new(SavedScheduler::default());
            let button = TouchButton::new();
            button.set_scheduler(scheduler.clone());
            if repeat {
                button.set_repeat_delay(Some(Duration::from_nanos(10000)));
            } else {
                button.long_press(|_| {});
            }
            let pointer = Pointer::new(1, PointerType::Mouse, true);
            press(&button, &pointer);
            assert_eq!(scheduler.work.borrow().len(), 1);
            let weak = button.downgrade();
            drop(pointer);
            drop(button);
            assert!(
                weak.upgrade().is_none(),
                "scheduled button work must not retain its receiver"
            );
            assert!(scheduler.work.borrow()[0].canceled.get());
        }
    });
}
use ferroui_base::{input::IPointer, threading::Dispatcher};
use ferroui_controls::{Control, Window};
fn configured(repeat: bool) -> (Ref<TouchButton>, Rc<SavedScheduler>) {
    let scheduler = Rc::new(SavedScheduler::default());
    let button = TouchButton::new();
    button.set_scheduler(scheduler.clone());
    if repeat {
        button.set_repeat_delay(Some(Duration::from_nanos(10000)));
    } else {
        button.long_press(|_| {});
    }
    (button, scheduler)
}
#[test]
fn retained_canceled_callbacks_cannot_invoke_or_rearm_a_dropped_button() {
    crate::test_support::run(|| {
        for repeat in [false, true] {
            let (button, scheduler) = configured(repeat);
            let count = Rc::new(Cell::new(0));
            let clicks = count.clone();
            button.click(move |_| clicks.set(clicks.get() + 1));
            let longs = count.clone();
            button.long_press(move |_| longs.set(longs.get() + 1));
            let pointer = Pointer::new(1, PointerType::Touch, true);
            press(&button, &pointer);
            let work = scheduler.work.borrow()[0].clone();
            let weak = button.downgrade();
            drop(pointer);
            drop(button);
            assert!(weak.upgrade().is_none());
            assert!(work.canceled.get());
            (work.action)(); // Deliberately retain and replay canceled work.
            assert_eq!(count.get(), 0);
            assert_eq!(scheduler.work.borrow().len(), 1);
        }
    });
}
#[test]
fn an_executed_timer_does_not_create_a_button_handle_callback_cycle() {
    crate::test_support::run(|| {
        for repeat in [false, true] {
            let (button, scheduler) = configured(repeat);
            let count = Rc::new(Cell::new(0));
            let clicks = count.clone();
            button.click(move |_| clicks.set(clicks.get() + 1));
            let longs = count.clone();
            button.long_press(move |_| longs.set(longs.get() + 1));
            let pointer = Pointer::new(1, PointerType::Mouse, true);
            press(&button, &pointer);
            let work = scheduler.work.borrow()[0].clone();
            (work.action)();
            assert_eq!(count.get(), 1);
            let current = scheduler.work.borrow().last().unwrap().clone();
            let weak = button.downgrade();
            drop(pointer);
            drop(button);
            assert!(weak.upgrade().is_none());
            assert!(current.canceled.get());
            let pending = scheduler.work.borrow().len();
            (work.action)();
            assert_eq!(count.get(), 1);
            assert_eq!(scheduler.work.borrow().len(), pending);
        }
    });
}
#[test]
fn tree_removal_cancels_capture_and_work_for_a_retained_button() {
    crate::test_support::run(|| {
        for repeat in [false, true] {
            let (button, scheduler) = configured(repeat);
            let window = Window::new();
            window.set_content(Some(Control::boxed(button.clone())));
            window.show();
            Dispatcher::current_dispatcher().run_jobs(None);
            let count = Rc::new(Cell::new(0));
            let clicks = count.clone();
            button.click(move |_| clicks.set(clicks.get() + 1));
            let longs = count.clone();
            button.long_press(move |_| longs.set(longs.get() + 1));
            let pointer = Pointer::new(1, PointerType::Mouse, true);
            press(&button, &pointer);
            let work = scheduler.work.borrow()[0].clone();
            window.set_content(None);
            assert!(
                pointer
                    .captured()
                    .and_then(|value| value.cast::<TouchButton>())
                    .is_none()
            );
            assert!(work.canceled.get());
            (work.action)();
            assert_eq!(count.get(), 0);
            assert_eq!(scheduler.work.borrow().len(), 1);
            window.close();
        }
    });
}
#[test]
fn repeat_callback_can_remove_its_owner_and_drop_the_last_external_button_reference() {
    crate::test_support::run(|| {
        let (button, scheduler) = configured(true);
        let window = Window::new();
        window.set_content(Some(Control::boxed(button.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        let slot = Rc::new(RefCell::new(Some(button.clone())));
        let weak_slot = Rc::downgrade(&slot);
        let weak_window = window.downgrade();
        let weak = button.downgrade();
        let during = weak.clone();
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        button.click(move |_| {
            observed.set(observed.get() + 1);
            weak_window.upgrade().unwrap().set_content(None);
            weak_slot.upgrade().unwrap().borrow_mut().take();
            assert!(
                during.upgrade().is_some(),
                "in-flight repeat receiver stays alive until callback returns"
            );
        });
        let pointer = Pointer::new(1, PointerType::Mouse, true);
        press(&button, &pointer);
        drop(button);
        let work = scheduler.work.borrow()[0].clone();
        (work.action)();
        assert_eq!(calls.get(), 1);
        assert!(
            pointer
                .captured()
                .and_then(|value| value.cast::<TouchButton>())
                .is_none()
        );
        // The renderer can legitimately own a detached dirty visual until its queued frame.
        // Drain via the same published timer/job progression used by Headless window helpers.
        let dispatcher = Dispatcher::current_dispatcher();
        for _ in 0..10 {
            dispatcher.run_jobs(None);
            ferroui_headless::FerroHeadlessPlatform::force_render_timer_tick(1);
            if !dispatcher
                .has_jobs_with_priority(ferroui_base::threading::DispatcherPriority::SYSTEM_IDLE)
            {
                break;
            }
        }
        dispatcher.run_jobs(None);
        assert!(weak.upgrade().is_none());
        assert!(
            scheduler
                .work
                .borrow()
                .iter()
                .all(|work| work.canceled.get())
        );
        let pending = scheduler.work.borrow().len();
        (work.action)();
        assert_eq!(calls.get(), 1);
        assert_eq!(scheduler.work.borrow().len(), pending);
        window.close();
    });
}
#[test]
fn capture_retains_an_active_button_until_capture_loss_even_with_weak_timer_work() {
    crate::test_support::run(|| {
        let (button, scheduler) = configured(true);
        let pointer = Pointer::new(1, PointerType::Mouse, true);
        press(&button, &pointer);
        let weak = button.downgrade();
        drop(button);
        assert!(
            weak.upgrade().is_some(),
            "active native capture is an actual owner"
        );
        pointer.platform_capture_lost();
        assert!(weak.upgrade().is_none());
        assert!(scheduler.work.borrow()[0].canceled.get());
    });
}
#[test]
fn drop_stops_real_dispatcher_timer_and_late_ticks_do_not_invoke_handlers() {
    crate::test_support::run(|| {
        use crate::services::DispatcherKeyboardScheduler;
        let button = TouchButton::new();
        button.set_scheduler(Rc::new(DispatcherKeyboardScheduler::default()));
        button.set_repeat_delay(Some(Duration::from_millis(1000)));
        let count = Rc::new(Cell::new(0));
        let clicks = count.clone();
        button.click(move |_| clicks.set(clicks.get() + 1));
        let before = Dispatcher::timers_for_unit_tests();
        let pointer = Pointer::new(1, PointerType::Mouse, true);
        press(&button, &pointer);
        let timers = Dispatcher::timers_for_unit_tests()
            .into_iter()
            .filter(|timer| !before.iter().any(|old| Rc::ptr_eq(old, timer)))
            .collect::<Vec<_>>();
        assert_eq!(timers.len(), 1);
        assert!(timers[0].is_enabled());
        let weak = button.downgrade();
        drop(pointer);
        drop(button);
        assert!(weak.upgrade().is_none());
        assert!(!timers[0].is_enabled());
        Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);
        assert_eq!(count.get(), 0);
    });
}
#[test]
fn retained_reentrant_detach_preserves_source_post_callback_schedule_but_emits_no_more_clicks() {
    crate::test_support::run(|| {
        let (button, scheduler) = configured(true);
        let window = Window::new();
        window.set_content(Some(Control::boxed(button.clone())));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        let weak_window = window.downgrade();
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        button.click(move |_| {
            count.set(count.get() + 1);
            weak_window.upgrade().unwrap().set_content(None);
        });
        let pointer = Pointer::new(1, PointerType::Mouse, true);
        press(&button, &pointer);
        let first = scheduler.work.borrow()[0].clone();
        (first.action)();
        assert_eq!(calls.get(), 1);
        assert!(first.canceled.get());
        assert!(
            pointer
                .captured()
                .and_then(|value| value.cast::<TouchButton>())
                .is_none()
        );
        // Source Repeat always rereads the scheduler and schedules after Click returns,
        // even if a reentrant capture-loss/reset happened in that callback.
        assert_eq!(scheduler.work.borrow().len(), 2);
        let next = scheduler.work.borrow()[1].clone();
        (next.action)();
        assert_eq!(calls.get(), 1);
        assert_eq!(
            scheduler.work.borrow().len(),
            2,
            "reset state prevents further repetition"
        );
        window.close();
    });
}

struct RetiredWork {
    inactive: Rc<Cell<bool>>,
    on_drop: Option<Rc<dyn Fn()>>,
}
impl RetiredWork {
    fn cancel(&self) {
        self.inactive.set(true);
    }
}
impl Drop for RetiredWork {
    fn drop(&mut self) {
        if let Some(callback) = self.on_drop.take() {
            callback();
        }
    }
}
#[derive(Default)]
struct RetirementScheduler {
    actions: RefCell<Vec<Rc<dyn Fn()>>>,
    inactive: RefCell<Vec<Rc<Cell<bool>>>>,
    handles: RefCell<Vec<std::rc::Weak<RetiredWork>>>,
    first_drop: RefCell<Option<Rc<dyn Fn()>>>,
}
impl IKeyboardScheduler for RetirementScheduler {
    fn schedule(&self, _: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        let inactive = Rc::new(Cell::new(false));
        let work = Rc::new(RetiredWork {
            inactive: inactive.clone(),
            on_drop: self.first_drop.borrow_mut().take(),
        });
        // Deliberately archive inactive actions separately from their owned handles.
        let completed = inactive.clone();
        self.actions.borrow_mut().push(Rc::new(move || {
            completed.set(true);
            action();
        }));
        self.inactive.borrow_mut().push(inactive);
        self.handles.borrow_mut().push(Rc::downgrade(&work));
        ScheduledAction::new(move || work.cancel())
    }
}
fn retirement_button(repeat: bool) -> (Ref<TouchButton>, Rc<RetirementScheduler>, Rc<Cell<usize>>) {
    let button = TouchButton::new();
    let scheduler = Rc::new(RetirementScheduler::default());
    button.set_scheduler(scheduler.clone());
    let calls = Rc::new(Cell::new(0));
    let clicks = calls.clone();
    button.click(move |_| clicks.set(clicks.get() + 1));
    let longs = calls.clone();
    button.long_press(move |_| longs.set(longs.get() + 1));
    if repeat {
        button.set_repeat_delay(Some(Duration::from_nanos(10000)));
    }
    let weak = button.downgrade();
    *scheduler.first_drop.borrow_mut() = Some(Rc::new(move || {
        weak.upgrade().unwrap().reset_core();
    }));
    (button, scheduler, calls)
}
fn assert_retired(
    button: Ref<TouchButton>,
    scheduler: &RetirementScheduler,
    calls: &Cell<usize>,
    expected: usize,
) {
    assert!(
        scheduler
            .handles
            .borrow()
            .iter()
            .all(|work| work.upgrade().is_none())
    );
    assert!(scheduler.inactive.borrow().iter().all(|flag| flag.get()));
    let count = scheduler.actions.borrow().len();
    let archived = scheduler.actions.borrow().clone();
    for action in archived {
        action();
    }
    assert_eq!(
        calls.get(),
        expected,
        "retired callbacks must not invoke handlers"
    );
    assert_eq!(
        scheduler.actions.borrow().len(),
        count,
        "retired callbacks must not rearm"
    );
    let weak = button.downgrade();
    drop(button);
    assert!(weak.upgrade().is_none());
}
#[test]
fn timer_retirement_on_press_allows_reentrant_reset() {
    crate::test_support::run(|| {
        for repeat in [false, true] {
            let (button, scheduler, calls) = retirement_button(repeat);
            let pointer = Pointer::new(1, PointerType::Touch, true);
            press(&button, &pointer);
            press(&button, &pointer);
            pointer.capture(None);
            drop(pointer);
            assert_retired(button, &scheduler, &calls, 0);
        }
    });
}
#[test]
fn timer_retirement_on_repeat_allows_reentrant_reset() {
    crate::test_support::run(|| {
        let (button, scheduler, calls) = retirement_button(true);
        let pointer = Pointer::new(1, PointerType::Touch, true);
        press(&button, &pointer);
        let first = scheduler.actions.borrow()[0].clone();
        first();
        pointer.capture(None);
        drop(pointer);
        assert_retired(button, &scheduler, &calls, 1);
    });
}
#[test]
fn timer_retirement_on_cancel_allows_reentrant_reset() {
    crate::test_support::run(|| {
        for repeat in [false, true] {
            let (button, scheduler, calls) = retirement_button(repeat);
            let pointer = Pointer::new(1, PointerType::Touch, true);
            press(&button, &pointer);
            pointer.capture(None);
            drop(pointer);
            assert_retired(button, &scheduler, &calls, 0);
        }
    });
}
struct RetiredScheduler(Option<Rc<dyn Fn()>>);
impl IKeyboardScheduler for RetiredScheduler {
    fn schedule(&self, _: Duration, _: Rc<dyn Fn()>) -> ScheduledAction {
        panic!("the replaced scheduler must never receive work");
    }
}
impl Drop for RetiredScheduler {
    fn drop(&mut self) {
        if let Some(callback) = self.0.take() {
            callback();
        }
    }
}
#[test]
fn scheduler_retirement_allows_reentrant_scheduler_replacement() {
    crate::test_support::run(|| {
        let button = TouchButton::new();
        button.set_repeat_delay(Some(Duration::from_nanos(10000)));
        let replacement = Rc::new(SavedScheduler::default());
        let chosen = Rc::new(SavedScheduler::default());
        let desired = chosen.clone();
        let weak = button.downgrade();
        button.set_scheduler(Rc::new(RetiredScheduler(Some(Rc::new(move || {
            weak.upgrade().unwrap().set_scheduler(desired.clone());
        })))));
        button.set_scheduler(replacement.clone());
        let pointer = Pointer::new(1, PointerType::Touch, true);
        press(&button, &pointer);
        assert!(replacement.work.borrow().is_empty());
        assert_eq!(chosen.work.borrow().len(), 1);
        pointer.capture(None);
        drop(pointer);
        let weak = button.downgrade();
        drop(button);
        assert!(weak.upgrade().is_none());
        assert!(chosen.work.borrow()[0].canceled.get());
    });
}
