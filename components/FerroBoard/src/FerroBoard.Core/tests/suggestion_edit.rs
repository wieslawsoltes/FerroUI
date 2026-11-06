#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
use ferroboard_core::{
    editor::*, engine::*, layouts::KeyboardResources, settings::KeyboardSettings, text::*,
};
use ferroui_system::events::Event;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};
fn text(v: &Value) -> Utf16Text {
    Utf16Text::from_units(
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
struct Target {
    buffer: Rc<TextBufferTarget>,
    accepts_null: bool,
    mutation: String,
    candidate: Rc<Suggestion>,
    log: Rc<RefCell<Vec<Value>>>,
}
impl ITextInputTarget for Target {
    fn attributes(&self) -> Arc<InputAttributes> {
        self.buffer.attributes()
    }
    fn text(&self) -> Utf16Text {
        self.buffer.text()
    }
    fn selection_start(&self) -> i32 {
        self.buffer.selection_start()
    }
    fn selection_length(&self) -> i32 {
        self.buffer.selection_length()
    }
    fn replace(&self, start: i32, length: i32, value: Option<&Utf16Text>) -> TargetReplaceResult {
        self.log.borrow_mut().push(json!({"tag":"replace","start":start,"length":length,"text":value.map(Utf16Text::units)}));
        // This reference custom target explicitly accepts null; the default source buffer rejects it.
        let empty = Utf16Text::default();
        let value = if self.accepts_null {
            Some(value.unwrap_or(&empty))
        } else {
            value
        };
        self.buffer.replace(start, length, value)?;
        match self.mutation.as_str() {
            "replace-text" => self.candidate.set_text(Some("after".into())),
            "replace-null" => self.candidate.set_text(None),
            "replace-kind" => self.candidate.set_kind(SuggestionKind::Glide),
            _ => {}
        }
        Ok(())
    }
    fn select(&self, start: i32, length: i32) {
        self.buffer.select(start, length)
    }
    fn perform_enter_action(&self, a: EnterAction) -> bool {
        self.buffer.perform_enter_action(a)
    }
    fn changed(&self) -> &Event<dyn ITextInputTarget> {
        self.buffer.changed()
    }
}
#[test]
fn original_nullable_candidate_edits_reentrant_reads_target_errors_and_partial_history() {
    let mut count = 0;
    for line in include_str!("../../../tests/fixtures/suggestion-edit.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let kind = row["kind"].as_i64().unwrap() as i32;
        let initial = text(&row["initial"]);
        let accepts_null = row["acceptsNull"].as_bool().unwrap();
        let mutation = row["mutation"].as_str().unwrap();
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_auto_capitalization(false);
        settings.set_auto_correct(false);
        settings.set_show_suggestions(false);
        settings.set_next_word_prediction(false);
        settings.set_emoji_suggestions(false);
        settings.set_clipboard_suggestion(false);
        settings.set_glide_typing(false);
        settings.set_learn_words(false);
        let engine = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_empty()),
            LanguageModels::new(),
        )
        .unwrap();
        engine.set_clock(Rc::new(|| fixture_clock::time(639028224000000000)));
        let candidate = Rc::new(Suggestion::new_nullable(
            if row["text"].is_null() {
                None
            } else {
                Some(text(&row["text"]))
            },
            SuggestionKind(kind),
            0.0,
            false,
        ));
        let log = Rc::new(RefCell::new(Vec::new()));
        let buffer = Rc::new(TextBufferTarget::new(initial, None));
        let target = Rc::new(Target {
            buffer: buffer.clone(),
            accepts_null,
            mutation: mutation.into(),
            candidate: candidate.clone(),
            log: log.clone(),
        });
        engine.attach(target).unwrap();
        let trace = log.clone();
        let current = candidate.clone();
        let mutator = mutation.to_string();
        engine.feedback_requested().add(Rc::new(move |v| {
            trace
                .borrow_mut()
                .push(json!({"tag":"feedback","value":v.0}));
            if mutator == "feedback-text" {
                current.set_text(Some("feedback".into()))
            }
        }));
        let trace = log.clone();
        engine.text_committed().add(Rc::new(move |v| {
            trace
                .borrow_mut()
                .push(json!({"tag":"commit","text":v.units()}))
        }));
        let trace = log.clone();
        engine.emoji_history().changed().add(Rc::new(move |_| {
            trace.borrow_mut().push(json!({"tag":"history"}))
        }));
        let (error, param) = match engine.commit_suggestion(&candidate) {
            Ok(()) => (None, None),
            Err(
                EngineError::NullReference
                | EngineError::Editor(EditorError::NullReference)
                | EngineError::Emoji(ferroboard_core::emoji::EmojiError::NullReference),
            ) => (Some("NullReferenceException"), None),
            Err(EngineError::Editor(EditorError::Target(TargetReplaceError::ArgumentNull(p)))) => {
                (Some("ArgumentNullException"), Some(p))
            }
            Err(e) => panic!("unmeasured source failure {e:?}"),
        };
        let editor = engine.editor().unwrap();
        let actual = json!({"kind":kind,"text":row["text"],"initial":row["initial"],"acceptsNull":accepts_null,"mutation":mutation,"error":error,"param":param,"log":*log.borrow(),"state":{"text":buffer.text().units(),"start":buffer.selection_start(),"length":buffer.selection_length(),"undo":editor.can_undo(),"redo":editor.can_redo(),"candidateText":candidate.text().as_ref().map(Utf16Text::units),"candidateKind":candidate.kind().0,"recent":engine.emoji_history().recent().lock().clone().iter().map(|v|v.as_ref().map(Utf16Text::units)).collect::<Vec<_>>(),"export":engine.emoji_history().export()}});
        assert_eq!(actual, row, "candidate edit {count}");
        engine.dispose().unwrap();
        count += 1;
    }
    assert_eq!(count, 720);
}
