use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::{EngineError, KeyboardEngine, LanguageModels},
    layouts::{KeyboardResources, Subtype, SubtypeValues},
    settings::KeyboardSettings,
    text::{TextUtils, Utf16Text},
};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};

fn engine() -> Rc<KeyboardEngine> {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_glide_typing(false);
    settings.set_show_suggestions(false);
    settings.set_emoji_suggestions(false);
    settings.set_clipboard_suggestion(false);
    settings.set_next_word_prediction(false);
    settings.set_learn_words(false);
    settings.set_auto_capitalization(false);
    settings.set_auto_space_punctuation(false);
    KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap()
}
fn units(value: &Value) -> Option<Utf16Text> {
    value.as_array().map(|items| {
        Utf16Text::from_units(
            items
                .iter()
                .map(|x| x.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn run(row: &Value) -> Value {
    let engine = engine();
    let target = Rc::new(TextBufferTarget::new("".into(), None));
    engine.attach(target.clone()).unwrap();
    let operation = row["operation"].as_str().unwrap();
    let subtype = Arc::new(Subtype::from_values(SubtypeValues {
        language_tag: units(&row["tag"]),
        display_name: (operation == "override").then(|| "Custom language".into()),
        ..Subtype::shared_default().values()
    }));
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    engine
        .keyboard_changed()
        .add(Rc::new(move |_| log.borrow_mut().push("keyboard")));
    let log = events.clone();
    engine
        .state_changed()
        .add(Rc::new(move |_| log.borrow_mut().push("state")));
    let result = match operation {
        "activate" => engine.set_active_subtype(subtype).map(|()| None),
        "set-subtypes" => engine.set_subtypes([subtype]).map(|()| None),
        "display" | "override" => KeyboardEngine::get_subtype_display_name(&subtype).map(Some),
        _ => unreachable!(),
    };
    let (error, display) = match result {
        Ok(display) => (None, display.map(|x| x.to_utf8().unwrap())),
        Err(EngineError::ArgumentNull("name")) => (Some("ArgumentNullException".to_owned()), None),
        Err(error) => (Some(format!("{error:?}")), None),
    };
    let transition_events = events.borrow().clone();
    if error.is_none() && matches!(operation, "activate" | "set-subtypes") {
        engine.type_text(&"i".into()).unwrap();
    }
    json!({
        "tag": row["tag"], "operation": operation, "error": error, "display": display,
        "upper": TextUtils::capitalize(&"i".into(), engine.culture()).to_utf8().unwrap(),
        "culture": engine.culture().name(),
        "active": engine.active_subtype().language_tag().map(|x| x.units().to_vec()),
        "text": target.text().to_utf8().unwrap(), "transitionEvents": transition_events
    })
}
fn check_operations(operations: &[&str]) {
    let mut checked = 0;
    for line in include_str!("../../../tests/fixtures/subtype-fallback.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        // Native locale identifiers require valid Unicode. The source-only malformed
        // UTF16 cases are retained in the fixture and tested as an explicit boundary below.
        if row["tag"] == json!([55296]) || !operations.contains(&row["operation"].as_str().unwrap())
        {
            continue;
        }
        assert_eq!(run(&row), row);
        checked += 1;
    }
    assert_eq!(checked, 16);
}
#[test]
fn subtype_activation_fallback_and_event_order_match_unchanged_source() {
    check_operations(&["activate", "set-subtypes"]);
}
#[test]
fn subtype_display_fallback_and_override_match_unchanged_source() {
    check_operations(&["display", "override"]);
}
#[test]
fn malformed_utf16_locale_remains_an_explicit_native_encoding_error() {
    let engine = engine();
    let subtype = Arc::new(Subtype::from_values(SubtypeValues {
        language_tag: Some(Utf16Text::from_units(vec![0xd800])),
        ..Subtype::shared_default().values()
    }));
    assert!(matches!(
        engine.set_active_subtype(subtype.clone()),
        Err(EngineError::UnrepresentedText)
    ));
    assert!(matches!(
        KeyboardEngine::get_subtype_display_name(&subtype),
        Err(EngineError::UnrepresentedText)
    ));
    let explicit_name = Subtype::from_values(SubtypeValues {
        display_name: Some("Custom language".into()),
        ..subtype.values()
    });
    assert_eq!(
        KeyboardEngine::get_subtype_display_name(&explicit_name).unwrap(),
        Utf16Text::from("Custom language")
    );
}
