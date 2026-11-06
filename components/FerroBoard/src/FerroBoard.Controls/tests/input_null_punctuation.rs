#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::input::{InputOverrideError, ScreenKeyboardInput};
use ferroboard_core::editor::*;
use ferroui_base::Visual;
use ferroui_controls::{StackPanel, TextBox};
use serde_json::{Value, json};
use std::{rc::Rc, sync::Arc};
fn state(a: Option<&InputAttributes>) -> Value {
    a.map_or(Value::Null,|a|json!({"kind":a.kind().0,"action":a.enter_action().0,"cap":a.capitalization().0,"multi":a.is_multiline(),"suggest":a.allow_suggestions(),"correct":a.allow_auto_correct(),"privateField":a.is_private()}))
}
fn result(e: Option<&Visual>, a: Option<Arc<InputAttributes>>) -> Value {
    match ScreenKeyboardInput::apply_nullable(e, a.clone()) {
        Ok(value) => {
            let same = match (&a, &value) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            };
            json!({"state":state(value.as_deref()),"same":same})
        }
        Err(InputOverrideError::NullReference) => json!({"error":"NullReferenceException"}),
        Err(e) => panic!("unexpected {e}"),
    }
}
#[test]
fn original_nullable_apply_and_disabled_parent_depth_follow_source_short_circuits() {
    headless::run(|| {
        for line in include_str!("../../../tests/fixtures/input-null-punctuation.jsonl").lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            match row["tag"].as_str().unwrap() {
                "apply" => {
                    let element = TextBox::new();
                    let mask = row["mask"].as_i64().unwrap();
                    if mask & 1 != 0 {
                        ScreenKeyboardInput::set_input_kind(
                            &element,
                            Some(Rc::new(InputKind::Email)),
                        );
                    }
                    if mask & 2 != 0 {
                        ScreenKeyboardInput::set_enter_action(
                            &element,
                            Some(Rc::new(EnterAction::Search)),
                        );
                    }
                    if mask & 4 != 0 {
                        ScreenKeyboardInput::set_capitalization(
                            &element,
                            Some(Rc::new(CapitalizationMode::Words)),
                        );
                    }
                    ScreenKeyboardInput::set_suggestions_enabled(&element, mask & 8 == 0);
                    ScreenKeyboardInput::set_is_private(&element, mask & 16 != 0);
                    let attributes =
                        (row["attributes"] == true).then(|| Arc::new(InputAttributes::default()));
                    let visual: Option<&Visual> = if row["element"] == true {
                        Some(&element)
                    } else {
                        None
                    };
                    assert_eq!(result(visual, attributes), row["result"], "{row}");
                    assert_eq!(
                        ScreenKeyboardInput::is_enabled_for_nullable(visual),
                        row["enabled"].as_bool().unwrap()
                    );
                }
                "tree" => {
                    let nodes = (0..11).map(|_| StackPanel::new()).collect::<Vec<_>>();
                    for i in 0..10 {
                        nodes[i + 1].children().add(nodes[i].to_ref());
                    }
                    let depth = row["depth"].as_u64().unwrap() as usize;
                    let property: Option<ferroui_base::BoxedValue> =
                        match row["shape"].as_str().unwrap() {
                            "typed" => Some(Rc::new(InputKind(99))),
                            "numeric" => Some(Rc::new(String::from("99"))),
                            "invalid" => Some(Rc::new(String::from("invalid"))),
                            "null" => None,
                            _ => panic!("shape"),
                        };
                    ScreenKeyboardInput::set_input_kind(&nodes[depth], property);
                    ScreenKeyboardInput::set_is_enabled(&nodes[depth], false);
                    let attributes =
                        (row["attributes"] == true).then(|| Arc::new(InputAttributes::default()));
                    assert_eq!(result(Some(&nodes[0]), attributes), row["result"], "{row}");
                    assert_eq!(
                        ScreenKeyboardInput::is_enabled_for_nullable(Some(&nodes[0])),
                        row["enabled"].as_bool().unwrap()
                    );
                }
                _ => {}
            }
        }
    });
}
