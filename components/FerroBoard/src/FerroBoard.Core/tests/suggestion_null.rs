use ferroboard_core::{
    editor::*, engine::*, layouts::KeyboardResources, settings::KeyboardSettings, text::*,
};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};
fn word(v: &Value) -> Option<Utf16Text> {
    v.as_array().map(|v| {
        Utf16Text::from_units(
            v.iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
struct Custom(Rc<RefCell<Vec<Value>>>);
impl PartialEq for Custom {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ISuggestionProvider for Custom {
    fn suggest(&self, _: Option<&SuggestionRequest>) -> SuggestionResult<Option<SuggestionList>> {
        Ok(Some(SuggestionList::default()))
    }
    fn learn(&self, w: Option<&Utf16Text>, p: Option<&Utf16Text>) -> SuggestionResult<()> {
        self.0.borrow_mut().push(json!({"tag":"learn","word":w.map(Utf16Text::units),"previous":p.map(Utf16Text::units)}));
        Ok(())
    }
    fn forget(&self, w: Option<&Utf16Text>) -> SuggestionResult<()> {
        self.0
            .borrow_mut()
            .push(json!({"tag":"forget","word":w.map(Utf16Text::units)}));
        Ok(())
    }
}
#[test]
fn source_nullable_words_candidates_and_factory_failure_precedence() {
    let mut matched = 0;
    for line in include_str!("../../../tests/fixtures/suggestion-null.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let op = row["op"].as_str().unwrap();
        let actual = match row["tag"].as_str().unwrap() {
            "dictionary" => {
                let p = DictionarySuggestionProvider::new(
                    Arc::new(WordDictionary::from_words([("ab".into(), 100)], "".into()).unwrap()),
                    None,
                );
                let w = word(&row["word"]);
                let previous = word(&row["previous"]);
                let result = if op == "learn" {
                    p.learn_nullable(w.as_ref(), previous.as_ref())
                } else {
                    p.forget_nullable(w.as_ref())
                };
                let (error, param) = match result {
                    Ok(()) => (None, None),
                    Err(SuggestionError::NullReference) => (Some("NullReferenceException"), None),
                    Err(SuggestionError::ArgumentNull(p)) => {
                        (Some("ArgumentNullException"), Some(p))
                    }
                    Err(e) => panic!("unexpected source error {e:?}"),
                };
                json!({"tag":"dictionary","op":op,"word":row["word"],"previous":row["previous"],"error":error,"param":param,"export":p.user_dictionary().export()})
            }
            "custom" => {
                let log = Rc::new(RefCell::new(Vec::new()));
                let p: Rc<dyn ISuggestionProvider> = Rc::new(Custom(log.clone()));
                let w = word(&row["word"]);
                if op == "learn" {
                    p.learn(w.as_ref(), None).unwrap()
                } else {
                    p.forget(w.as_ref()).unwrap()
                };
                json!({"tag":"custom","op":op,"word":row["word"],"log":*log.borrow()})
            }
            "engine" => {
                let attached = row["attached"].as_bool().unwrap();
                let null_object = row["nullObject"].as_bool().unwrap();
                let throws = row["throwsFactory"].as_bool().unwrap();
                let s = Rc::new(KeyboardSettings::default());
                s.set_auto_capitalization(false);
                s.set_show_suggestions(false);
                s.set_auto_correct(false);
                s.set_emoji_suggestions(false);
                s.set_glide_typing(false);
                s.set_next_word_prediction(false);
                s.set_clipboard_suggestion(false);
                let e = KeyboardEngine::new_explicit(
                    s,
                    Rc::new(KeyboardResources::create_empty()),
                    LanguageModels::new(),
                )
                .unwrap();
                if attached {
                    e.attach(Rc::new(TextBufferTarget::new("ab".into(), None)))
                        .unwrap()
                };
                let log = Rc::new(RefCell::new(Vec::new()));
                let trace = log.clone();
                e.feedback_requested().add(Rc::new(move |v| {
                    trace
                        .borrow_mut()
                        .push(json!({"tag":"feedback","value":v.0}))
                }));
                let trace = log.clone();
                e.set_suggestion_provider_factory(Some(Rc::new(move |_| {
                    trace.borrow_mut().push(json!({"tag":"factory"}));
                    if throws {
                        return Err(EngineError::Suggestion(SuggestionError::Custom(
                            "factory".into(),
                        )));
                    }
                    Ok(Some(Rc::new(Custom(trace.clone()))))
                })));
                let value = (!null_object)
                    .then(|| Suggestion::new_nullable(None, SuggestionKind::Clipboard, 0.0, false));
                let result = if op == "commit" {
                    e.commit_suggestion_nullable(value.as_ref())
                } else {
                    e.remove_suggestion_nullable(value.as_ref())
                };
                let (error, param): (Option<&str>, Option<&str>) = match result {
                    Ok(()) => (None, None),
                    Err(EngineError::NullReference) => (Some("NullReferenceException"), None),
                    Err(EngineError::Suggestion(SuggestionError::Custom(_))) => {
                        (Some("ApplicationException"), None)
                    }
                    Err(x) => panic!("unmeasured source failure {x:?}"),
                };
                let result = json!({"tag":"engine","attached":attached,"op":op,"nullObject":null_object,"throwsFactory":throws,"error":error,"param":param,"log":*log.borrow(),"text":e.target().as_ref().map(|t|t.text().units().to_vec())});
                e.dispose().unwrap();
                result
            }
            tag => panic!("unmeasured source tag {tag}"),
        };
        assert_eq!(actual, row);
        matched += 1;
    }
    assert_eq!(matched, 40);
}
