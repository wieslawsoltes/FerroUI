//! Replay unchanged C# behavior and an actual native trace across layout replacement.
use ferroboard_core::input::ScheduledAction;
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::{KeyboardEngine, LanguageModels},
    input::{IKeyboardScheduler, KeyboardTouchProcessor},
    layouts::{KeyRect, KeyboardResources},
    settings::KeyboardSettings,
};
use serde_json::{Value, json};
use std::time::Duration;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/native-layout-change.json"
    ))
    .unwrap()
}

fn engine(glide: bool) -> (Rc<KeyboardEngine>, Rc<TextBufferTarget>) {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_auto_capitalization(false);
    settings.set_auto_correct(false);
    settings.set_glide_typing(glide);
    settings.set_glide_preview(true);
    settings.set_suggestion_count(3);
    settings.set_learn_words(false);
    let engine = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap();
    let target = Rc::new(TextBufferTarget::default());
    engine.attach(target.clone()).unwrap();
    (engine, target)
}

// Explicitly retain or fire callbacks, matching the two public reference fixtures.
#[derive(Default)]
struct PendingScheduler {
    active: Rc<Cell<usize>>,
    timers: RefCell<Vec<Rc<PendingTimer>>>,
}
impl PendingScheduler {
    fn run_next(&self) {
        let timer = self
            .timers
            .borrow()
            .iter()
            .find(|t| !t.disposed.get())
            .unwrap()
            .clone();
        (timer.action)();
    }
}
struct PendingTimer {
    active: Rc<Cell<usize>>,
    disposed: Cell<bool>,
    action: Rc<dyn Fn()>,
}
impl IKeyboardScheduler for PendingScheduler {
    fn schedule(&self, _: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        self.active.set(self.active.get() + 1);
        let timer = Rc::new(PendingTimer {
            active: self.active.clone(),
            disposed: Cell::new(false),
            action,
        });
        self.timers.borrow_mut().push(timer.clone());
        ScheduledAction::new(move || timer.cancel())
    }
}
impl PendingTimer {
    fn cancel(&self) {
        if !self.disposed.replace(true) {
            self.active.set(self.active.get() - 1);
        }
    }
}

#[test]
fn held_key_survives_number_row_replacement_and_release_cancels_timers() {
    let expected = fixture();
    let (engine, target) = engine(false);
    engine.update_layout(720.0, 240.0, None).unwrap();
    let old = engine.keyboard().unwrap();
    let key = old
        .character_keys()
        .unwrap()
        .into_iter()
        .find(|key| key.data().as_string(false).unwrap() == Some("a".into()))
        .unwrap();
    let bounds = key.touch_bounds();
    let scheduler = Rc::new(PendingScheduler::default());
    let touch = KeyboardTouchProcessor::new(engine.clone(), scheduler.clone());
    touch
        .pointer_down(29, bounds.center_x(), bounds.center_y())
        .unwrap();
    let before = touch.pressed_keys().into_iter().next().unwrap().unwrap();
    assert!(scheduler.active.get() > 0);
    engine.settings().set_number_row(true);
    let after = touch.pressed_keys().into_iter().next().unwrap().unwrap();
    let changed = !Rc::ptr_eq(&old, &engine.keyboard().unwrap());
    let retained = Rc::ptr_eq(&before, &after);
    let during = target.text();
    touch
        .pointer_up(29, bounds.center_x(), bounds.center_y())
        .unwrap();
    assert_eq!(
        json!({"changed": changed, "retained": retained,
            "during": String::from_utf16(during.units()).unwrap(),
            "committed": String::from_utf16(target.text().units()).unwrap(),
            "pressedAfter": touch.pressed_keys().into_iter().count(), "activeTimers": scheduler.active.get()}),
        expected["heldKey"]
    );
    engine.dispose().unwrap();
}

