#[path = "../../../tests/support/key_null_contract.rs"]
mod contract;
#[test]
fn original_nullable_key_labels_outputs_copy_compute_display_and_json() {
    assert_eq!(
        contract::verify::<contract::Core>(include_str!("../../../tests/fixtures/key-null.jsonl")),
        (6336, 1152, 0)
    );
}

use ferroboard_core::{
    editor::*, engine::*, layouts::*, settings::KeyboardSettings, text::Utf16Text,
};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
struct OutputKey {
    base: TextKeyData,
    value: Option<Utf16Text>,
    trace: Rc<RefCell<Vec<String>>>,
}
impl AbstractKeyData for OutputKey {
    fn compute_nullable(
        self: Rc<Self>,
        _: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        Ok(Some(self))
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        self.trace.borrow_mut().push("output".into());
        Ok(self.value.clone())
    }
}
impl ITextKeyData for OutputKey {
    fn data(&self) -> &TextKeyData {
        &self.base
    }
    fn produces_text(&self) -> Result<bool, KeyDataError> {
        Ok(true)
    }
}
#[test]
fn original_nullable_virtual_output_respects_editor_short_circuit_and_reentrant_feedback_detach() {
    let mut count = 0;
    for line in include_str!("../../../tests/fixtures/key-null-engine.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_glide_typing(false);
        settings.set_auto_capitalization(false);
        settings.set_show_suggestions(false);
        settings.set_auto_correct(false);
        settings.set_next_word_prediction(false);
        settings.set_emoji_suggestions(false);
        settings.set_clipboard_suggestion(false);
        settings.set_learn_words(false);
        let engine = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_empty()),
            LanguageModels::new(),
        )
        .unwrap();
        let target = Rc::new(TextBufferTarget::new("Q".into(), None));
        let attached = row["attached"].as_bool().unwrap();
        if attached {
            engine.attach(target.clone()).unwrap();
        }
        let trace = Rc::new(RefCell::new(Vec::new()));
        let detach = row["detach"].as_bool().unwrap();
        let weak = Rc::downgrade(&engine);
        let log = trace.clone();
        engine.feedback_requested().add(Rc::new(move |kind| {
            log.borrow_mut().push(format!("feedback:{}", kind.0));
            if detach {
                weak.upgrade().unwrap().detach().unwrap();
                log.borrow_mut().push("detach".into());
            }
        }));
        let value: Option<Vec<u16>> = serde_json::from_value(row["value"].clone()).unwrap();
        let key = OutputKey {
            base: TextKeyData::default(),
            value: value.map(Utf16Text::from_units),
            trace: trace.clone(),
        };
        let result = if row["method"] == "up" {
            engine.on_key_up(&key)
        } else {
            engine.on_key_repeat(&key)
        };
        let error = match result {
            Ok(()) => None,
            Err(EngineError::Key(KeyDataError::NullReference)) => Some("NullReferenceException"),
            Err(e) => panic!("unexpected {e:?}"),
        };
        let actual = json!({"tag":"engine","attached":attached,"method":row["method"],"value":row["value"],"detach":detach,"error":error,"trace":*trace.borrow(),"text":target.text().units(),"start":target.selection_start(),"length":target.selection_length(),"hasTarget":engine.target().is_some(),"shift":engine.shift_state().0});
        assert_eq!(actual, row, "{row}");
        engine.dispose().unwrap();
        count += 1;
    }
    assert_eq!(count, 24);
}
