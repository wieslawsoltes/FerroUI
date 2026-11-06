use crate::sample_options::{OptionErrorKind, SampleOptions};
use ferroboard_core::{
    settings::KeyboardSettings,
    text::{OrdinalKey, Utf16Text},
};
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};
fn text(v: &Value) -> Utf16Text {
    Utf16Text::from_units(
        v.as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
fn state(s: &KeyboardSettings) -> Value {
    json!({"theme":s.theme_mode().0,"light":s.light_theme().encode_utf16().collect::<Vec<_>>(),"dark":s.dark_theme().encode_utf16().collect::<Vec<_>>(),"oneHanded":s.one_handed_mode().0,"split":s.split_keyboard(),"floating":s.floating(),"numberRow":s.number_row(),"glide":s.glide_typing(),"height":format!("{:016x}",s.key_height().to_bits()),"langs":s.subtypes().borrow().iter().map(|s|s.as_ref().map(|s|s.encode_utf16().collect::<Vec<_>>())).collect::<Vec<_>>(),"longPress":s.long_press_delay()})
}
#[test]
fn source_utf16_parser_values_apply_order_reentrancy_errors_and_numeric_bits() {
    let mut parity = 0;
    let mut gaps = 0;
    for line in include_str!("../../tests/fixtures/sample-options.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let options = Arc::new(SampleOptions::default());
        if row["kind"] == "parse" {
            for input in row["inputs"].as_array().unwrap() {
                options.parse(
                    if input.is_null() {
                        None
                    } else {
                        Some(text(input))
                    }
                    .as_ref(),
                );
            }
            let actual = Value::Array(
                options
                    .values()
                    .iter()
                    .map(|(key, value)| json!({"key":key.0.units(),"value":value.as_ref().map(Utf16Text::units)}))
                    .collect(),
            );
            assert_eq!(actual, row["values"], "input {:?}", row["inputs"]);
            parity += 1;
            continue;
        }
        for assignment in row["assignments"].as_array().unwrap() {
            options.values().insert(
                OrdinalKey(text(&assignment["key"])),
                if assignment["value"].is_null() {
                    None
                } else {
                    Some(text(&assignment["value"]))
                },
            );
        }
        let settings = KeyboardSettings::default();
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        let reentrant = row["reentrant"] == true;
        let options_callback = options.clone();
        settings.property_changed().add(Rc::new(move |name| {
            observed.borrow_mut().push(name.to_owned());
            if reentrant && name == "ThemeMode" {
                options_callback
                    .values()
                    .insert(OrdinalKey("keyheight".into()), Some("62.5".into()));
            }
        }));
        let result = options.apply(&settings);
        let has_unpaired = row["assignments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| !a["value"].is_null() && text(&a["value"]).to_utf8().is_err());
        if has_unpaired {
            assert_eq!(
                result.unwrap_err().kind,
                OptionErrorKind::UnrepresentableText
            );
            // Three SOURCE-ONLY Apply cases: Core settings store UTF-8 strings. Do not call
            // these port parity passes or silently replace source UTF-16 values.
            gaps += 1;
            continue;
        }
        let error = result.err().map(|e| match e.kind {
            OptionErrorKind::Format => "FormatException",
            OptionErrorKind::Overflow => "OverflowException",
            OptionErrorKind::UnrepresentableText => "UnrepresentableText",
        });
        assert_eq!(json!(error), row["error"], "{row}");
        assert_eq!(json!(*events.borrow()), row["events"], "{row}");
        assert_eq!(state(&settings), row["state"], "{row}");
        parity += 1;
    }
    assert_eq!(parity, 1472);
    assert_eq!(gaps, 3);
}
#[test]
fn shared_options_have_stable_process_identity_and_get_uses_original_ordinal_ignore_case() {
    let first = SampleOptions::shared();
    let second = SampleOptions::shared();
    assert!(Arc::ptr_eq(&first, &second));
    first.parse(Some(&"__ferro_shared_test=one".into()));
    assert_eq!(
        second.get(&"__FERRO_SHARED_TEST".into()),
        Some("one".into())
    );
    first
        .values()
        .remove(&OrdinalKey("__ferro_shared_test".into()));
}
