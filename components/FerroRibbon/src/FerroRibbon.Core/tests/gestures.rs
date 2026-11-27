use ferroribbon_core::{RibbonString, commands::*};
use serde_json::Value;
use std::{
    panic::catch_unwind,
    rc::Rc,
    sync::{Mutex, MutexGuard},
};
static GATE: Mutex<()> = Mutex::new(());
struct State(
    MutexGuard<'static, ()>,
    bool,
    Option<Rc<RibbonGestureNumberFormatProvider>>,
);
impl State {
    fn lock() -> Self {
        let guard = GATE.lock().unwrap_or_else(|e| e.into_inner());
        let meta = RibbonKeyGesture::treat_meta_as_control();
        RibbonKeyGesture::set_treat_meta_as_control(true);
        Self(
            guard,
            meta,
            RibbonKeyGesture::set_number_format_provider(None),
        )
    }
}
impl Drop for State {
    fn drop(&mut self) {
        let _ = &self.0;
        RibbonKeyGesture::set_treat_meta_as_control(self.1);
        RibbonKeyGesture::set_number_format_provider(self.2.take());
    }
}
fn text(value: &Value) -> Option<RibbonString> {
    value.as_array().map(|units| {
        RibbonString::from_utf16(
            units
                .iter()
                .map(|n| n.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn source() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/gestures.json")).unwrap()
}
fn gesture(input: &str) -> RibbonKeyGesture {
    RibbonKeyGesture::parse(Some(&input.into())).unwrap()
}
#[test]
fn original_key_gesture_parsing() {
    let _state = State::lock();
    for (input, mods, key) in [
        ("Ctrl+Shift+L", 3, "L"),
        ("F5", 0, "F5"),
        ("Alt+Down", 4, "Down"),
        ("Ctrl+]", 1, "OemCloseBracket"),
        ("Ctrl+Plus", 1, "Add"),
        ("Ctrl+1", 1, "Number1"),
        ("cmd+s", 8, "S"),
    ] {
        let g = gesture(input);
        assert_eq!(g.modifiers.raw(), mods);
        assert_eq!(g.key, Some(key.into()));
    }
}
#[test]
fn original_key_gesture_matching_and_display() {
    let _state = State::lock();
    let g = gesture("Ctrl+B");
    assert!(g.matches(Some(&"B".into()), RibbonModifierKeys::CONTROL));
    assert!(g.matches(Some(&"B".into()), RibbonModifierKeys::META));
    assert!(!g.matches(
        Some(&"B".into()),
        RibbonModifierKeys::CONTROL | RibbonModifierKeys::SHIFT
    ));
    assert!(gesture("Ctrl+1").matches(Some(&"NumberPad1".into()), RibbonModifierKeys::CONTROL));
    assert_eq!(
        gesture("ctrl+shift+]").to_ribbon_string(),
        "Ctrl+Shift+]".into()
    );
    assert_eq!(
        gesture("Ctrl+Shift+L").to_display_string(true),
        "⌘⇧L".into()
    );
    for text in ["Ctrl+Shift", "A+B"] {
        assert!(!RibbonKeyGesture::try_parse(
            Some(&text.into()),
            &mut RibbonKeyGesture::default()
        ));
    }
}
#[test]
fn original_gestures_parse_edge_cases() {
    let _state = State::lock();
    for (input, key) in [
        ("Ctrl + +", "Add"),
        ("Ctrl+ +", "Add"),
        ("ctrl+pageup", "PageUp"),
        ("Alt+f4", "F4"),
        ("Ctrl+Backspace", "Back"),
    ] {
        assert_eq!(gesture(input).key, Some(key.into()));
    }
}
#[test]
fn original_malformed_gestures_never_throw() {
    let _state = State::lock();
    for text in ["Ctrl+", "+", "Ctrl+A+B", "   ", "Ctrl++Shift"] {
        assert!(
            catch_unwind(|| RibbonKeyGesture::try_parse(
                Some(&text.into()),
                &mut RibbonKeyGesture::default()
            ))
            .is_ok()
        );
    }
}
#[test]
fn reference_parse_preserves_default_out_value_error_message_and_utf16() {
    let _state = State::lock();
    for row in source()["parse"].as_array().unwrap() {
        let input = text(&row["text"]);
        let mut g = gesture("Ctrl+A");
        assert_eq!(
            RibbonKeyGesture::try_parse(input.as_ref(), &mut g),
            row["ok"].as_bool().unwrap(),
            "{row}"
        );
        assert_eq!(
            g.modifiers.raw(),
            row["modifiers"].as_i64().unwrap() as i32,
            "{row}"
        );
        assert_eq!(g.key, text(&row["key"]), "{row}");
        match RibbonKeyGesture::parse(input.as_ref()) {
            Ok(parsed) => assert_eq!(parsed, g),
            Err(error) => assert_eq!(Some(error.message), text(&row["error"]), "{row}"),
        }
    }
}
#[test]
fn reference_matching_and_display_preserve_meta_modes_unknown_flags_and_null_failures() {
    let _state = State::lock();
    let source = source();
    for row in source["matching"].as_array().unwrap() {
        RibbonKeyGesture::set_treat_meta_as_control(row["meta"].as_bool().unwrap());
        let g = RibbonKeyGesture::new(
            RibbonModifierKeys::from_raw(row["modifiers"].as_i64().unwrap() as i32),
            text(&row["key"]),
        );
        assert_eq!(
            g.matches(
                text(&row["press"]).as_ref(),
                RibbonModifierKeys::from_raw(row["pressedModifiers"].as_i64().unwrap() as i32)
            ),
            row["result"].as_bool().unwrap(),
            "{row}"
        );
    }
    for row in source["display"].as_array().unwrap() {
        RibbonKeyGesture::set_treat_meta_as_control(row["meta"].as_bool().unwrap());
        let g = RibbonKeyGesture::new(
            RibbonModifierKeys::from_raw(row["modifiers"].as_i64().unwrap() as i32),
            text(&row["key"]),
        );
        if row["error"].is_null() {
            assert_eq!(Some(g.to_ribbon_string()), text(&row["plain"]), "{row}");
            assert_eq!(Some(g.to_display_string(true)), text(&row["mac"]), "{row}");
        } else {
            assert_eq!(row["error"], "ArgumentNullException");
            assert!(catch_unwind(|| g.to_display_string(false)).is_err());
        }
    }
}
#[test]
fn current_culture_sign_provider_matches_original_integer_normalization() {
    let _state = State::lock();
    for row in source()["cultures"].as_array().unwrap() {
        let format = RibbonGestureNumberFormat {
            positive_sign: text(&row["positive"]).unwrap(),
            negative_sign: text(&row["negative"]).unwrap(),
        };
        RibbonKeyGesture::set_number_format_provider(Some(Rc::new(move || format.clone())));
        let input = text(&row["text"]).unwrap();
        let mut parsed = RibbonKeyGesture::default();
        RibbonKeyGesture::try_parse(Some(&input), &mut parsed);
        assert_eq!(parsed.key, text(&row["key"]), "{row}");
        assert_eq!(
            gesture("F1").matches(Some(&input), RibbonModifierKeys::NONE),
            row["match"].as_bool().unwrap(),
            "{row}"
        );
    }
}
#[test]
fn record_equality_is_ordinal_case_sensitive_and_preserves_string_identity() {
    let _state = State::lock();
    let key: RibbonString = "b".into();
    let g = RibbonKeyGesture::new(RibbonModifierKeys::CONTROL, Some(key.clone()));
    let copy = g.clone();
    assert_eq!(g, copy);
    assert!(copy.key.as_ref().unwrap().ptr_eq(&key));
    assert_ne!(g, gesture("Ctrl+B"));
    assert!(g.matches(Some(&"B".into()), RibbonModifierKeys::CONTROL));
    assert_eq!(RibbonKeyGesture::default().key, None);
    assert_eq!(
        RibbonModifierKeys::from_raw(-1) & !RibbonModifierKeys::META,
        RibbonModifierKeys::from_raw(-9)
    );
}

#[test]
fn all_utf16_single_unit_parses_match_unchanged_source_including_categories_and_surrogates() {
    let _state = State::lock();
    let source = source();
    let changes = source["singleUnitChanges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["unit"].as_u64().unwrap() as u16, row))
        .collect::<std::collections::HashMap<_, _>>();
    for unit in 0..=u16::MAX {
        let input = RibbonString::from_utf16(vec![unit]);
        let mut parsed = RibbonKeyGesture::default();
        let ok = RibbonKeyGesture::try_parse(Some(&input), &mut parsed);
        if let Some(row) = changes.get(&unit) {
            assert_eq!(ok, row["ok"].as_bool().unwrap(), "{unit:x}");
            assert_eq!(parsed.key, text(&row["key"]), "{unit:x}");
            assert_eq!(
                parsed.modifiers.raw(),
                row["modifiers"].as_i64().unwrap() as i32,
                "{unit:x}"
            );
        } else {
            assert!(ok, "{unit:x}");
            assert_eq!(
                parsed,
                RibbonKeyGesture::new(RibbonModifierKeys::NONE, Some(input)),
                "{unit:x}"
            );
        }
    }
}

#[test]
fn canonical_aliases_and_unmodified_display_keep_values_without_intern_identity() {
    let _state = State::lock();
    for (left, right, expected) in [
        ("Esc", "Escape", "Escape"),
        ("PgUp", "PageUp", "PageUp"),
        ("F5", "F5", "F5"),
    ] {
        let left = gesture(left);
        let right = gesture(right);
        assert_eq!(left.key, right.key);
        assert_eq!(left.key.unwrap().to_utf8().unwrap(), expected);
    }
    // Ordinary clones and an unchanged display retain their existing shared
    // payload. No cross-parse or cross-table string-intern identity is promised.
    let key: RibbonString = "Example".into();
    let g = RibbonKeyGesture::new(RibbonModifierKeys::NONE, Some(key.clone()));
    assert!(g.to_display_string(false).ptr_eq(&key));
    assert!(g.to_display_string(true).ptr_eq(&key));
}

#[test]
fn original_flag_enum_formatting_retains_combined_and_unknown_numeric_values() {
    for row in source()["flags"].as_array().unwrap() {
        let flags = RibbonModifierKeys::from_raw(row["value"].as_i64().unwrap() as i32);
        assert_eq!(flags.to_string(), row["display"].as_str().unwrap());
    }
}
