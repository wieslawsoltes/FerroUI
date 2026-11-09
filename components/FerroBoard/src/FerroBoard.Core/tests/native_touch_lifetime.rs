use ferroboard_core::input::ScheduledAction;
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::KeyboardEngine,
    input::{IKeyboardScheduler, KeyboardTouchProcessor},
    keys::KeyCode,
};
use std::time::Duration;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
struct Job {
    id: usize,
    action: Rc<dyn Fn()>,
    owner: Weak<Archive>,
}
impl Job {
    fn cancel(&self) {
        if let Some(owner) = self.owner.upgrade() {
            owner.cancelled.set(owner.cancelled.get() + 1);
            owner.active.borrow_mut().retain(|job| job.id != self.id);
        }
    }
}
struct Archive {
    me: Weak<Self>,
    active: RefCell<Vec<Rc<Job>>>,
    all: RefCell<Vec<Rc<Job>>>,
    cancelled: Cell<usize>,
}
impl Archive {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|me| Self {
            me: me.clone(),
            active: RefCell::new(Vec::new()),
            all: RefCell::new(Vec::new()),
            cancelled: Cell::new(0),
        })
    }
    fn fire(&self, index: usize) {
        let job = self.all.borrow()[index].clone();
        self.active.borrow_mut().retain(|entry| entry.id != job.id);
        (job.action)();
    }
    fn fire_archived(&self) {
        let actions: Vec<_> = self
            .all
            .borrow()
            .iter()
            .map(|job| job.action.clone())
            .collect();
        for action in actions {
            action();
        }
    }
}
impl IKeyboardScheduler for Archive {
    fn schedule(&self, _delay: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        let job = Rc::new(Job {
            id: self.all.borrow().len(),
            action,
            owner: self.me.clone(),
        });
        self.active.borrow_mut().push(job.clone());
        self.all.borrow_mut().push(job.clone());
        ScheduledAction::new(move || job.cancel())
    }
}
fn setup() -> (
    Rc<KeyboardEngine>,
    Rc<TextBufferTarget>,
    Rc<Archive>,
    Rc<KeyboardTouchProcessor>,
) {
    let engine = KeyboardEngine::with_defaults().unwrap();
    engine.settings().set_auto_capitalization(false);
    engine.settings().set_show_suggestions(false);
    engine.settings().set_glide_typing(false);
    let target = Rc::new(TextBufferTarget::new("fixture".into(), None));
    engine.attach(target.clone()).unwrap();
    engine.update_layout(1000.0, 240.0, None).unwrap();
    let clock = Archive::new();
    let touch = KeyboardTouchProcessor::new(engine.clone(), clock.clone());
    (engine, target, clock, touch)
}
fn press(engine: &KeyboardEngine, touch: &KeyboardTouchProcessor, code: i32) {
    let bounds = engine
        .keyboard()
        .unwrap()
        .find_key(code)
        .unwrap()
        .unwrap()
        .touch_bounds();
    touch
        .pointer_down(1, bounds.center_x(), bounds.center_y())
        .unwrap();
}
#[test]
fn pending_callbacks_and_retained_pressed_queries_do_not_own_the_controller_or_engine() {
    for code in [i32::from(b'e'), KeyCode::Delete, KeyCode::Shift] {
        let (engine, target, clock, touch) = setup();
        press(&engine, &touch, code);
        assert!(!clock.active.borrow().is_empty());
        let pressed = touch.pressed_keys();
        let weak = Rc::downgrade(&touch);
        let weak_engine = Rc::downgrade(&engine);
        let released = Rc::new(Cell::new(0));
        let log = released.clone();
        touch
            .key_released()
            .add(Rc::new(move |_| log.set(log.get() + 1)));
        let before = target.text();
        drop(touch);
        assert!(
            weak.upgrade().is_none(),
            "scheduler callback must not retain the controller"
        );
        assert!(clock.active.borrow().is_empty());
        assert!(clock.cancelled.get() > 0);
        assert_eq!(released.get(), 0, "Drop does not emit gesture/input events");
        clock.fire_archived();
        assert_eq!(target.text(), before);
        assert_eq!(released.get(), 0);
        drop(engine);
        assert!(
            weak_engine.upgrade().is_none(),
            "archived callbacks/queries must not retain the engine"
        );
        drop(pressed);
    }
}
#[test]
fn last_external_owner_released_during_a_timer_callback_is_cleaned_after_the_callback() {
    let (engine, target, clock, touch) = setup();
    press(&engine, &touch, KeyCode::Shift);
    let weak = Rc::downgrade(&touch);
    let slot = Rc::new(RefCell::new(Some(touch)));
    let owner = Rc::downgrade(&slot);
    engine.state_changed().add(Rc::new(move |_| {
        if let Some(owner) = owner.upgrade() {
            owner.borrow_mut().take();
        }
    }));
    clock.fire(0);
    assert!(slot.borrow().is_none());
    assert!(weak.upgrade().is_none());
    assert!(clock.active.borrow().is_empty());
    let before = target.text();
    clock.fire_archived();
    assert_eq!(target.text(), before);
}
#[test]
fn explicit_cancel_stays_eventful_and_drop_does_not_repeat_it() {
    let (engine, _target, clock, touch) = setup();
    press(&engine, &touch, i32::from(b'e'));
    let released = Rc::new(Cell::new(0));
    let count = released.clone();
    touch
        .key_released()
        .add(Rc::new(move |_| count.set(count.get() + 1)));
    touch.cancel_all().unwrap();
    assert_eq!(released.get(), 1);
    let cancelled = clock.cancelled.get();
    assert!(clock.active.borrow().is_empty());
    let weak = Rc::downgrade(&touch);
    drop(touch);
    assert!(weak.upgrade().is_none());
    assert_eq!(clock.cancelled.get(), cancelled);
    assert_eq!(released.get(), 1);
    clock.fire_archived();
    assert_eq!(released.get(), 1);
}

