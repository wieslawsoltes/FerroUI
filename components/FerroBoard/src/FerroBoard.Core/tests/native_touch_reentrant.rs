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
struct Work {
    id: usize,
    action: Rc<dyn Fn()>,
    owner: Weak<Scheduler>,
}
impl Work {
    fn cancel(&self) {
        if let Some(owner) = self.owner.upgrade() {
            owner.active.borrow_mut().retain(|w| w.id != self.id);
        }
    }
}
struct Scheduler {
    me: Weak<Self>,
    all: RefCell<Vec<Rc<Work>>>,
    active: RefCell<Vec<Rc<Work>>>,
    before_return: RefCell<Option<Rc<dyn Fn()>>>,
}
impl Scheduler {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|me| Self {
            me: me.clone(),
            all: RefCell::new(Vec::new()),
            active: RefCell::new(Vec::new()),
            before_return: RefCell::new(None),
        })
    }
    fn fire(&self, index: usize) {
        let work = self.all.borrow()[index].clone();
        self.active.borrow_mut().retain(|w| w.id != work.id);
        (work.action)();
    }
    fn fire_archived(&self) {
        let actions: Vec<_> = self.all.borrow().iter().map(|w| w.action.clone()).collect();
        for action in actions {
            action();
        }
    }
}
impl IKeyboardScheduler for Scheduler {
    fn schedule(&self, _delay: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        let callback = self.before_return.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        let work = Rc::new(Work {
            id: self.all.borrow().len(),
            action,
            owner: self.me.clone(),
        });
        self.all.borrow_mut().push(work.clone());
        self.active.borrow_mut().push(work.clone());
        ScheduledAction::new(move || work.cancel())
    }
}
fn setup() -> (
    Rc<KeyboardEngine>,
    Rc<TextBufferTarget>,
    Rc<Scheduler>,
    Rc<KeyboardTouchProcessor>,
) {
    let engine = KeyboardEngine::with_defaults().unwrap();
    engine.settings().set_auto_capitalization(false);
    engine.settings().set_show_suggestions(false);
    engine.settings().set_glide_typing(false);
    let target = Rc::new(TextBufferTarget::new("fixture".into(), None));
    engine.attach(target.clone()).unwrap();
    engine.update_layout(1000.0, 240.0, None).unwrap();
    let scheduler = Scheduler::new();
    let touch = KeyboardTouchProcessor::new(engine.clone(), scheduler.clone());
    (engine, target, scheduler, touch)
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
fn verify_drop(
    engine: Rc<KeyboardEngine>,
    target: Rc<TextBufferTarget>,
    scheduler: Rc<Scheduler>,
    touch: Rc<KeyboardTouchProcessor>,
) {
    assert!(touch.pressed_keys().iter().next().is_none());
    assert_eq!(
        scheduler.active.borrow().len(),
        1,
        "source post-callback scheduling remains until controller teardown"
    );
    let weak = Rc::downgrade(&touch);
    let weak_engine = Rc::downgrade(&engine);
    let before = target.text();
    drop(touch);
    assert!(weak.upgrade().is_none());
    assert!(
        scheduler.active.borrow().is_empty(),
        "retired pointer timers must also be canceled by Drop"
    );
    scheduler.fire_archived();
    assert_eq!(target.text(), before);
    drop(engine);
    assert!(weak_engine.upgrade().is_none());
    // Drop all diagnostic archives: no pointer->timer->action->pointer cycle may remain.
    let work: Vec<_> = scheduler.all.borrow().iter().map(Rc::downgrade).collect();
    scheduler.all.borrow_mut().clear();
    assert!(work.iter().all(|weak| weak.upgrade().is_none()));
}
#[test]
fn press_callback_cancellation_does_not_hide_a_later_timer_from_native_drop() {
    let (engine, target, scheduler, touch) = setup();
    let weak = Rc::downgrade(&touch);
    touch.key_pressed().add(Rc::new(move |_| {
        weak.upgrade().unwrap().cancel_all().unwrap()
    }));
    press(&engine, &touch, i32::from(b'e'));
    verify_drop(engine, target, scheduler, touch);
}
#[test]
fn repeat_callback_cancellation_does_not_hide_its_post_callback_timer() {
    let (engine, target, scheduler, touch) = setup();
    press(&engine, &touch, KeyCode::Delete);
    let weak = Rc::downgrade(&touch);
    let once = Cell::new(true);
    target.edited().add(Rc::new(move |_| {
        if once.replace(false) {
            weak.upgrade().unwrap().cancel_all().unwrap();
        }
    }));
    scheduler.fire(0);
    verify_drop(engine, target, scheduler, touch);
}
#[test]
fn synchronous_custom_scheduler_cancellation_before_handle_return_is_cleaned() {
    let (engine, target, scheduler, touch) = setup();
    let weak = Rc::downgrade(&touch);
    *scheduler.before_return.borrow_mut() = Some(Rc::new(move || {
        weak.upgrade().unwrap().cancel_all().unwrap()
    }));
    press(&engine, &touch, i32::from(b'e'));
    verify_drop(engine, target, scheduler, touch);
}

#[test]
fn reentrant_pointer_id_replacement_tracks_both_original_and_current_states() {
    let (engine, target, scheduler, touch) = setup();
    let weak = Rc::downgrade(&touch);
    let weak_engine = Rc::downgrade(&engine);
    let once = Cell::new(true);
    touch.key_pressed().add(Rc::new(move |_| {
        if once.replace(false) {
            let touch = weak.upgrade().unwrap();
            let engine = weak_engine.upgrade().unwrap();
            press(&engine, &touch, i32::from(b'a'));
        }
    }));
    press(&engine, &touch, i32::from(b'e'));
    assert_eq!(touch.pressed_keys().iter().count(), 1);
    assert_eq!(scheduler.active.borrow().len(), 2);
    let weak = Rc::downgrade(&touch);
    let before = target.text();
    drop(touch);
    assert!(weak.upgrade().is_none());
    assert!(
        scheduler.active.borrow().is_empty(),
        "same pointer ID must not hide the original retired state"
    );
    scheduler.fire_archived();
    assert_eq!(target.text(), before);
    let work: Vec<_> = scheduler.all.borrow().iter().map(Rc::downgrade).collect();
    scheduler.all.borrow_mut().clear();
    assert!(work.iter().all(|weak| weak.upgrade().is_none()));
}

struct DropWork {
    action: RefCell<Option<Rc<dyn Fn()>>>,
    canceled: Cell<bool>,
    on_drop: Option<Rc<dyn Fn()>>,
}
impl DropWork {
    fn cancel(&self) {
        self.canceled.set(true);
        let action = self.action.borrow_mut().take();
        drop(action);
    }
}
impl Drop for DropWork {
    fn drop(&mut self) {
        if let Some(callback) = self.on_drop.take() {
            callback();
        }
    }
}
#[derive(Default)]
struct DropScheduler {
    calls: Cell<usize>,
    second_handle_drop: RefCell<Option<Rc<dyn Fn()>>>,
    handles: RefCell<Vec<Weak<DropWork>>>,
}
impl IKeyboardScheduler for DropScheduler {
    fn schedule(&self, _delay: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        let index = self.calls.replace(self.calls.get() + 1);
        // Synchronous completion is supported by the scheduler contract. A completed
        // callback no longer retains its contact when the cancellation handle is retired.
        let action = if index == 1 {
            action();
            None
        } else {
            Some(action)
        };
        let work = Rc::new(DropWork {
            action: RefCell::new(action),
            canceled: Cell::new(false),
            on_drop: if index == 1 {
                self.second_handle_drop.borrow_mut().take()
            } else {
                None
            },
        });
        self.handles.borrow_mut().push(Rc::downgrade(&work));
        ScheduledAction::new(move || work.cancel())
    }
}

#[test]
fn replaced_contact_timer_drop_can_query_and_cancel_the_replacement() {
    let (engine, target, _unused_scheduler, unused_touch) = setup();
    drop(unused_touch);
    let scheduler = Rc::new(DropScheduler::default());
    let touch = KeyboardTouchProcessor::new(engine.clone(), scheduler.clone());
    let drops = Rc::new(Cell::new(0));
    let count = drops.clone();
    let weak = Rc::downgrade(&touch);
    *scheduler.second_handle_drop.borrow_mut() = Some(Rc::new(move || {
        count.set(count.get() + 1);
        let touch = weak.upgrade().unwrap();
        // The new contact is installed but its press phase has not started yet.
        assert!(touch.pressed_keys().iter().next().is_none());
        touch.cancel_all().unwrap();
    }));
    let weak = Rc::downgrade(&touch);
    let weak_engine = Rc::downgrade(&engine);
    let once = Cell::new(true);
    touch.key_released().add(Rc::new(move |_| {
        if once.replace(false) {
            // Reenter while an old same-ID contact is canceled. The outer down then
            // replaces this held Shift, retiring its scheduler handle synchronously.
            press(
                &weak_engine.upgrade().unwrap(),
                &weak.upgrade().unwrap(),
                KeyCode::Shift,
            );
        }
    }));
    press(&engine, &touch, i32::from(b'e'));
    press(&engine, &touch, i32::from(b's'));
    assert_eq!(drops.get(), 1);
    assert_eq!(scheduler.calls.get(), 3);
    assert!(touch.pressed_keys().iter().next().is_none());
    assert_eq!(
        scheduler
            .handles
            .borrow()
            .iter()
            .filter(|handle| handle.upgrade().is_some())
            .count(),
        1,
        "post-callback scheduling is retained until controller teardown"
    );
    assert_eq!(target.text().to_utf8().unwrap(), "fixture");
    let weak = Rc::downgrade(&touch);
    drop(touch);
    assert!(weak.upgrade().is_none());
    assert!(
        scheduler
            .handles
            .borrow()
            .iter()
            .all(|handle| handle.upgrade().is_none())
    );
    engine.dispose().unwrap();
}
