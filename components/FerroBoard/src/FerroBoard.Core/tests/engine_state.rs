use ferroboard_core::{engine::*, keys::*, layouts::KeyboardResources, settings::KeyboardSettings};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/engine-state.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn dimension(n: f64) -> Value {
    if n.fract() == 0.0 {
        json!(n as i64)
    } else {
        json!(n)
    }
}
fn state(e: &KeyboardEngine) -> Value {
    let k = e.keyboard().unwrap();
    json!({"mode":e.mode().0,"shift":e.shift_state().0,"panel":e.ui_mode().0,"active":e.active_subtype().id().to_utf8().unwrap(),"culture":e.culture().name(),"subtypes":e.subtypes().borrow().iter().map(|s|s.id().to_utf8().unwrap()).collect::<Vec<_>>(),"version":k.version(),"rows":k.row_count(),"keys":k.keys().collect::<Result<Vec<_>, _>>().unwrap().len(),"label":e.create_compute_context().unwrap().space_bar_label().unwrap().to_utf8().unwrap(),"width":dimension(k.width()),"height":dimension(k.height())})
}
#[test]
fn constructible_engine_layout_modes_subtypes_and_settings_match_source() {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_glide_typing(false);
    let resources = Rc::new(KeyboardResources::create_default().unwrap());
    let e =
        KeyboardEngine::new_explicit(settings.clone(), resources, LanguageModels::new()).unwrap();
    let log = Rc::new(RefCell::new(Vec::new()));
    let events = log.clone();
    e.keyboard_changed().add(Rc::new(move |e| {
        events
            .borrow_mut()
            .push(json!({"kind":"keyboard","state":state(e)}))
    }));
    let events = log.clone();
    e.state_changed().add(Rc::new(move |e| {
        events
            .borrow_mut()
            .push(json!({"kind":"state","state":state(e)}))
    }));
    for row in rows() {
        if row["kind"] == "initial" {
            assert_eq!(state(&e), row["state"]);
            continue;
        }
        if row["kind"] != "op" {
            continue;
        }
        log.borrow_mut().clear();
        let op = row["op"].as_array().unwrap();
        match op[0].as_str().unwrap() {
            "layout" => e
                .update_layout(op[1].as_f64().unwrap(), op[2].as_f64().unwrap(), None)
                .unwrap(),
            "mode" => e
                .set_mode(match op[1].as_i64().unwrap() {
                    0 => KeyboardMode::Characters,
                    1 => KeyboardMode::Symbols,
                    2 => KeyboardMode::Symbols2,
                    _ => unreachable!(),
                })
                .unwrap(),
            "panel" => e.set_ui_mode(KeyboardUiMode::Media).unwrap(),
            "shift" => e
                .set_shift_state(if op[1] == 3 {
                    ShiftState::CapsLock
                } else {
                    ShiftState::ShiftedManual
                })
                .unwrap(),
            "number" => settings.set_number_row(op[1].as_bool().unwrap()),
            "hint" => settings.set_hinted_symbols(op[1].as_bool().unwrap()),
            "active" => e
                .set_active_subtype(e.find_subtype(&op[1].as_str().unwrap().into()).unwrap())
                .unwrap(),
            "next" => e.next_subtype().unwrap(),
            "previous" => e.previous_subtype().unwrap(),
            "recompute" => e.recompute_keyboard().unwrap(),
            _ => unreachable!(),
        };
        assert_eq!(state(&e), row["state"], "{row}");
        assert_eq!(json!(*log.borrow()), row["events"], "{row}");
    }
    e.dispose().unwrap();
}
#[test]
fn engine_keyboard_cache_identity_and_explicit_disposal() {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_glide_typing(false);
    let resources = Rc::new(KeyboardResources::create_default().unwrap());
    let count = resources.changed().handler_count();
    let e =
        KeyboardEngine::new_explicit(settings.clone(), resources.clone(), LanguageModels::new())
            .unwrap();
    let k = e.keyboard().unwrap();
    e.set_mode(KeyboardMode::Symbols).unwrap();
    e.set_mode(KeyboardMode::Characters).unwrap();
    assert!(Rc::ptr_eq(&k, &e.keyboard().unwrap()));
    let weak = Rc::downgrade(&e);
    assert_eq!(resources.changed().handler_count(), count + 1);
    e.dispose().unwrap();
    e.dispose().unwrap();
    assert_eq!(resources.changed().handler_count(), count);
    let version = k.version();
    settings.set_number_row(true);
    assert_eq!(k.version(), version);
    drop(e);
    assert!(weak.upgrade().is_none());
}

#[test]
fn engine_default_glide_words_use_real_dictionary_and_layout() {
    let settings = Rc::new(KeyboardSettings::default());
    let models = LanguageModels::new();
    let e = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        models.clone(),
    )
    .unwrap();
    let rows = rows();
    let before = rows.iter().find(|r| r["kind"] == "default-glide").unwrap();
    assert_eq!(
        json!({"kind":"default-glide","loaded":models.is_loaded(&"en".into()),"readyBefore":e.glide_classifier().is_ready(),"users":models.user_dictionaries().len()}),
        *before
    );
    e.update_layout(800.0, 240.0, None).unwrap();
    let after = rows
        .iter()
        .find(|r| r["kind"] == "default-glide-layout")
        .unwrap();
    assert_eq!(
        json!({"kind":"default-glide-layout","ready":e.glide_classifier().is_ready()}),
        *after
    );
    e.dispose().unwrap();
}
