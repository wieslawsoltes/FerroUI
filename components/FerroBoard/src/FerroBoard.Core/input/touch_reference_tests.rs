use super::*;
use crate::fixture_clock;
use crate::text::Utf16Text;
use crate::{
    editor::TextBufferTarget, engine::LanguageModels, layouts::KeyboardResources,
    text::ordinal_ignore_case_equal,
};
use serde_json::{Value, json};
use std::time::Duration;
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/touch.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn bits(n: f64) -> String {
    format!("{:016x}", n.to_bits())
}
fn float(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn number(v: &Value) -> f64 {
    if let Some(value) = v.get("f64") {
        float(value)
    } else if let Some(s) = v.as_str() {
        match s {
            "nan" => f64::from_bits(0xfff8000000000000),
            "inf" => f64::INFINITY,
            "-inf" => f64::NEG_INFINITY,
            "-zero" => -0.0,
            s => s.parse().unwrap(),
        }
    } else {
        v.as_f64().unwrap()
    }
}
fn rect(r: KeyRect) -> Value {
    json!([bits(r.x), bits(r.y), bits(r.width), bits(r.height)])
}
fn key(k: &ComputedKey) -> Value {
    json!({"Row":k.row(),"Column":k.column(),"code":k.code(),"label":k.label().map(|s|json!(s.units()))})
}
struct Token {
    id: i64,
    due: i64,
    action: Rc<dyn Fn()>,
    owner: Weak<RecorderScheduler>,
    handle: RefCell<Option<Weak<dyn std::any::Any>>>,
}
impl Token {
    fn cancel(&self) {
        if let Some(owner) = self.owner.upgrade() {
            owner
                .events
                .borrow_mut()
                .push(json!({"kind":"cancel","id":self.id}));
            owner.entries.borrow_mut().retain(|t| t.id != self.id);
        }
    }
}
struct RecorderScheduler {
    self_ref: Weak<Self>,
    now: Cell<i64>,
    next: Cell<i64>,
    entries: RefCell<Vec<Rc<Token>>>,
    all: RefCell<Vec<Rc<Token>>>,
    events: RefCell<Vec<Value>>,
}
impl RecorderScheduler {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            self_ref: weak.clone(),
            now: Cell::new(0),
            next: Cell::new(0),
            entries: RefCell::new(Vec::new()),
            all: RefCell::new(Vec::new()),
            events: RefCell::new(Vec::new()),
        })
    }
    fn pending(&self) -> Value {
        json!(
            self.entries
                .borrow()
                .iter()
                .map(|t| json!({"id":t.id,"due":t.due}))
                .collect::<Vec<_>>()
        )
    }
    fn token_id(&self, timer: &RefCell<Option<ScheduledAction>>) -> Option<i64> {
        let timer = timer.borrow().clone()?;
        self.all
            .borrow()
            .iter()
            .find(|t| {
                std::ptr::addr_eq(
                    t.handle.borrow().as_ref().unwrap().as_ptr(),
                    timer.test_identity().as_ptr(),
                )
            })
            .map(|t| t.id)
    }
    fn advance(&self, ticks: i64) {
        let target = self.now.get() + ticks;
        loop {
            let index = {
                let entries = self.entries.borrow();
                let mut index = None;
                let mut due = 0;
                for (i, t) in entries.iter().enumerate() {
                    if t.due <= target && (index.is_none() || t.due < due) {
                        index = Some(i);
                        due = t.due;
                    }
                }
                index
            };
            let Some(index) = index else {
                break;
            };
            let t = self.entries.borrow_mut().remove(index);
            self.now.set(t.due);
            self.events
                .borrow_mut()
                .push(json!({"kind":"fire","id":t.id}));
            (t.action)();
        }
        self.now.set(target);
    }
    fn fire(&self, id: i64) {
        let token = self
            .all
            .borrow()
            .iter()
            .find(|t| t.id == id)
            .unwrap()
            .clone();
        self.events.borrow_mut().push(json!({"kind":"old","id":id}));
        (token.action)();
    }
}
impl IKeyboardScheduler for RecorderScheduler {
    fn schedule(&self, delay: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        let id = self.next.get();
        self.next.set(id + 1);
        let t = Rc::new(Token {
            id,
            due: self.now.get() + i64::try_from(delay.as_nanos() / 100).unwrap(),
            action,
            owner: self.self_ref.clone(),
            handle: RefCell::new(None),
        });
        self.entries.borrow_mut().push(t.clone());
        self.all.borrow_mut().push(t.clone());
        self.events
            .borrow_mut()
            .push(json!({"kind":"schedule","id":id,"due":t.due}));
        let work = t.clone();
        let guard = ScheduledAction::new(move || work.cancel());
        *t.handle.borrow_mut() = Some(guard.test_identity());
        guard
    }
}
fn state(e: &KeyboardEngine, t: &KeyboardTouchProcessor, s: &RecorderScheduler) -> Value {
    let pointers=t.pointers.snapshot().into_iter().map(|p|json!({"id":p.id,"phase":p.phase.get()as i32,"x":bits(p.start_x),"y":bits(p.start_y),"lastX":bits(p.last_x.get()),"lastY":bits(p.last_y.get()),"anchor":bits(p.anchor_x.get()),"key":p.key.as_ref().map(|k|key(k)),"repeated":p.repeated.get(),"steps":p.steps.get(),"longTimer":s.token_id(&p.long_press_timer),"repeatTimer":s.token_id(&p.repeat_timer)})).collect::<Vec<_>>();
    json!({"text":e.target().map_or_default(|t|t.text()).units(),"start":e.target().map_or(0,|t|t.selection_start()),"length":e.target().map_or(0,|t|t.selection_length()),"shift":e.shift_state().0,"mode":e.mode().0,"panel":e.ui_mode().0,"gliding":e.is_gliding(),"version":e.keyboard().unwrap().version(),"pressed":t.pressed_keys().into_iter().map(|k|key(&k.unwrap())).collect::<Vec<_>>(),"pointers":pointers,"popup":t.popup().map(|p|json!({"key":key(&p.key),"keys":p.keys.snapshot().unwrap().iter().map(|k|json!({"code":k.data().code(),"text":k.as_string(false).unwrap().unwrap().units()})).collect::<Vec<_>>(),"bounds":p.bounds.borrow().iter().map(|b|rect(*b)).collect::<Vec<_>>(),"selected":p.selected_index.get(),"frame":rect(p.frame())})),"trail":t.glide_trail.borrow().iter().map(|p|[bits(p.x),bits(p.y)]).collect::<Vec<_>>(),"suggestions":e.suggestions().snapshot().unwrap().into_iter().map(|s|s.expect("ordinary source fixture has nonnull candidates")).map(|v|json!({"text":v.text().as_ref().map(Utf16Text::units),"kind":v.kind().0,"confidence":bits(v.confidence())})).collect::<Vec<_>>(),"pending":s.pending(),"now":s.now.get()})
}
fn find_key(e: &KeyboardEngine, value: &Value) -> Rc<ComputedKey> {
    let keyboard = e.keyboard().unwrap();
    if let Some(code) = value.as_i64() {
        keyboard.find_key(code as i32).unwrap().unwrap()
    } else {
        keyboard
            .visible_keys()
            .map(|key| key.unwrap().unwrap())
            .find(|k| {
                k.label().is_some_and(|label| {
                    ordinal_ignore_case_equal(&label, &value.as_str().unwrap().into())
                })
            })
            .unwrap()
    }
}
fn point(e: &KeyboardEngine, op: &[Value], index: usize) -> (f64, f64) {
    if let Some(raw) = op[index].as_array() {
        (number(&raw[0]), number(&raw[1]))
    } else {
        let b = find_key(e, &op[index]).touch_bounds();
        (
            b.center_x() + op.get(index + 1).map_or(0.0, number),
            b.center_y() + op.get(index + 2).map_or(0.0, number),
        )
    }
}
struct Fixture {
    e: Rc<KeyboardEngine>,
    t: Rc<KeyboardTouchProcessor>,
    s: Rc<RecorderScheduler>,
    events: Rc<RefCell<Vec<Value>>>,
}
impl Fixture {
    fn new(initial: &str, reenter: Option<&str>) -> Self {
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_auto_capitalization(false);
        let e = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        let s = RecorderScheduler::new();
        let clock = s.clone();
        e.set_clock(Rc::new(move || {
            fixture_clock::time(639028656000000000 + clock.now.get())
        }));
        e.attach(Rc::new(TextBufferTarget::new(initial.into(), None)))
            .unwrap();
        e.update_layout(1000.0, 240.0, None).unwrap();
        let t = KeyboardTouchProcessor::new(e.clone(), s.clone());
        let events = Rc::new(RefCell::new(Vec::new()));
        for (kind, event) in [("press", t.key_pressed()), ("release", t.key_released())] {
            let events = events.clone();
            let w = Rc::downgrade(&t);
            let se = Rc::downgrade(&s);
            let en = Rc::downgrade(&e);
            event.add(Rc::new(move |k|{let(t,s,e)=(w.upgrade().unwrap(),se.upgrade().unwrap(),en.upgrade().unwrap());events.borrow_mut().push(json!({"kind":kind,"key":key(k),"feedback":Value::Null,"state":state(&e,&t,&s)}));}));
        }
        for (kind, event) in [
            ("popup", t.popup_changed()),
            ("trail", t.glide_trail_changed()),
            ("ended", t.glide_ended()),
        ] {
            let events = events.clone();
            let se = Rc::downgrade(&s);
            let en = Rc::downgrade(&e);
            event.add(Rc::new(move |t|{let(s,e)=(se.upgrade().unwrap(),en.upgrade().unwrap());events.borrow_mut().push(json!({"kind":kind,"key":Value::Null,"feedback":Value::Null,"state":state(&e,t,&s)}));}));
        }
        for (kind, event) in [
            ("keyboard", e.keyboard_changed()),
            ("state", e.state_changed()),
            ("suggestions", e.suggestions_changed()),
            ("hide", e.hide_requested()),
            ("subtypes", e.subtype_picker_requested()),
        ] {
            let events = events.clone();
            let w = Rc::downgrade(&t);
            let se = Rc::downgrade(&s);
            event.add(Rc::new(move |e|{let(t,s)=(w.upgrade().unwrap(),se.upgrade().unwrap());events.borrow_mut().push(json!({"kind":kind,"key":Value::Null,"feedback":Value::Null,"state":state(e,&t,&s)}));}));
        }
        let events0 = events.clone();
        let w = Rc::downgrade(&t);
        let se = Rc::downgrade(&s);
        let en = Rc::downgrade(&e);
        e.feedback_requested().add(Rc::new(move|kind|{let(t,s,e)=(w.upgrade().unwrap(),se.upgrade().unwrap(),en.upgrade().unwrap());events0.borrow_mut().push(json!({"kind":"feedback","key":Value::Null,"feedback":kind.0,"state":state(&e,&t,&s)}));}));
        let events0 = events.clone();
        let w = Rc::downgrade(&t);
        let se = Rc::downgrade(&s);
        let en = Rc::downgrade(&e);
        e.text_committed().add(Rc::new(move|_|{let(t,s,e)=(w.upgrade().unwrap(),se.upgrade().unwrap(),en.upgrade().unwrap());events0.borrow_mut().push(json!({"kind":"committed","key":Value::Null,"feedback":Value::Null,"state":state(&e,&t,&s)}));}));
        if let Some(mode) = reenter {
            let once = Cell::new(true);
            let release_down = mode == "release-down";
            let throw_press = mode == "press-throw";
            let w = Rc::downgrade(&t);
            let en = Rc::downgrade(&e);
            let action: Rc<dyn Fn()> = Rc::new(move || {
                let (t, e) = (w.upgrade().unwrap(), en.upgrade().unwrap());
                if once.replace(false) {
                    assert!(!throw_press, "touch fixture callback");
                    if release_down {
                        let b = find_key(&e, &json!("s")).touch_bounds();
                        t.pointer_down(1, b.center_x(), b.center_y()).unwrap();
                    } else {
                        t.cancel_all().unwrap();
                    }
                }
            });
            match mode {
                "press-cancel" | "press-throw" => {
                    t.key_pressed().add(Rc::new(move |_| action()));
                }
                "release-down" => {
                    t.key_released().add(Rc::new(move |_| action()));
                }
                "popup-cancel" => {
                    t.popup_changed().add(Rc::new(move |t| {
                        if t.popup().is_some() {
                            action();
                        }
                    }));
                }
                "feedback-cancel" => {
                    e.feedback_requested().add(Rc::new(move |_| action()));
                }
                "repeat-clock-throw" => e
                    .editor()
                    .unwrap()
                    .set_clock(Rc::new(|| panic!("touch fixture callback"))),
                _ => unreachable!(),
            }
        }
        Self { e, t, s, events }
    }
    fn apply(&self, op: &[Value]) -> TouchResult<()> {
        let (e, t, s) = (&self.e, &self.t, &self.s);
        match op[0].as_str().unwrap() {
            "down" | "move" | "up" => {
                let (x, y) = point(e, op, 2);
                let id = op[1].as_i64().unwrap();
                match op[0].as_str().unwrap() {
                    "down" => t.pointer_down(id, x, y),
                    "move" => t.pointer_move(id, x, y),
                    _ => t.pointer_up(id, x, y),
                }
            }
            "cancel" => t.pointer_cancel(op[1].as_i64().unwrap()),
            "cancel-all" => t.cancel_all(),
            "clear-trail" => {
                t.clear_glide_trail();
                Ok(())
            }
            "advance" => {
                s.advance(op[1].as_i64().unwrap());
                Ok(())
            }
            "fire-old" => {
                s.fire(op[1].as_i64().unwrap());
                Ok(())
            }
            "popup-move" | "popup-up" => {
                let b = t.popup().unwrap().bounds.borrow()[op[2].as_u64().unwrap() as usize];
                let id = op[1].as_i64().unwrap();
                if op[0] == "popup-move" {
                    t.pointer_move(id, b.center_x(), b.center_y())
                } else {
                    t.pointer_up(id, b.center_x(), b.center_y())
                }
            }
            "popup-below" => {
                let p = t.popup().unwrap();
                let b = p.key.visible_bounds();
                t.pointer_move(
                    op[1].as_i64().unwrap(),
                    b.center_x(),
                    b.bottom() + b.height + number(&op[2]),
                )
            }
            "layout" => Ok(e.update_layout(number(&op[1]), number(&op[2]), None)?),
            "detach" => Ok(e.detach()?),
            "setting" => {
                let settings = e.settings();
                match op[1].as_str().unwrap() {
                    "GlideTyping" => settings.set_glide_typing(op[2].as_bool().unwrap()),
                    "DeleteKeySwipeLeft" => settings
                        .set_delete_key_swipe_left(SwipeAction(op[2].as_i64().unwrap() as i32)),
                    "DeleteKeyLongPress" => settings
                        .set_delete_key_long_press(SwipeAction(op[2].as_i64().unwrap() as i32)),
                    "KeyRepeatInterval" => {
                        settings.set_key_repeat_interval(op[2].as_i64().unwrap() as i32)
                    }
                    name => panic!("unknown setting {name}"),
                };
                Ok(())
            }
            "property" => {
                match op[1].as_str().unwrap() {
                    "CursorStepDistance" => t.set_cursor_step_distance(number(&op[2])),
                    name => panic!("unknown property {name}"),
                };
                Ok(())
            }
            "glide-words" => {
                e.glide_classifier()
                    .set_words(
                        [("hello", 200), ("help", 150), ("held", 120), ("world", 220)]
                            .map(|(w, f)| (w.into(), f)),
                    )
                    .map_err(EngineError::from)?;
                Ok(())
            }
            "path" => {
                let labels = op[2].as_str().unwrap().chars().collect::<Vec<_>>();
                for j in 1..labels.len() {
                    let a = find_key(e, &json!(labels[j - 1].to_string())).visible_bounds();
                    let b = find_key(e, &json!(labels[j].to_string())).visible_bounds();
                    let mut v = 0.1;
                    while v <= 1.0001 {
                        t.pointer_move(
                            op[1].as_i64().unwrap(),
                            a.center_x() + (b.center_x() - a.center_x()) * v,
                            a.center_y() + (b.center_y() - a.center_y()) * v,
                        )?;
                        v += 0.1;
                    }
                }
                Ok(())
            }
            name => panic!("unknown op {name}"),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.t.cancel_all().unwrap();
        self.e.dispose().unwrap();
        self.s.entries.borrow_mut().clear();
        self.s.all.borrow_mut().clear();
    }
}
// Rust guards cancel on final drop and cancellation is idempotent. Managed
// fixtures also log Dispose calls on already-fired/cancelled handles. Those
// infrastructure-only calls are intentionally outside the native contract.
// Keep exact schedule/fire/old events and cancellation of pending callbacks;
// every full keyboard state, pending queue and input event is still compared.
#[derive(Default)]
struct SchedulerTrace {
    retired: std::collections::HashSet<i64>,
    ignored_completed_cancels: usize,
}
impl SchedulerTrace {
    fn pending_events(&mut self, events: &Value) -> Value {
        Value::Array(
            events
                .as_array()
                .unwrap()
                .iter()
                .filter(|event| {
                    let id = event["id"].as_i64().unwrap();
                    match event["kind"].as_str().unwrap() {
                        "fire" => {
                            self.retired.insert(id);
                            true
                        }
                        "cancel" => {
                            if self.retired.insert(id) {
                                true
                            } else {
                                self.ignored_completed_cancels += 1;
                                false
                            }
                        }
                        _ => true,
                    }
                })
                .cloned()
                .collect(),
        )
    }
}
#[test]
fn source_touch_states_events_timers_precedence_and_reentrancy() {
    let rows = rows();
    let mut names = Vec::new();
    for row in &rows {
        if row["kind"] == "op" && !names.contains(&row["name"]) {
            names.push(row["name"].clone());
        }
    }
    for name in names {
        let cases = rows
            .iter()
            .filter(|r| r["kind"] == "op" && r["name"] == name)
            .collect::<Vec<_>>();
        let f = Fixture::new(
            cases[0]["initial"].as_str().unwrap(),
            cases[0]["reenter"].as_str(),
        );
        let mut actual_scheduler = SchedulerTrace::default();
        let mut expected_scheduler = SchedulerTrace::default();
        for (index, row) in cases.into_iter().enumerate() {
            f.events.borrow_mut().clear();
            f.s.events.borrow_mut().clear();
            let op = row["op"].as_array().unwrap();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f.apply(op)));
            let error = match outcome {
                Ok(result) => result.err().map(|error| match error {
                    TouchError::StepCountOverflow => "OverflowException",
                    e => panic!("unexpected {e:?}"),
                }),
                Err(payload) => {
                    assert!(
                        payload.downcast_ref::<&str>() == Some(&"touch fixture callback")
                            || payload
                                .downcast_ref::<String>()
                                .is_some_and(|s| s == "touch fixture callback")
                    );
                    Some("ApplicationException")
                }
            };
            assert_eq!(json!(error), row["error"], "{name} {index} {op:?}");
            assert_eq!(
                state(&f.e, &f.t, &f.s),
                row["state"],
                "{name} {index} {op:?}"
            );
            assert_eq!(
                json!(*f.events.borrow()),
                row["events"],
                "events {name} {index} {op:?}"
            );
            assert_eq!(
                actual_scheduler.pending_events(&json!(*f.s.events.borrow())),
                expected_scheduler.pending_events(&row["scheduler"]),
                "pending scheduler {name} {index} {op:?}"
            );
        }
    }
}
#[test]
fn native_pointer_ids_preserve_source_rollover_shift_and_cancellation_order() {
    fn pointer_id(id: i64, variant: usize) -> i64 {
        match (variant, id) {
            (0, 1) => i64::MAX,
            (0, 2) => i64::MIN,
            (0, 3) => 0,
            (0, 4) => -1,
            (0, _) => i64::MAX - id,
            (_, _) => -id * 1_000_003,
        }
    }
    fn remap_states(value: &mut Value, variant: usize) {
        match value {
            Value::Object(object) => {
                if let Some(Value::Array(pointers)) = object.get_mut("pointers") {
                    for pointer in pointers {
                        pointer["id"] = json!(pointer_id(pointer["id"].as_i64().unwrap(), variant));
                    }
                }
                for value in object.values_mut() {
                    remap_states(value, variant);
                }
            }
            Value::Array(values) => {
                for value in values {
                    remap_states(value, variant);
                }
            }
            _ => {}
        }
    }
    let rows = rows();
    for variant in 0..2 {
        for name in ["rollover", "duplicate", "shift", "repeat-old"] {
            let cases: Vec<_> = rows
                .iter()
                .filter(|row| row["kind"] == "op" && row["name"] == name)
                .collect();
            assert!(!cases.is_empty(), "missing source scenario {name}");
            let f = Fixture::new(cases[0]["initial"].as_str().unwrap(), None);
            let mut actual_scheduler = SchedulerTrace::default();
            let mut expected_scheduler = SchedulerTrace::default();
            for row in cases {
                assert!(row["reenter"].is_null() && row["error"].is_null());
                f.events.borrow_mut().clear();
                f.s.events.borrow_mut().clear();
                let mut op = row["op"].as_array().unwrap().clone();
                if matches!(op[0].as_str().unwrap(), "down" | "move" | "up" | "cancel") {
                    op[1] = json!(pointer_id(op[1].as_i64().unwrap(), variant));
                }
                f.apply(&op).unwrap();
                let mut expected_state = row["state"].clone();
                let mut expected_events = row["events"].clone();
                remap_states(&mut expected_state, variant);
                remap_states(&mut expected_events, variant);
                assert_eq!(state(&f.e, &f.t, &f.s), expected_state, "{name} {op:?}");
                assert_eq!(json!(*f.events.borrow()), expected_events, "{name} {op:?}");
                assert_eq!(
                    actual_scheduler.pending_events(&json!(*f.s.events.borrow())),
                    expected_scheduler.pending_events(&row["scheduler"]),
                    "pending scheduler {name} {op:?}"
                );
            }
        }
    }
}

