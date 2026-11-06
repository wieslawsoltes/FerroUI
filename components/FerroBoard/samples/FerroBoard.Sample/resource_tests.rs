use crate::custom_layouts;
use ferroboard_core::{
    engine::{KeyboardEngine, LanguageModels},
    keys::ShiftState,
    layouts::{ComponentName, KeyComputeContext, KeyboardResources, LayoutType},
    settings::KeyboardSettings,
};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};
fn value(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}
fn text(s: &ferroboard_core::text::Utf16Text) -> String {
    s.to_utf8().unwrap()
}
fn snapshot(resources: &KeyboardResources) -> Value {
    let layout = resources
        .get_layout(
            LayoutType::Characters,
            &ComponentName::new("sample", "alphabetical"),
        )
        .unwrap();
    let layout=layout.map(|layout|json!({"type":layout.layout_type().0,"extension":layout.extension_id().as_ref().map(text),"id":layout.id().as_ref().map(text),"label":layout.label().as_ref().map(text),"direction":layout.direction().0,"authors":layout.authors().as_ref().unwrap().snapshot().iter().map(|s|s.as_ref().map(text)).collect::<Vec<_>>(),"rows":layout.arrangement().as_ref().unwrap().rows.as_ref().unwrap().iter().unwrap().map(|r|r.unwrap().unwrap().iter().unwrap().map(|k|k.unwrap().unwrap().compute(&KeyComputeContext::default()).unwrap().unwrap().data().code()).collect::<Vec<_>>()).collect::<Vec<_>>()}));
    let presets=resources.subtype_presets().into_iter().filter(|s|s.layouts().unwrap().characters().extension_id==Some("sample".into())).map(|s| {
        let names=[("Characters",LayoutType::Characters),("Symbols",LayoutType::Symbols),("Symbols2",LayoutType::Symbols2),("Numeric",LayoutType::Numeric),("NumericAdvanced",LayoutType::NumericAdvanced),("NumericRow",LayoutType::NumericRow),("Phone",LayoutType::Phone),("Phone2",LayoutType::Phone2)];
        let layouts=names.into_iter().map(|(n,t)|(n.to_string(),json!(text(&s.layouts().unwrap().get(t).unwrap().to_text())))).collect::<serde_json::Map<_,_>>();
        json!({"id":text(&s.id()),"language":text(&s.language_tag().unwrap()),"display":s.display_name().as_ref().map(text),"composer":text(&s.composer().to_text()),"currency":text(&s.currency_set().to_text()),"popup":text(&s.popup_mapping().to_text()),"punctuation":text(&s.punctuation_rule().to_text()),"layouts":layouts})
    }).collect::<Vec<_>>();
    json!({"layout":layout,"presets":presets})
}
#[test]
fn unchanged_custom_registration_events_defaults_replacement_and_computed_geometry() {
    let rows = include_str!("../../tests/fixtures/sample-resources.jsonl")
        .lines()
        .map(value)
        .collect::<Vec<_>>();
    let resources = Rc::new(KeyboardResources::create_default().unwrap());
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let token = resources
        .changed()
        .add(Rc::new(move |r| observed.borrow_mut().push(snapshot(r))));
    let mut old_layout = None;
    let mut old_subtype = None;
    for row in rows.iter().filter(|r| r["kind"] == "register") {
        events.borrow_mut().clear();
        custom_layouts::register(&resources).unwrap();
        assert_eq!(json!(*events.borrow()), row["events"]);
        assert_eq!(snapshot(&resources), row["snapshot"]);
        let layout = resources
            .get_layout(
                LayoutType::Characters,
                &ComponentName::new("sample", "alphabetical"),
            )
            .unwrap()
            .unwrap();
        let subtype = resources
            .subtype_presets()
            .into_iter()
            .find(|s| s.id() == "en-US/alphabetical".into())
            .unwrap();
        if let Some(old) = old_layout {
            assert!(!Rc::ptr_eq(&layout, &old));
        }
        if let Some(old) = old_subtype {
            assert!(!Arc::ptr_eq(&subtype, &old));
        }
        old_layout = Some(layout);
        old_subtype = Some(subtype);
    }
    resources.changed().remove(token);
    for row in rows.iter().filter(|r| r["kind"] == "compute") {
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_auto_capitalization(false);
        settings.set_glide_typing(false);
        settings.set_number_row(row["number"].as_bool().unwrap());
        settings.set_subtypes(Some(Rc::new(RefCell::new(vec![Some(
            "en-US/alphabetical".into(),
        )]))));
        let engine =
            KeyboardEngine::new_explicit(settings, resources.clone(), LanguageModels::new())
                .unwrap();
        let shift = match row["shift"].as_i64().unwrap() {
            0 => ShiftState::Unshifted,
            1 => ShiftState::ShiftedManual,
            3 => ShiftState::CapsLock,
            v => panic!("source shift {v}"),
        };
        engine.set_shift_state(shift).unwrap();
        engine.update_layout(768.25, 300.5, None).unwrap();
        let keyboard = engine.keyboard().unwrap();
        let rect = |b: ferroboard_core::layouts::KeyRect| {
            [b.x, b.y, b.width, b.height].map(|n| format!("{:016x}", n.to_bits()))
        };
        let actual = json!({"kind":"compute","shift":row["shift"],"number":row["number"],"active":text(&engine.active_subtype().id()),"rows":keyboard.row_count(),"keys":keyboard.keys().map(|key| key.unwrap().unwrap()).map(|k|json!({"code":k.code(),"label":k.label().as_ref().map(text),"visible":rect(k.visible_bounds()),"touch":rect(k.touch_bounds())})).collect::<Vec<_>>()});
        assert_eq!(actual, *row);
        engine.dispose().unwrap();
    }
}