#[test]
fn long_press_popup_survives_number_row_replacement_with_source_choices_and_commit() {
    let expected = fixture();
    let (engine, target) = engine(false);
    engine.update_layout(720.0, 240.0, None).unwrap();
    let old = engine.keyboard().unwrap();
    let key = old
        .character_keys()
        .unwrap()
        .into_iter()
        .find(|k| k.data().as_string(false).unwrap() == Some("a".into()))
        .unwrap();
    let b = key.touch_bounds();
    let scheduler = Rc::new(PendingScheduler::default());
    let touch = KeyboardTouchProcessor::new(engine.clone(), scheduler.clone());
    touch.pointer_down(29, b.center_x(), b.center_y()).unwrap();
    scheduler.run_next();
    let popup = touch.popup().unwrap();
    engine.settings().set_number_row(true);
    let changed = !Rc::ptr_eq(&old, &engine.keyboard().unwrap());
    let retained = Rc::ptr_eq(&popup, &touch.popup().unwrap())
        && Rc::ptr_eq(&key, &touch.popup().unwrap().key());
    let during = target.text();
    let selected = popup.selected_index();
    let choices: Vec<_> = popup
        .keys()
        .iter()
        .map(|k| {
            k.unwrap()
                .as_string(false)
                .unwrap()
                .unwrap()
                .units()
                .to_vec()
        })
        .collect();
    touch.pointer_up(29, b.center_x(), b.center_y()).unwrap();
    assert_eq!(
        json!({"changed":changed,"retained":retained,"during":String::from_utf16(during.units()).unwrap(),
        "selectedIndex":selected,"choices":choices,"committed":target.text().units(),
        "pressedAfter":touch.pressed_keys().into_iter().count(),"popupAfter":touch.popup().is_some(),
        "activeTimers":scheduler.active.get()}),
        expected["popupKey"]
    );
    engine.dispose().unwrap();
}

fn number(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn set_layout(engine: &KeyboardEngine, layout: &Value) {
    engine
        .glide_classifier()
        .set_layout(layout.as_array().unwrap().iter().map(|key| {
            let r = &key["r"];
            (
                key["c"].as_u64().unwrap() as u16,
                KeyRect::new(number(&r[0]), number(&r[1]), number(&r[2]), number(&r[3])),
            )
        }))
        .unwrap();
}

#[test]
fn real_native_glide_keeps_samples_across_changed_classifier_geometry() {
    let fixture = fixture();
    assert_eq!(
        fixture["sourceRevision"],
        "3694995fabbebc45a801a455880e3d31888367f1"
    );
    let case = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "logical")
        .unwrap();
    let (engine, target) = engine(true);
    let points = case["points"].as_array().unwrap();
    set_layout(&engine, &case["initialLayout"]);
    engine
        .begin_glide(number(&points[0][0]), number(&points[0][1]))
        .unwrap();
    let mut transitions = Vec::new();
    for (i, point) in points.iter().enumerate().skip(1) {
        for update in case["layoutUpdates"].as_array().unwrap() {
            if update["after"].as_u64().unwrap() as usize == i {
                let before = engine.glide_classifier().point_count();
                set_layout(&engine, &update["layout"]);
                transitions.push(json!({"after":i,"beforeCount":before,
                    "afterCount":engine.glide_classifier().point_count(),
                    "active":engine.is_gliding(),"text":String::from_utf16(target.text().units()).unwrap()}));
            }
        }
        engine
            .add_glide_point(number(&point[0]), number(&point[1]))
            .unwrap();
    }
    assert_eq!(json!(transitions), case["transitionStates"]);
    assert_eq!(
        engine.glide_classifier().point_count(),
        case["pointCount"].as_u64().unwrap() as usize
    );
    engine.finish_glide(true).unwrap();
    assert_eq!(json!(target.text().units()), case["committed"]);
    assert_eq!(
        engine.shift_state().0,
        case["shiftAfter"].as_i64().unwrap() as i32
    );
    let ranked: Vec<_> = engine
        .suggestions()
        .snapshot()
        .unwrap()
        .into_iter()
        .map(|s| s.unwrap().text().unwrap().units().to_vec())
        .collect();
    assert_eq!(json!(ranked), case["ranked"]);
    engine.dispose().unwrap();
}