#[test]
fn source_popup_arrangement_and_frame_bits() {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_glide_typing(false);
    let e = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap();
    let mut last_size = None;
    for row in rows() {
        if row["kind"] == "arrange" {
            if last_size.as_ref() != Some(&row["size"]) {
                e.update_layout(float(&row["size"][0]), float(&row["size"][1]), None)
                    .unwrap();
                last_size = Some(row["size"].clone());
            }
            let k = find_key(&e, &row["label"]);
            let (bounds, selected) = KeyboardTouchProcessor::arrange_popup(
                &k,
                row["count"].as_i64().unwrap() as i32,
                row["defaultIndex"].as_i64().unwrap() as i32,
                float(&row["size"][0]),
                row["columns"].as_i64().unwrap() as i32,
                -48.0,
            )
            .unwrap();
            assert_eq!(
                json!(bounds.borrow().iter().map(|b| rect(*b)).collect::<Vec<_>>()),
                row["bounds"],
                "{row}"
            );
            assert_eq!(selected, row["selected"], "{row}");
        } else if row["kind"] == "frame" {
            let bounds = row["bounds"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| KeyRect {
                    x: float(&b[0]),
                    y: float(&b[1]),
                    width: float(&b[2]),
                    height: float(&b[3]),
                })
                .collect();
            let popup = KeyPopupState {
                key: e
                    .keyboard()
                    .unwrap()
                    .keys()
                    .map(|key| key.unwrap().unwrap())
                    .next()
                    .unwrap(),
                keys: PopupKeys::default(),
                bounds: Rc::new(RefCell::new(bounds)),
                selected_index: Cell::new(0),
            };
            assert_eq!(rect(popup.frame()), row["frame"], "{row}");
        }
    }
    e.dispose().unwrap();
}

