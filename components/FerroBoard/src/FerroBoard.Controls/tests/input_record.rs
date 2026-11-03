#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::input::ScreenKeyboardInput;
use ferroboard_core::editor::*;
use ferroui_controls::TextBox;
use serde_json::{Value, json};
use std::rc::Rc;
use std::sync::Arc;
fn state(a: &InputAttributes) -> Value {
    json!({"kind":a.kind().0,"action":a.enter_action().0,"cap":a.capitalization().0,"multi":a.is_multiline(),"suggest":a.allow_suggestions(),"correct":a.allow_auto_correct(),"privateField":a.is_private(),"password":a.is_password(),"variation":a.variation().0,"mode":a.initial_mode().0,"effective":a.effective_enter_action().0,"nlp":a.supports_nlp()})
}
#[test]
fn unchanged_source_apply_preserves_no_override_identity_and_allocates_for_equal_overrides() {
    headless::run(|| {
        for line in include_str!("../../../tests/fixtures/input-record.jsonl").lines() {
            let mut row: Value = serde_json::from_str(line).unwrap();
            if row["tag"] != "apply" {
                continue;
            }
            row["before"].as_object_mut().unwrap().remove("text");
            row["state"].as_object_mut().unwrap().remove("text");
            let profile = row["profile"].as_i64().unwrap();
            let mask = row["mask"].as_i64().unwrap();
            let a = Arc::new(InputAttributes::from_values(InputAttributesValues {
                kind: InputKind::Email,
                enter_action: EnterAction::Search,
                capitalization: CapitalizationMode::Words,
                allow_suggestions: profile & 1 != 0,
                allow_auto_correct: profile & 1 != 0,
                is_private: profile & 2 != 0,
                ..Default::default()
            }));
            let element = TextBox::new();
            if mask & 1 != 0 {
                ScreenKeyboardInput::set_input_kind(&element, Some(Rc::new(InputKind::Email)));
            }
            if mask & 2 != 0 {
                ScreenKeyboardInput::set_enter_action(&element, Some(Rc::new(EnterAction::Search)));
            }
            if mask & 4 != 0 {
                ScreenKeyboardInput::set_capitalization(
                    &element,
                    Some(Rc::new(CapitalizationMode::Words)),
                );
            }
            ScreenKeyboardInput::set_suggestions_enabled(&element, mask & 8 == 0);
            ScreenKeyboardInput::set_is_private(&element, mask & 16 != 0);
            let output = ScreenKeyboardInput::apply(&element, a.clone()).unwrap();
            assert_eq!(
                json!({"tag":"apply","profile":profile,"mask":mask,"before":state(&a),"state":state(&output),"same":Arc::ptr_eq(&a,&output),"equal":a==output}),
                row
            );
        }
    });
}
