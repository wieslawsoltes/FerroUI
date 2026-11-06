//! Actual unchanged Core shift/panel enum transitions, selector dispatch and callback snapshots.
#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
use ferroboard_core::{editor::*, engine::*, keys::*, layouts::*, settings::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    sync::Arc,
};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/state-enums.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn bits(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}
fn auto_key(code: i32, label: Utf16Text) -> Rc<AutoTextKeyData> {
    Rc::new(AutoTextKeyData {
        base: TextKeyData::from_fields(TextKeyDataFields {
            code,
            label: Some(label),
            ..Default::default()
        }),
    })
}
fn resources() -> Rc<KeyboardResources> {
    let r = Rc::new(KeyboardResources::create_empty());
    r.add_layout(Rc::new(KeyboardLayout::new(
        LayoutType::Characters,
        "org.florisboard.layouts".into(),
        "qwerty".into(),
        Rc::new(LayoutArrangement::from_rows(vec![vec![
            auto_key(97, "a".into()),
            TextKeyData::function(KeyCode::Shift, KeyType::Function, None),
        ]])),
    )));
    r
}
fn engine(auto: bool) -> Rc<KeyboardEngine> {
    let s = Rc::new(KeyboardSettings::new());
    s.set_glide_typing(false);
    s.set_auto_capitalization(auto);
    s.set_show_suggestions(false);
    s.set_next_word_prediction(false);
    s.set_emoji_suggestions(false);
    s.set_clipboard_suggestion(false);
    let e = KeyboardEngine::new_explicit(s, resources(), LanguageModels::new()).unwrap();
    e.set_clock(Rc::new(|| fixture_clock::time(639028224000000000)));
    e
}
fn attach(e: &Rc<KeyboardEngine>, cap: CapitalizationMode) {
    e.attach(Rc::new(TextBufferTarget::new(
        "ab".into(),
        Some(Arc::new(InputAttributes::from_values(
            ferroboard_core::editor::InputAttributesValues {
                capitalization: cap,
                ..Default::default()
            },
        ))),
    )))
    .unwrap();
}
fn keys(k: &ComputedKeyboard) -> Vec<Value> {
    k.keys().map(|key| key.unwrap().unwrap()).map(|v|json!({"Code":v.code(),"label":v.label().map(|s|s.units().to_vec()),"icon":(v.icon()).0})).collect()
}
fn state(e: &KeyboardEngine, original: &Rc<ComputedKeyboard>) -> Value {
    let k = e.keyboard().unwrap();
    let t = e.target();
    json!({"shift":e.shift_state().0,"panel":e.ui_mode().0,"mode":e.mode().0,"context":e.create_compute_context().unwrap().shift_state().0,"computed":k.context().unwrap().shift_state().0,"same":Rc::ptr_eq(&k,original),"version":k.version(),"selecting":e.editor().is_some_and(|v|v.is_selecting()),"text":t.as_ref().map(|t|t.text().units().to_vec()),"start":t.as_ref().map(|t|t.selection_start()),"length":t.as_ref().map(|t|t.selection_length()),"keys":keys(&k),"suggestions":e.suggestions().len().unwrap(),"loaded":e.language_models().is_loaded(&"en".into())})
}
type Log = Rc<RefCell<Vec<Value>>>;
fn observe(e: &Rc<KeyboardEngine>, original: &Rc<ComputedKeyboard>) -> Log {
    let log = Log::default();
    for (name, event) in [
        ("keyboard", e.keyboard_changed()),
        ("state", e.state_changed()),
        ("suggestions", e.suggestions_changed()),
    ] {
        let log = log.clone();
        let weak = Rc::downgrade(e);
        let original = original.clone();
        event.add(Rc::new(move |_| {
            let s = state(&weak.upgrade().unwrap(), &original);
            log.borrow_mut().push(json!({"kind":name,"state":s}));
        }));
    }
    let weak = Rc::downgrade(e);
    let original_text = original.clone();
    let log_text = log.clone();
    e.text_committed().add(Rc::new(move |v| {
        let s = state(&weak.upgrade().unwrap(), &original_text);
        log_text
            .borrow_mut()
            .push(json!({"kind":"text","value":v.units(),"state":s}));
    }));
    let weak = Rc::downgrade(e);
    let original = original.clone();
    let log_feedback = log.clone();
    e.feedback_requested().add(Rc::new(move |v| {
        let s = state(&weak.upgrade().unwrap(), &original);
        log_feedback
            .borrow_mut()
            .push(json!({"kind":"feedback","value":v.0,"state":s}));
    }));
    log
}
#[derive(Debug)]
struct ProbeFailure;
fn attempt(action: impl FnOnce() -> EngineResult<()>) -> Value {
    match catch_unwind(AssertUnwindSafe(action)) {
        Ok(Ok(())) => json!({"value":null}),
        Ok(Err(e)) => panic!("unexpected production failure: {e:?}"),
        Err(p) if p.is::<ProbeFailure>() => json!({"error":"InvalidOperationException"}),
        Err(p) => std::panic::resume_unwind(p),
    }
}
#[test]
fn source_raw_shift_flags_exact_selectors_and_auto_key_identity() {
    for row in rows() {
        match row["kind"].as_str().unwrap() {
            "enum" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                assert_eq!(
                    format!("{:?}", ShiftState(raw)),
                    row["shiftName"].as_str().unwrap()
                );
                assert_eq!(
                    format!("{:?}", KeyboardUiMode(raw)),
                    row["panelName"].as_str().unwrap()
                );
                assert_eq!(
                    json!({"kind":"enum","raw":raw,"upper":ShiftState(raw).is_uppercase(),"shiftName":ShiftState(raw).to_string(),"panelName":KeyboardUiMode(raw).to_string()}),
                    row
                );
            }
            "selector" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                let mask = row["mask"].as_u64().unwrap();
                let leaves = (0..6)
                    .map(|i| {
                        TextKeyData::char(Utf16Text::from_units(vec![97 + i]), None).unwrap() as Key
                    })
                    .collect::<Vec<_>>();
                let get = |i: usize| {
                    if mask & (1 << i) != 0 {
                        Some(leaves[i].clone())
                    } else {
                        None
                    }
                };
                let s = Rc::new(ShiftStateSelector::from_parts(
                    get(0),
                    get(1),
                    get(2),
                    get(3),
                    get(4),
                    get(5),
                ));
                let e = KeyEvaluator::from_values(KeyEvaluatorValues {
                    shift_state: ShiftState(raw),
                    ..Default::default()
                });
                let key = s.compute(&e).unwrap();
                let identity = key
                    .as_ref()
                    .and_then(|k| {
                        leaves
                            .iter()
                            .position(|v| Rc::as_ptr(v).cast::<()>() == Rc::as_ptr(k).cast::<()>())
                    })
                    .map_or(-1, |i| i as i32);
                assert_eq!(
                    json!({"kind":"selector","raw":raw,"mask":mask,"retained":e.shift_state().0,"code":key.map(|v|v.data().code()),"identity":identity}),
                    row
                );
            }
            "case" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                let lower: Key = TextKeyData::char("a".into(), None).unwrap();
                let upper: Key = TextKeyData::char("A".into(), None).unwrap();
                let key = Rc::new(CaseSelector::from_parts(
                    Some(lower.clone()),
                    Some(upper.clone()),
                ))
                .compute(&KeyEvaluator::from_values(KeyEvaluatorValues {
                    shift_state: ShiftState(raw),
                    ..Default::default()
                }))
                .unwrap()
                .unwrap();
                assert_eq!(
                    json!({"kind":"case","raw":raw,"code":key.data().code(),"lower":Rc::as_ptr(&lower).cast::<()>()==Rc::as_ptr(&key).cast::<()>(),"upper":Rc::as_ptr(&upper).cast::<()>()==Rc::as_ptr(&key).cast::<()>()}),
                    row
                );
            }
            "auto" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                let code = row["code"].as_i64().unwrap() as i32;
                let label = Utf16Text::from_units(
                    serde_json::from_value::<Vec<u16>>(row["label"].clone()).unwrap(),
                );
                let s = auto_key(code, label.clone());
                let original: TextKey = s.clone();
                let key = s
                    .compute(&KeyEvaluator::from_values(KeyEvaluatorValues {
                        shift_state: ShiftState(raw),
                        ..Default::default()
                    }))
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    json!({"kind":"auto","raw":raw,"label":label.units(),"code":code,"same":Rc::ptr_eq(&original,&key),"resultCode":key.data().code(),"type":key.data().key_type().0,"result":key.data().label().as_ref().unwrap().units()}),
                    row
                );
            }
            _ => {}
        }
    }
}
#[test]
fn source_raw_shift_computed_labels_icons_and_fractional_geometry() {
    for row in rows().into_iter().filter(|r| r["kind"] == "computed") {
        let raw = row["raw"].as_i64().unwrap() as i32;
        let mode = row["mode"].as_i64().unwrap() as i32;
        let k = LayoutComputer::new(resources())
            .compute(KeyboardMode::Characters, Subtype::shared_default(), false)
            .unwrap();
        k.compute(Rc::new(KeyComputeContext::from_values(
            ferroboard_core::layouts::KeyComputeContextValues {
                shift_state: ShiftState(raw),
                mode: KeyboardMode(mode),
                ..Default::default()
            },
        )))
        .unwrap();
        k.layout(321.375, 117.25, None).unwrap();
        let keys=k.keys().map(|key| key.unwrap().unwrap()).map(|v|{let b=v.touch_bounds();json!({"Code":v.code(),"label":v.label().map(|s|s.units().to_vec()),"icon":(v.icon()).0,"bounds":[bits(b.x),bits(b.y),bits(b.width),bits(b.height)]})}).collect::<Vec<_>>();
        assert_eq!(
            json!({"kind":"computed","raw":raw,"mode":mode,"context":k.context().unwrap().shift_state().0,"keys":keys}),
            row
        );
    }
}
#[test]
fn source_raw_shift_and_panel_engine_transition_snapshots() {
    let rows = rows();
    for (kind, count) in [("shift", 17), ("panel", 12)] {
        let cases = rows
            .iter()
            .filter(|v| v["kind"] == kind)
            .collect::<Vec<_>>();
        assert_eq!(cases.len() % count, 0);
        for group in cases.chunks_exact(count) {
            let raw = group[0]["raw"].as_i64().unwrap() as i32;
            let attached = group[0]["attached"].as_bool().unwrap();
            let auto = group[0]["auto"].as_bool().unwrap_or(false);
            let e = engine(auto);
            if attached {
                attach(
                    &e,
                    if kind == "shift" {
                        CapitalizationMode::Sentences
                    } else {
                        CapitalizationMode::None
                    },
                );
            }
            if let Some(editor) = e.editor() {
                editor.toggle_selection_mode().unwrap();
            }
            let original = e.keyboard().unwrap();
            let log = observe(&e, &original);
            for row in group {
                log.borrow_mut().clear();
                let operation = row["operation"].as_str().unwrap();
                let result = attempt(|| {
                    if kind == "shift" {
                        match operation {
                            "set" | "same" | "reset" | "reset2" | "reset3" | "numeric-shift" => {
                                e.set_shift_state(ShiftState(raw))
                            }
                            "recompute" => e.recompute_keyboard(),
                            "auto" | "numeric-auto" => e.update_auto_capitalization(),
                            "down" => e.on_key_down(
                                TextKeyData::function(KeyCode::Shift, KeyType::Function, None)
                                    .as_ref(),
                            ),
                            "up" => e.on_key_up(
                                TextKeyData::function(KeyCode::Shift, KeyType::Function, None)
                                    .as_ref(),
                            ),
                            "character" => {
                                e.input_key(TextKeyData::char("x".into(), None).unwrap().as_ref())
                            }
                            "swipe" => e.execute_swipe_action(SwipeAction::Shift, 1),
                            "numeric" => e.set_mode(KeyboardMode::Numeric),
                            "characters" => e.set_mode(KeyboardMode::Characters),
                            "text" => e.input_text(&"z".into()),
                            "detach" => e.detach(),
                            _ => panic!("unknown shift operation {operation}"),
                        }
                    } else {
                        match operation {
                            "set" | "same" | "same-select" | "set-again" => {
                                e.set_ui_mode(KeyboardUiMode(raw))
                            }
                            "select" => {
                                if let Some(editor) = e.editor() {
                                    editor.toggle_selection_mode()?;
                                }
                                Ok(())
                            }
                            "suggestions" => e.update_suggestions(),
                            "text" => e.input_text(&"x".into()),
                            "shift" => e.set_shift_state(ShiftState(raw)),
                            "characters" => e.set_mode(KeyboardMode::Characters),
                            "media-key" => e.input_key(
                                TextKeyData::function(
                                    KeyCode::ImeUiModeMedia,
                                    KeyType::Function,
                                    None,
                                )
                                .as_ref(),
                            ),
                            "text-key" => e.input_key(
                                TextKeyData::function(
                                    KeyCode::ImeUiModeText,
                                    KeyType::Function,
                                    None,
                                )
                                .as_ref(),
                            ),
                            "detach" => e.detach(),
                            _ => panic!("unknown panel operation {operation}"),
                        }
                    }
                });
                let mut actual = json!({"kind":kind,"raw":raw,"attached":attached,"operation":operation,"result":result,"state":state(&e,&original),"events":*log.borrow()});
                if kind == "shift" {
                    actual["auto"] = json!(auto);
                }
                assert_eq!(
                    &actual, *row,
                    "{kind} raw{raw} attached{attached} auto{auto} {operation}"
                );
            }
            e.dispose().unwrap();
        }
    }
    for row in rows.iter().filter(|v| v["kind"] == "mixed") {
        let shift = row["shift"].as_i64().unwrap() as i32;
        let panel = row["panel"].as_i64().unwrap() as i32;
        let e = engine(false);
        attach(&e, CapitalizationMode::None);
        e.set_shift_state(ShiftState(shift)).unwrap();
        e.set_ui_mode(KeyboardUiMode(panel)).unwrap();
        e.editor().unwrap().toggle_selection_mode().unwrap();
        let original = e.keyboard().unwrap();
        let log = observe(&e, &original);
        let result = attempt(|| e.set_mode(KeyboardMode::Characters));
        assert_eq!(
            json!({"kind":"mixed","shift":shift,"panel":panel,"result":result,"state":state(&e,&original),"events":*log.borrow()}),
            *row
        );
        e.dispose().unwrap();
    }
}
#[test]
fn source_raw_enum_reentrancy_and_observer_failure_retain_post_mutation_state() {
    let rows = rows();
    for kind in ["reentrant", "failure"] {
        let cases = rows
            .iter()
            .filter(|r| r["kind"] == kind)
            .collect::<Vec<_>>();
        let count = if kind == "reentrant" { 2 } else { 3 };
        for group in cases.chunks_exact(count) {
            let which = group[0]["which"].as_str().unwrap();
            let raw = group[0]["raw"].as_i64().unwrap() as i32;
            let e = engine(false);
            if kind == "failure" {
                e.set_shift_state(ShiftState::ShiftedManual).unwrap();
                e.set_ui_mode(KeyboardUiMode::Media).unwrap();
            }
            let original = e.keyboard().unwrap();
            let log = observe(&e, &original);
            let weak = Rc::downgrade(&e);
            let once = Cell::new(false);
            let events = log.clone();
            let baseline = original.clone();
            let is_shift = which == "shift";
            let reentrant = kind == "reentrant";
            let event = if is_shift {
                e.keyboard_changed()
            } else {
                e.state_changed()
            };
            event.add(Rc::new(move |_| {
                if !once.replace(true) {
                    let e = weak.upgrade().unwrap();
                    if reentrant {
                        if is_shift {
                            e.set_shift_state(ShiftState(raw)).unwrap();
                        } else {
                            e.set_ui_mode(KeyboardUiMode(raw)).unwrap();
                        }
                    } else {
                        let snapshot = state(&e, &baseline);
                        events
                            .borrow_mut()
                            .push(json!({"kind":"throw","state":snapshot}));
                        std::panic::panic_any(ProbeFailure);
                    }
                }
            }));
            if reentrant {
                let weak = Rc::downgrade(&e);
                let events = log.clone();
                let baseline = original.clone();
                event.add(Rc::new(move |_| {
                    let snapshot = state(&weak.upgrade().unwrap(), &baseline);
                    events
                        .borrow_mut()
                        .push(json!({"kind":"last","state":snapshot}));
                }));
            }
            for row in group {
                log.borrow_mut().clear();
                let operation = row["operation"].as_str().unwrap();
                let value = if operation == "recover" {
                    0
                } else if reentrant {
                    1
                } else {
                    raw
                };
                let result = attempt(|| {
                    if is_shift {
                        e.set_shift_state(ShiftState(value))
                    } else {
                        e.set_ui_mode(KeyboardUiMode(value))
                    }
                });
                assert_eq!(
                    json!({"kind":kind,"which":which,"raw":raw,"operation":operation,"result":result,"state":state(&e,&original),"events":*log.borrow()}),
                    **row,
                    "{kind} {which} raw{raw} {operation}"
                );
            }
            e.dispose().unwrap();
        }
    }
}
