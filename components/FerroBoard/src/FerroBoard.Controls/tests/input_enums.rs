#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::input::*;
use ferroboard_core::{editor::*, keys::KeyboardMode, text::Utf16Text};
use ferroui_base::{BoxedValue, metadata::into_markup_value, threading::Dispatcher};
use ferroui_controls::{Border, Control, TextBox, Window};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/input-enums.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn attr(a: &InputAttributes) -> Value {
    json!({"kind":a.kind().0,"action":a.enter_action().0,"cap":a.capitalization().0,"multi":a.is_multiline(),"suggest":a.allow_suggestions(),"correct":a.allow_auto_correct(),"privateField":a.is_private(),"password":a.is_password(),"variation":a.variation().0,"mode":a.initial_mode().0,"effective":a.effective_enter_action().0,"nlp":a.supports_nlp()})
}
fn boxed(row: &Value) -> Option<BoxedValue> {
    let raw = row["raw"].as_i64().unwrap() as i32;
    match row["shape"].as_str().unwrap() {
        "string" => {
            if row["text"].is_null() {
                None
            } else {
                Some(Rc::new(Utf16Text::from_units(
                    serde_json::from_value::<Vec<u16>>(row["text"].clone()).unwrap(),
                )))
            }
        }
        "kind" => into_markup_value(InputKind(raw)),
        "action" => into_markup_value(EnterAction(raw)),
        "cap" => into_markup_value(CapitalizationMode(raw)),
        "int" => into_markup_value(raw),
        "long" => into_markup_value(i64::from(raw)),
        "bool" => into_markup_value(true),
        "other" => into_markup_value(KeyboardMode(raw)),
        _ => panic!("unknown shape"),
    }
}
#[test]
fn unchanged_source_override_parser_nominal_identity_and_utf16() {
    headless::run(|| {
        for row in rows().into_iter().filter(|v| v["tag"] == "override") {
            let field = TextBox::new();
            let value = boxed(&row);
            ScreenKeyboardInput::set_input_kind(&field, value.clone());
            ScreenKeyboardInput::set_enter_action(&field, value.clone());
            ScreenKeyboardInput::set_capitalization(&field, value.clone());
            let returned = ScreenKeyboardInput::get_input_kind(&field);
            let same = match (&value, &returned) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            };
            let state = ScreenKeyboardInput::apply(&field, InputAttributes::default()).unwrap();
            assert_eq!(
                json!({"tag":"override","shape":row["shape"],"raw":row["raw"],"text":row["text"],"same":same,"state":attr(&state)}),
                row
            );
            if let Some(text) = value
                .as_ref()
                .and_then(|v| v.downcast_ref::<Utf16Text>())
                .and_then(|v| v.to_utf8().ok())
            {
                let value = Some(Rc::new(text) as BoxedValue);
                ScreenKeyboardInput::set_input_kind(&field, value.clone());
                ScreenKeyboardInput::set_enter_action(&field, value.clone());
                ScreenKeyboardInput::set_capitalization(&field, value);
                assert_eq!(
                    attr(&ScreenKeyboardInput::apply(&field, InputAttributes::default()).unwrap()),
                    row["state"]
                );
            }
        }
    });
}
#[test]
fn source_parent_depth_precedence_flags_and_native_target_snapshot() {
    headless::run(|| {
        for row in rows().into_iter().filter(|v| v["tag"] == "tree") {
            let depth = row["depth"].as_u64().unwrap() as usize;
            let shape = row["shape"].as_str().unwrap();
            let privacy = row["privacy"].as_bool().unwrap();
            let nodes = (0..11).map(|_| Border::new()).collect::<Vec<_>>();
            for i in 1..11 {
                nodes[i].set_child(nodes[i - 1].clone());
            }
            let window = Window::new();
            window.set_content(Some(Control::boxed(nodes[10].clone())));
            window.show();
            Dispatcher::current_dispatcher().run_jobs(None);
            let value: BoxedValue = match shape {
                "typed" => Rc::new(InputKind(99)),
                "numeric" => Rc::new("99".to_string()),
                _ => Rc::new("invalid".to_string()),
            };
            ScreenKeyboardInput::set_input_kind(&nodes[depth], Some(value));
            ScreenKeyboardInput::set_input_kind(&nodes[10], Some(Rc::new(InputKind::Email)));
            ScreenKeyboardInput::set_enter_action(&nodes[depth], Some(Rc::new(EnterAction(99))));
            ScreenKeyboardInput::set_capitalization(
                &nodes[depth],
                Some(Rc::new(CapitalizationMode(99))),
            );
            ScreenKeyboardInput::set_suggestions_enabled(&nodes[depth], !privacy);
            ScreenKeyboardInput::set_is_private(&nodes[depth], privacy);
            ScreenKeyboardInput::set_is_enabled(&nodes[depth], false);
            assert_eq!(
                json!({"tag":"tree","depth":depth,"shape":shape,"privacy":privacy,"enabled":ScreenKeyboardInput::is_enabled_for(&nodes[0]),"state":attr(&ScreenKeyboardInput::apply(&nodes[0],InputAttributes::default()).unwrap())}),
                row
            );
            window.close();
        }
        let field = TextBox::new();
        ScreenKeyboardInput::set_input_kind(&field, Some(Rc::new(InputKind(i32::MIN))));
        ScreenKeyboardInput::set_enter_action(&field, Some(Rc::new(EnterAction(i32::MAX))));
        ScreenKeyboardInput::set_capitalization(&field, Some(Rc::new(CapitalizationMode(-1))));
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = seen.clone();
        let target = TextBoxInputTarget::new(
            field.clone(),
            Some(Rc::new(move |a| {
                log.borrow_mut().push(a);
                true
            })),
        )
        .unwrap();
        assert_eq!(target.attributes().kind(), InputKind(i32::MIN));
        assert_eq!(target.attributes().enter_action(), EnterAction(i32::MAX));
        assert_eq!(target.attributes().capitalization(), CapitalizationMode(-1));
        ScreenKeyboardInput::set_input_kind(&field, Some(Rc::new(InputKind::Text)));
        assert_eq!(target.attributes().kind(), InputKind(i32::MIN));
        assert!(target.perform_enter_action(EnterAction(i32::MIN)));
        assert_eq!(*seen.borrow(), [EnterAction(i32::MIN)]);
        target.detach();
    });
}