#[test]
fn cancellation_panics_propagate_without_a_second_disposal_during_unwind() {
    struct FailingScheduler {
        calls: Rc<Cell<usize>>,
        actions: RefCell<Vec<Rc<dyn Fn()>>>,
    }
    struct FailingToken(Rc<Cell<usize>>);
    impl FailingToken {
        fn cancel(&self) {
            self.0.set(self.0.get() + 1);
            std::panic::panic_any("custom cancellation fixture");
        }
    }
    impl IKeyboardScheduler for FailingScheduler {
        fn schedule(&self, _delay: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
            self.actions.borrow_mut().push(action);
            let token = FailingToken(self.calls.clone());
            ScheduledAction::new(move || token.cancel())
        }
    }
    let (engine, target, _clock, empty) = setup();
    drop(empty);
    let calls = Rc::new(Cell::new(0));
    let scheduler = Rc::new(FailingScheduler {
        calls: calls.clone(),
        actions: RefCell::new(Vec::new()),
    });
    let weak = RefCell::new(None);
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let touch = KeyboardTouchProcessor::new(engine.clone(), scheduler.clone());
        *weak.borrow_mut() = Some(Rc::downgrade(&touch));
        press(&engine, &touch, i32::from(b'e'));
        let bounds = engine
            .keyboard()
            .unwrap()
            .find_key(i32::from(b'a'))
            .unwrap()
            .unwrap()
            .touch_bounds();
        // Rollover cancels the first contact while it is still in the active map.
        touch
            .pointer_down(2, bounds.center_x(), bounds.center_y())
            .unwrap();
    }))
    .unwrap_err();
    assert_eq!(
        error.downcast_ref::<&str>(),
        Some(&"custom cancellation fixture")
    );
    assert_eq!(calls.get(), 1);
    assert!(weak.borrow().as_ref().unwrap().upgrade().is_none());
    let before = target.text();
    for action in scheduler.actions.borrow().iter() {
        action();
    }
    assert_eq!(target.text(), before);
}
