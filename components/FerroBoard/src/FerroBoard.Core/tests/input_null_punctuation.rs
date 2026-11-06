//! Actual source null ordering, mutable ordinary-class identity and UTF-16 punctuation rules.
use ferroboard_core::{
    editor::*,
    engine::{EngineError, KeyboardEngine, LanguageModels},
    layouts::*,
    settings::KeyboardSettings,
    text::{Suggestion, SuggestionKind, Utf16Text},
};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc, sync::Arc};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/input-null-punctuation.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn text(v: &Value) -> Utf16Text {
    Utf16Text::from_units(
        v.as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}

fn set(r: &PunctuationRule, field: usize, value: Option<Utf16Text>) {
    match field {
        0 => r.set_extension_id(value),
        1 => r.set_id(value),
        2 => r.set_label(value),
        3 => r.set_symbols_preceding_auto_space(value),
        4 => r.set_symbols_following_auto_space(value),
        5 => r.set_symbols_preceding_phantom_space(value),
        6 => r.set_symbols_following_phantom_space(value),
        7 => r.set_symbols_terminating_sentence(value),
        _ => panic!("field"),
    }
}
fn state(r: &PunctuationRule) -> Value {
    json!([
        r.extension_id().map(|v| v.units().to_vec()),
        r.id().map(|v| v.units().to_vec()),
        r.label().map(|v| v.units().to_vec()),
        r.symbols_preceding_auto_space().map(|v| v.units().to_vec()),
        r.symbols_following_auto_space().map(|v| v.units().to_vec()),
        r.symbols_preceding_phantom_space()
            .map(|v| v.units().to_vec()),
        r.symbols_following_phantom_space()
            .map(|v| v.units().to_vec()),
        r.symbols_terminating_sentence().map(|v| v.units().to_vec())
    ])
}
fn values() -> Vec<Option<Utf16Text>> {
    vec![
        None,
        Some("".into()),
        Some("custom".into()),
        Some(" \t\r\n".into()),
        Some(".?!".into()),
        Some("。！？".into()),
        Some("😀".into()),
        Some(Utf16Text::from_units(vec![0xd800, 0x78, 0xdfff])),
    ]
}
#[test]
fn original_class_defaults_nullable_init_registry_key_snapshots_and_notifications() {
    for row in rows() {
        match row["tag"].as_str().unwrap() {
            "default" => {
                // One deliberate namespace migration; keep the frozen C# row byte-identical.
                assert_eq!(row["text"], "ScreenKeyboard.Layouts.PunctuationRule");
                let mut expected = row.clone();
                expected["text"] = json!("FerroBoard.Layouts.PunctuationRule");
                let first = PunctuationRule::shared_default();
                assert_eq!(
                    json!({"tag":"default","state":state(&PunctuationRule::default()),"same":Arc::ptr_eq(&first,&PunctuationRule::shared_default()),"equal":PunctuationRule::default()==PunctuationRule::default(),"text":PunctuationRule::default().to_string()}),
                    expected
                );
            }
            "mutation" => {
                let r = Arc::new(PunctuationRule::from_values(PunctuationRuleValues {
                    extension_id: Some("custom".into()),
                    id: Some("id".into()),
                    ..Default::default()
                }));
                let alias = r.clone();
                let resources = KeyboardResources::create_empty();
                let count = Rc::new(Cell::new(0));
                let c = count.clone();
                resources
                    .changed()
                    .add(Rc::new(move |_| c.set(c.get() + 1)));
                resources.add_punctuation_rule(r.clone());
                let field = row["field"].as_u64().unwrap() as usize;
                let vi = row["vi"].as_u64().unwrap() as usize;
                set(&r, field, values()[vi].clone());
                let key = ComponentName::new_nullable(r.extension_id(), r.id());
                let old = ComponentName::new("custom", "id");
                let old_same = Arc::ptr_eq(&r, &resources.get_punctuation_rule(&old));
                let new_same = Arc::ptr_eq(&r, &resources.get_punctuation_rule(&key));
                let before = count.get();
                resources.add_punctuation_rule(r.clone());
                assert_eq!(
                    json!({"tag":"mutation","field":field,"vi":vi,"state":state(&r),"aliasSame":Arc::ptr_eq(&r,&alias),"oldSame":old_same,"newSame":new_same,"before":before,"after":count.get(),"oldAfterSame":Arc::ptr_eq(&r,&resources.get_punctuation_rule(&old)),"newAfterSame":Arc::ptr_eq(&r,&resources.get_punctuation_rule(&key))}),
                    row
                );
            }
            _ => {}
        }
    }
}
#[test]
fn original_editor_null_rule_and_null_utf16_terminators_preserve_short_circuit_order() {
    for row in rows().into_iter().filter(|v| v["tag"] == "capitalize") {
        let target = Rc::new(TextBufferTarget::new(text(&row["text"]), None));
        if row["selected"] == true {
            target.select(0, 1);
        }
        let editor = EditorController::new(target.clone());
        let vi = row["vi"].as_i64().unwrap();
        let rule = if vi < 0 {
            None
        } else {
            Some(PunctuationRule::from_values(PunctuationRuleValues {
                symbols_terminating_sentence: values()[vi as usize].clone(),
                ..Default::default()
            }))
        };
        let result = match editor.should_auto_capitalize_nullable(
            CapitalizationMode(row["mode"].as_i64().unwrap() as i32),
            rule.as_ref(),
        ) {
            Ok(v) => json!({"value":v}),
            Err(EditorError::NullReference) => json!({"error":"NullReferenceException"}),
            Err(e) => panic!("unexpected {e}"),
        };
        assert_eq!(
            target.selection_start(),
            row["start"].as_i64().unwrap() as i32
        );
        assert_eq!(
            target.selection_length(),
            row["length"].as_i64().unwrap() as i32
        );
        assert_eq!(result, row["result"], "{row}");
    }
}
#[test]
fn original_engine_null_auto_space_rule_failure_and_partial_text_are_preserved() {
    // Reuse only the explicitly supplied language registry across source observations.
    let models = LanguageModels::new();
    for row in rows().into_iter().filter(|v| v["tag"] == "engine") {
        let resources = Rc::new(KeyboardResources::create_empty());
        let rule = Arc::new(PunctuationRule::from_values(PunctuationRuleValues {
            extension_id: Some(Subtype::CORE_LOCALIZATION.into()),
            symbols_preceding_auto_space: values()[row["vi"].as_u64().unwrap() as usize].clone(),
            ..Default::default()
        }));
        resources.add_punctuation_rule(rule.clone());
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_glide_typing(false);
        settings.set_auto_capitalization(false);
        settings.set_show_suggestions(false);
        settings.set_next_word_prediction(false);
        settings.set_emoji_suggestions(false);
        settings.set_clipboard_suggestion(false);
        settings.set_auto_correct(false);
        let engine =
            KeyboardEngine::new(Some(settings), Some(resources), Some(models.clone())).unwrap();
        let target = Rc::new(TextBufferTarget::new("a".into(), None));
        engine.attach(target.clone()).unwrap();
        if row["commit"] == true {
            let suggestion = Suggestion::new("word".into(), SuggestionKind::Completion);
            suggestion.set_confidence(1.0);
            engine.commit_suggestion(&suggestion).unwrap();
        }
        let result = match engine.type_text(&text(&row["input"])) {
            Ok(()) => json!({"ok":true}),
            Err(EngineError::NullReference) => json!({"error":"NullReferenceException"}),
            Err(e) => panic!("unexpected {e}"),
        };
        assert_eq!(result, row["result"], "{row}");
        assert_eq!(target.text().units(), text(&row["text"]).units(), "{row}");
        assert_eq!(
            target.selection_start(),
            row["start"].as_i64().unwrap() as i32
        );
        assert_eq!(
            target.selection_length(),
            row["length"].as_i64().unwrap() as i32
        );
        rule.set_symbols_preceding_auto_space(Some(".!?".into()));
        engine.type_text(&"!".into()).unwrap();
        assert_eq!(row["recovery"]["result"], json!({"ok":true}));
        assert_eq!(
            target.text().units(),
            text(&row["recovery"]["text"]).units(),
            "{row}"
        );
        assert_eq!(
            target.selection_start(),
            row["recovery"]["start"].as_i64().unwrap() as i32
        );
        assert_eq!(
            target.selection_length(),
            row["recovery"]["length"].as_i64().unwrap() as i32
        );
        engine.dispose().unwrap();
    }
}
struct Restore(Arc<PunctuationRule>, PunctuationRuleValues);
impl Drop for Restore {
    fn drop(&mut self) {
        let v = &self.1;
        for (i, value) in [
            v.extension_id.clone(),
            v.id.clone(),
            v.label.clone(),
            v.symbols_preceding_auto_space.clone(),
            v.symbols_following_auto_space.clone(),
            v.symbols_preceding_phantom_space.clone(),
            v.symbols_following_phantom_space.clone(),
            v.symbols_terminating_sentence.clone(),
        ]
        .into_iter()
        .enumerate()
        {
            set(&self.0, i, value);
        }
    }
}
#[test]
fn original_shared_rule_runtime_init_is_visible_to_worker_with_same_identity() {
    let expected = rows().into_iter().find(|v| v["tag"] == "shared").unwrap();
    let r = PunctuationRule::shared_default();
    let _restore = Restore(r.clone(), r.values());
    r.set_symbols_terminating_sentence(None);
    let worker = std::thread::spawn(PunctuationRule::shared_default)
        .join()
        .unwrap();
    assert_eq!(
        json!({"tag":"shared","workerSame":Arc::ptr_eq(&r,&worker),"state":state(&worker)}),
        expected
    );
}
