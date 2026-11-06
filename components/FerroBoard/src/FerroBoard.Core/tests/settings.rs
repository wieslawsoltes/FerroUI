use ferroboard_core::settings::*;
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::Value;
use std::{cell::RefCell, rc::Rc};
#[test]
fn unchanged_csharp_settings_corpus() {
    for line in include_str!("../../../tests/fixtures/settings.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let input = row["input"].as_str().unwrap();
        let settings = KeyboardSettings::from_json(input);
        if row.get("error").is_some() {
            assert!(settings.is_err(), "{input}");
            continue;
        }
        let settings = settings.unwrap_or_else(|e| panic!("{input}: {e}"));
        let actual: Value = serde_json::from_str(&settings.to_json().unwrap()).unwrap();
        let expected = row["output"].as_object().unwrap();
        assert_eq!(actual.as_object().unwrap().len(), expected.len());
        for (name, value) in expected {
            if value.is_number() {
                assert_eq!(actual[name].as_f64(), value.as_f64(), "{input}: {name}");
            } else {
                assert_eq!(&actual[name], value, "{input}: {name}");
            }
        }
        let destination = KeyboardSettings::new();
        let events = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = events.clone();
        destination
            .property_changed()
            .add(Rc::new(move |name| log.borrow_mut().push(name.into())));
        destination.copy_from(&settings).unwrap();
        assert_eq!(
            serde_json::to_value(&*events.borrow()).unwrap(),
            row["events"],
            "{input}"
        );
    }
}
#[test]
fn settings_roundtrip_json() {
    let settings = KeyboardSettings::new();
    settings.set_number_row(true);
    settings.set_swipe_up(SwipeAction::Undo);
    settings.set_theme_mode(ThemeMode::Dark);
    settings.set_subtypes(Some(Rc::new(RefCell::new(vec![Some(
        "de-DE/qwertz".into(),
    )]))));
    let json = settings.to_json().unwrap();
    assert!(json.contains("\"swipeUp\": \"Undo\""));
    let copy = KeyboardSettings::from_json(&json).unwrap();
    assert!(copy.number_row());
    assert_eq!(copy.swipe_up(), SwipeAction::Undo);
    assert_eq!(*copy.subtypes().borrow(), [Some("de-DE/qwertz".into())]);
}
#[test]
fn settings_raise_property_changed() {
    let settings = KeyboardSettings::new();
    let changed = Rc::new(RefCell::new(vec![]));
    let log = changed.clone();
    settings
        .property_changed()
        .add(Rc::new(move |n| log.borrow_mut().push(n.to_string())));
    settings.set_glide_typing(false);
    settings.set_glide_typing(false);
    assert_eq!(*changed.borrow(), ["GlideTyping"]);
}
#[test]
fn list_identity_deep_clone_and_reentrant_notifications() {
    let settings = Rc::new(KeyboardSettings::new());
    let events = Rc::new(RefCell::new(vec![]));
    let log = events.clone();
    let weak = Rc::downgrade(&settings);
    settings.property_changed().add(Rc::new(move |n| {
        log.borrow_mut().push(n.to_string());
        if n == "NumberRow" {
            weak.upgrade().unwrap().set_hinted_symbols(false);
        }
    }));
    let original = settings.subtypes();
    original.borrow_mut().push(None); // Mutation does not call the property setter.
    settings.set_subtypes(Some(original.clone()));
    assert!(events.borrow().is_empty());
    settings.set_subtypes(Some(Rc::new(RefCell::new(original.borrow().clone()))));
    settings.set_number_row(true);
    assert_eq!(*events.borrow(), ["Subtypes", "NumberRow", "HintedSymbols"]);
    let copy = settings.deep_clone().unwrap();
    assert!(!Rc::ptr_eq(&settings.subtypes(), &copy.subtypes()));
    assert_eq!(*settings.subtypes().borrow(), *copy.subtypes().borrow());
}
#[test]
fn floating_point_equality_and_serialization_failure_are_preserved() {
    let s = KeyboardSettings::new();
    let events = Rc::new(RefCell::new(vec![]));
    let log = events.clone();
    s.property_changed()
        .add(Rc::new(move |n| log.borrow_mut().push(n.to_string())));
    s.set_key_height(f64::NAN);
    s.set_key_height(f64::NAN);
    assert_eq!(*events.borrow(), ["KeyHeight"]);
    assert!(s.key_height().is_nan());
    assert!(s.to_json().is_err());
    let destination = KeyboardSettings::new();
    assert!(destination.copy_from(&s).is_err());
    assert_eq!(destination.key_height(), 54.0);
    s.set_key_spacing_horizontal(-0.0);
    s.set_key_spacing_horizontal(0.0);
    assert!(s.key_spacing_horizontal().is_sign_negative());
    s.set_key_height(f64::INFINITY);
    assert_eq!(s.key_height(), 160.0);
}
