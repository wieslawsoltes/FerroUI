//! Corresponding assertions from all nine source TouchProcessorTests.cs tests.
#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
#[path = "support/scheduler.rs"]
mod scheduler;
use ferroboard_core::{editor::*, engine::*, input::*, keys::*, layouts::*, settings::*, text::*};
use scheduler::ManualScheduler;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Fixture {
    e: Rc<KeyboardEngine>,
    t: Rc<TextBufferTarget>,
    touch: Rc<KeyboardTouchProcessor>,
    scheduler: Rc<ManualScheduler>,
}
impl Fixture {
    fn new(text: &str, configure: impl FnOnce(&KeyboardSettings)) -> Self {
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_auto_capitalization(false);
        configure(&settings);
        let e = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        let t = Rc::new(TextBufferTarget::new(text.into(), None));
        e.attach(t.clone()).unwrap();
        e.update_layout(1000.0, 240.0, None).unwrap();
        let scheduler = ManualScheduler::new();
        let clock = scheduler.clone();
        e.set_clock(Rc::new(move || {
            fixture_clock::time(639028656000000000 + clock.now())
        }));
        let touch = KeyboardTouchProcessor::new(e.clone(), scheduler.clone());
        Self {
            e,
            t,
            touch,
            scheduler,
        }
    }
    fn key(&self, label: &str) -> Rc<ComputedKey> {
        self.e
            .keyboard()
            .unwrap()
            .visible_keys()
            .map(|key| key.unwrap().unwrap())
            .find(|k| k.label() == Some(label.into()))
            .unwrap()
    }
    fn tap(&self, key: Rc<ComputedKey>) {
        let b = key.touch_bounds();
        self.touch
            .pointer_down(1, b.center_x(), b.center_y())
            .unwrap();
        self.scheduler.advance(500000);
        self.touch
            .pointer_up(1, b.center_x(), b.center_y())
            .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.touch.cancel_all().unwrap();
        self.e.dispose().unwrap();
    }
}
#[test]
fn tap_commits_character() {
    let f = Fixture::new("", |_| {});
    let pressed = Rc::new(RefCell::new(Vec::new()));
    let marks = pressed.clone();
    f.touch
        .key_pressed()
        .add(Rc::new(move |key| marks.borrow_mut().push(key.clone())));
    f.tap(f.key("h"));
    f.tap(f.key("i"));
    assert_eq!(f.t.text(), Utf16Text::from("hi"));
    assert_eq!(pressed.borrow().len(), 2);
}
#[test]
fn rollover_commits_first_key_when_second_finger_goes_down() {
    let f = Fixture::new("", |_| {});
    let a = f.key("a").touch_bounds();
    let s = f.key("s").touch_bounds();
    f.touch.pointer_down(1, a.center_x(), a.center_y()).unwrap();
    f.touch.pointer_down(2, s.center_x(), s.center_y()).unwrap();
    assert_eq!(f.t.text(), Utf16Text::from("a"));
    f.touch.pointer_up(1, a.center_x(), a.center_y()).unwrap();
    f.touch.pointer_up(2, s.center_x(), s.center_y()).unwrap();
    assert_eq!(f.t.text(), Utf16Text::from("as"));
}
#[test]
fn long_press_opens_popup_and_commits_selection() {
    let f = Fixture::new("", |_| {});
    let e = f.key("e").touch_bounds();
    f.touch.pointer_down(1, e.center_x(), e.center_y()).unwrap();
    f.scheduler
        .advance((f.e.settings().long_press_delay() as i64 + 10) * 10000);
    let popup = f.touch.popup().unwrap();
    let index = usize::from(popup.keys().len() > 1);
    let bounds = popup.bounds().borrow()[index];
    f.touch
        .pointer_move(1, bounds.center_x(), bounds.center_y())
        .unwrap();
    assert_eq!(f.touch.popup().unwrap().selected_index(), index as i32);
    f.touch
        .pointer_up(1, bounds.center_x(), bounds.center_y())
        .unwrap();
    assert!(f.touch.popup().is_none());
    assert_eq!(
        f.t.text(),
        popup
            .keys()
            .get(index)
            .unwrap()
            .as_string(false)
            .unwrap()
            .unwrap()
    );
}
#[test]
fn delete_repeats_while_held() {
    let f = Fixture::new("abcdefghij", |_| {});
    let del =
        f.e.keyboard()
            .unwrap()
            .find_key(KeyCode::Delete)
            .unwrap()
            .unwrap()
            .touch_bounds();
    f.touch
        .pointer_down(1, del.center_x(), del.center_y())
        .unwrap();
    f.scheduler.advance(
        (f.e.settings().key_repeat_delay() as i64
            + f.e.settings().key_repeat_interval() as i64 * 3
            + 5)
            * 10000,
    );
    f.touch
        .pointer_up(1, del.center_x(), del.center_y())
        .unwrap();
    assert_eq!(f.t.text(), Utf16Text::from("abcdef"));
}
#[test]
fn space_drag_moves_cursor() {
    let f = Fixture::new("hello world", |_| {});
    let space =
        f.e.keyboard()
            .unwrap()
            .find_key(KeyCode::Space)
            .unwrap()
            .unwrap()
            .touch_bounds();
    let (x, y) = (space.center_x(), space.center_y());
    f.touch.pointer_down(1, x, y).unwrap();
    for i in 1..=10 {
        f.touch.pointer_move(1, x - i as f64 * 7.0, y).unwrap();
    }
    f.touch.pointer_up(1, x - 70.0, y).unwrap();
    assert_eq!(f.t.text(), Utf16Text::from("hello world"));
    assert_eq!(f.t.selection_start(), 6);
}
#[test]
fn glide_over_keys_types_word() {
    let f = Fixture::new("", |_| {});
    let keys = "hello"
        .chars()
        .map(|c| f.key(&c.to_string()))
        .collect::<Vec<_>>();
    let b = keys[0].visible_bounds();
    f.touch.pointer_down(1, b.center_x(), b.center_y()).unwrap();
    for i in 1..keys.len() {
        let from = keys[i - 1].visible_bounds();
        let to = keys[i].visible_bounds();
        let mut t = 0.1;
        while t <= 1.0001 {
            f.touch
                .pointer_move(
                    1,
                    from.center_x() + (to.center_x() - from.center_x()) * t,
                    from.center_y() + (to.center_y() - from.center_y()) * t,
                )
                .unwrap();
            t += 0.1;
        }
    }
    assert!(f.e.is_gliding());
    assert!(!f.touch.glide_trail().borrow().is_empty());
    let b = keys.last().unwrap().visible_bounds();
    f.touch.pointer_up(1, b.center_x(), b.center_y()).unwrap();
    assert_eq!(f.t.text(), Utf16Text::from("hello"));
    assert!(!f.e.is_gliding());
}
#[test]
fn swipe_down_hides_keyboard_when_glide_disabled() {
    let f = Fixture::new("", |s| s.set_glide_typing(false));
    let hidden = Rc::new(Cell::new(false));
    let mark = hidden.clone();
    f.e.hide_requested().add(Rc::new(move |_| mark.set(true)));
    let g = f.key("g").touch_bounds();
    f.touch
        .pointer_down(1, g.center_x(), g.top() + 2.0)
        .unwrap();
    f.touch
        .pointer_move(1, g.center_x(), g.top() + 40.0)
        .unwrap();
    f.touch
        .pointer_move(1, g.center_x(), g.top() + 90.0)
        .unwrap();
    f.touch.pointer_up(1, g.center_x(), g.top() + 90.0).unwrap();
    assert!(hidden.get());
    assert!(f.t.text().is_empty());
}
#[test]
fn delete_swipe_left_deletes_words_precisely() {
    let f = Fixture::new("one two three", |_| {});
    let del =
        f.e.keyboard()
            .unwrap()
            .find_key(KeyCode::Delete)
            .unwrap()
            .unwrap()
            .touch_bounds();
    let (x, y) = (del.center_x(), del.center_y());
    f.touch.pointer_down(1, x, y).unwrap();
    f.touch.pointer_move(1, x - 20.0, y).unwrap();
    f.touch.pointer_move(1, x - 35.0, y).unwrap();
    f.touch.pointer_up(1, x - 35.0, y).unwrap();
    assert_eq!(f.t.text(), Utf16Text::from("one "));
}
#[test]
fn long_press_shift_enables_caps_lock() {
    let f = Fixture::new("", |_| {});
    let shift =
        f.e.keyboard()
            .unwrap()
            .find_key(KeyCode::Shift)
            .unwrap()
            .unwrap()
            .touch_bounds();
    f.touch
        .pointer_down(1, shift.center_x(), shift.center_y())
        .unwrap();
    f.scheduler
        .advance((f.e.settings().long_press_delay() as i64 + 10) * 10000);
    f.touch
        .pointer_up(1, shift.center_x(), shift.center_y())
        .unwrap();
    assert_eq!(f.e.shift_state(), ShiftState::CapsLock);
}
