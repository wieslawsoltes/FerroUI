//! Actual source mode transition failure boundaries, retained cache and callback reentrancy.
#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
use ferroboard_core::{editor::*, engine::*, keys::*, layouts::*, settings::*};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    sync::Arc,
};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/keyboard-mode.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn error(e: EngineError) -> Value {
    match e {
        EngineError::Resource(ResourceError::KeyboardMode(e)) => {
            json!({"error":"ArgumentOutOfRangeException","parameter":e.parameter_name()})
        }
        e => panic!("unexpected mode failure: {e:?}"),
    }
}
fn attempt(f: impl FnOnce() -> EngineResult<()>) -> Value {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => json!({"value":null}),
        Ok(Err(e)) => error(e),
        Err(payload) => match payload.downcast::<EngineError>() {
            Ok(e) => error(*e),
            Err(other) => std::panic::resume_unwind(other),
        },
    }
}
fn state(e: &KeyboardEngine, original: &Rc<ComputedKeyboard>) -> Value {
    let keyboard = match e.keyboard() {
        Ok(k) => {
            json!({"value":{"same":Rc::ptr_eq(&k,original),"mode":k.mode().0,"version":k.version(),"rows":k.row_count(),"width":bits(k.width()),"height":bits(k.height())}})
        }
        Err(e) => error(e),
    };
    json!({"mode":e.mode().0,"shift":e.shift_state().0,"panel":e.ui_mode().0,"context":e.create_compute_context().unwrap().mode().0,"attached":e.target().is_some(),"selecting":e.editor().is_some_and(|v|v.is_selecting()),"suggestions":e.suggestions().len().unwrap(),"loaded":e.language_models().is_loaded(&"en".into()),"keyboard":keyboard})
}
fn engine() -> Rc<KeyboardEngine> {
    let settings = Rc::new(KeyboardSettings::new());
    settings.set_glide_typing(false);
    settings.set_auto_capitalization(false);
    settings.set_show_suggestions(false);
    settings.set_next_word_prediction(false);
    settings.set_emoji_suggestions(false);
    settings.set_clipboard_suggestion(false);
    let e = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_empty()),
        LanguageModels::new(),
    )
    .unwrap();
    e.set_clock(Rc::new(|| fixture_clock::time(639028224000000000)));
    e
}
fn bits(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}
#[test]
fn source_layout_mode_rejection_and_independent_context_fallback_geometry() {
    for row in rows() {
        match row["kind"].as_str().unwrap() {
            "compute" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                let number = row["number"].as_bool().unwrap();
                let r = Rc::new(KeyboardResources::create_empty());
                let paths = Rc::new(RefCell::new(Vec::<String>::new()));
                let observed = paths.clone();
                let manifest=LayoutJson::read_extension_manifest(r#"{"meta":{"id":"org.florisboard.layouts"},"layouts":{"characters":[{"id":"qwerty"}]}}"#).unwrap();
                r.add_extension(
                    Rc::new(manifest),
                    Rc::new(move |p| {
                        observed.borrow_mut().push(p.to_utf8().unwrap());
                        Ok(Some("[[\"a\"]]".into()))
                    }),
                )
                .unwrap();
                let result = match LayoutComputer::new(r).compute(
                    KeyboardMode(raw),
                    Subtype::shared_default(),
                    number,
                ) {
                    Ok(k) => {
                        k.compute(Rc::new(KeyComputeContext::from_values(
                            ferroboard_core::layouts::KeyComputeContextValues {
                                mode: KeyboardMode(raw),
                                ..Default::default()
                            },
                        )))
                        .unwrap();
                        json!({"value":{"mode":k.mode().0,"rows":k.row_count()}})
                    }
                    Err(e) => error(EngineError::Resource(e)),
                };
                assert_eq!(
                    json!({"kind":"compute","raw":raw,"number":number,"result":result,"paths":*paths.borrow()}),
                    row
                );
            }
            "context" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                let space = row["space"].as_i64().unwrap() as i32;
                let r = Rc::new(KeyboardResources::create_empty());
                r.add_layout(Rc::new(KeyboardLayout::new(
                    LayoutType::Numeric,
                    "e".into(),
                    "keys".into(),
                    LayoutJson::read_arrangement(
                        r#"[[{"code":32},{"code":-7},{"code":-5},{"code":-13},{"code":97}]]"#,
                    )
                    .unwrap(),
                )));
                let subtype = Arc::new(Subtype::from_values(
                    ferroboard_core::layouts::SubtypeValues {
                        layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                            ferroboard_core::layouts::SubtypeLayoutMapValues {
                                numeric: ComponentName::new("e", "keys"),
                                ..Default::default()
                            },
                        ))),
                        ..(*Subtype::shared_default()).values()
                    },
                ));
                let k = LayoutComputer::new(r)
                    .compute(KeyboardMode::Numeric, subtype, false)
                    .unwrap();
                k.compute(Rc::new(KeyComputeContext::from_values(
                    ferroboard_core::layouts::KeyComputeContextValues {
                        mode: KeyboardMode(raw),
                        space_bar_mode: SpaceBarMode(space),
                        space_bar_label: Some("label".into()),
                        ..Default::default()
                    },
                )))
                .unwrap();
                k.layout(321.375, 117.25, None).unwrap();
                assert_eq!(
                    json!({"kind":"context","raw":raw,"space":space,"keyboardMode":k.mode().0,"contextMode":k.context().unwrap().mode().0,"keys":k.keys().map(|key| key.unwrap().unwrap()).map(|v|{let b=v.touch_bounds();json!({"Code":v.code(),"label":v.label().map(|v|v.units().to_vec()),"icon":(v.icon()).0,"width":bits(v.width_factor()),"grow":bits(v.grow()),"shrink":bits(v.shrink()),"bounds":[bits(b.x),bits(b.y),bits(b.width),bits(b.height)]})}).collect::<Vec<_>>()}),
                    row
                );
            }
            _ => {}
        }
    }
}
#[test]
fn source_engine_mode_failure_noop_cache_recovery_and_synchronous_resource_errors() {
    let all = rows();
    let mut at = 0;
    let records = all
        .iter()
        .filter(|r| r["kind"] == "engine")
        .collect::<Vec<_>>();
    while at < records.len() {
        let first = records[at];
        let raw = first["raw"].as_i64().unwrap() as i32;
        let attached = first["attached"].as_bool().unwrap();
        let panel = first["panel"].as_i64().unwrap() as i32;
        let shift = first["shift"].as_i64().unwrap() as i32;
        let e = engine();
        if attached {
            e.attach(Rc::new(TextBufferTarget::new(
                "abc".into(),
                Some(Arc::new(InputAttributes::from_values(
                    ferroboard_core::editor::InputAttributesValues {
                        capitalization: CapitalizationMode::None,
                        ..Default::default()
                    },
                ))),
            )))
            .unwrap();
        }
        e.set_ui_mode(match panel {
            0 => KeyboardUiMode::Text,
            1 => KeyboardUiMode::Media,
            3 => KeyboardUiMode::Editing,
            _ => unreachable!(),
        })
        .unwrap();
        e.set_shift_state(if shift == 0 {
            ShiftState::Unshifted
        } else {
            ShiftState::CapsLock
        })
        .unwrap();
        if let Some(editor) = e.editor() {
            editor.toggle_selection_mode().unwrap();
        }
        let original = e.keyboard().unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        for (kind, event) in [
            ("keyboard", e.keyboard_changed()),
            ("state", e.state_changed()),
            ("suggestions", e.suggestions_changed()),
        ] {
            let events = events.clone();
            let original = original.clone();
            event.add(Rc::new(move |e| {
                let current = state(e, &original);
                events
                    .borrow_mut()
                    .push(json!({"kind":kind,"state":current}));
            }));
        }
        for row in &records[at..at + 11] {
            events.borrow_mut().clear();
            let op = row["operation"].as_str().unwrap();
            let result = attempt(|| match op {
                "set" | "same" | "from-panel" | "set-again" => e.set_mode(KeyboardMode(raw)),
                "shift" => e.set_shift_state(ShiftState::ShiftedManual),
                "layout" => e.update_layout(123.5, 81.25, None),
                "panel" => e.set_ui_mode(KeyboardUiMode::Media),
                "next" => e.execute_swipe_action(SwipeAction::CycleToNextKeyboardMode, 1),
                "resource" => {
                    e.resources()
                        .add_punctuation_rule(Arc::new(PunctuationRule::from_values(
                            ferroboard_core::layouts::PunctuationRuleValues {
                                extension_id: Some("probe".into()),
                                id: Some("rule".into()),
                                ..Default::default()
                            },
                        )));
                    Ok(())
                }
                "previous" => e.execute_swipe_action(SwipeAction::CycleToPreviousKeyboardMode, 1),
                "recover" => e.set_mode(KeyboardMode::Characters),
                _ => unreachable!(),
            });
            assert_eq!(
                json!({"kind":"engine","raw":raw,"attached":attached,"panel":panel,"shift":shift,"operation":op,"result":result,"state":state(&e,&original),"events":*events.borrow()}),
                **row
            );
        }
        let resources = e.resources();
        e.dispose().unwrap();
        assert_eq!(resources.changed().handler_count(), 0);
        at += 11;
    }
    assert_eq!(at, 1584);
}
#[test]
fn source_caught_reentrant_mode_failure_changes_later_observers_and_outer_state() {
    let records = rows()
        .into_iter()
        .filter(|r| r["kind"] == "reentrant")
        .collect::<Vec<_>>();
    let (pairs, remainder) = records.as_chunks::<2>();
    assert!(remainder.is_empty());
    for pair in pairs {
        let raw = pair[0]["raw"].as_i64().unwrap() as i32;
        let e = engine();
        let original = e.keyboard().unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        let nested = Rc::new(Cell::new(false));
        let log = events.clone();
        let old = original.clone();
        e.keyboard_changed().add(Rc::new(move |e| {
            let state = state(e, &old);
            log.borrow_mut().push(json!({"kind":"first","state":state}));
        }));
        let log = events.clone();
        let old = original.clone();
        e.keyboard_changed().add(Rc::new(move |e| {
            if !nested.replace(true) {
                let result = attempt(|| e.set_mode(KeyboardMode(raw)));
                let state = state(e, &old);
                log.borrow_mut()
                    .push(json!({"kind":"caught","result":result,"state":state}));
            }
        }));
        for (kind, event) in [("last", e.keyboard_changed()), ("state", e.state_changed())] {
            let log = events.clone();
            let old = original.clone();
            event.add(Rc::new(move |e| {
                let state = state(e, &old);
                log.borrow_mut().push(json!({"kind":kind,"state":state}));
            }));
        }
        for row in pair {
            events.borrow_mut().clear();
            let op = row["operation"].as_str().unwrap();
            let result = attempt(|| {
                e.set_mode(if op == "outer" {
                    KeyboardMode::Symbols
                } else {
                    KeyboardMode::Characters
                })
            });
            assert_eq!(
                json!({"kind":"reentrant","raw":raw,"operation":op,"result":result,"state":state(&e,&original),"events":*events.borrow()}),
                *row
            );
        }
        e.dispose().unwrap();
    }
}