#[test]
fn source_pressed_query_is_deferred_and_preserves_enumerator_invalidation() {
    let f = Fixture::new("", None);
    let query = f.t.pressed_keys();
    let mut lazy = query.iter();
    let down = |id, code| {
        let b =
            f.e.keyboard()
                .unwrap()
                .find_key(code)
                .unwrap()
                .unwrap()
                .touch_bounds();
        f.t.pointer_down(id, b.center_x(), b.center_y()).unwrap();
    };
    fn next(it: &mut PressedKeysIterator) -> Value {
        match it.next() {
            Some(Ok(k)) => json!({"key":key(&k),"error":Value::Null}),
            Some(Err(TouchError::PointerCollectionModified)) => {
                json!({"key":Value::Null,"error":"InvalidOperationException"})
            }
            None => json!({"key":Value::Null,"error":Value::Null}),
            other => panic!("unexpected iterator result {}", other.is_some()),
        }
    }
    down(1, 97);
    let mut trace = vec![next(&mut lazy)];
    down(2, 115);
    trace.push(next(&mut lazy));
    trace.push(next(&mut lazy));
    f.t.cancel_all().unwrap();
    trace.push(next(&mut lazy));
    let mut ended = query.iter();
    trace.push(next(&mut ended));
    down(3, 100);
    trace.push(next(&mut ended));
    f.t.cancel_all().unwrap();
    down(4, KeyCode::Shift);
    down(5, KeyCode::Delete);
    down(6, KeyCode::Enter);
    let mut removed = query.iter();
    trace.push(next(&mut removed));
    f.t.pointer_cancel(4).unwrap();
    trace.push(next(&mut removed));
    f.t.pointer_cancel(6).unwrap();
    trace.push(next(&mut removed));
    f.t.cancel_all().unwrap();
    let expected = rows()
        .into_iter()
        .find(|r| r["kind"] == "pressed-query")
        .unwrap();
    assert_eq!(json!(trace), expected["trace"]);
}
