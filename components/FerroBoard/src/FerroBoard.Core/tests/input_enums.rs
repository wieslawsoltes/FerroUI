//! Raw input enum behavior from the entire unchanged Core project.
#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
use ferroboard_core::{editor::*, engine::*, keys::*, layouts::*, settings::*, text::*};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/input-enums.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn units(v: &Value) -> Utf16Text {
    Utf16Text::from_units(serde_json::from_value::<Vec<u16>>(v.clone()).unwrap())
}
fn attr(a: &InputAttributes) -> Value {
    json!({"kind":a.kind().0,"action":a.enter_action().0,"cap":a.capitalization().0,"multi":a.is_multiline(),"suggest":a.allow_suggestions(),"correct":a.allow_auto_correct(),"privateField":a.is_private(),"password":a.is_password(),"variation":a.variation().0,"mode":a.initial_mode().0,"effective":a.effective_enter_action().0,"nlp":a.supports_nlp()})
}
fn engine(settings: Rc<KeyboardSettings>, models: &Arc<LanguageModels>) -> Rc<KeyboardEngine> {
    let e = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_empty()),
        models.clone(),
    )
    .unwrap();
    e.set_clock(Rc::new(|| fixture_clock::time(639028224000000000)));
    e
}
fn settings() -> Rc<KeyboardSettings> {
    let s = Rc::new(KeyboardSettings::new());
    s.set_glide_typing(false);
    s.set_auto_capitalization(false);
    s.set_show_suggestions(false);
    s.set_next_word_prediction(false);
    s.set_emoji_suggestions(false);
    s.set_clipboard_suggestion(false);
    s
}
#[test]
fn source_input_enum_names_values_policy_and_enter_icons() {
    for row in rows() {
        match row["tag"].as_str().unwrap() {
            "name" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                let ty = row["type"].as_str().unwrap();
                let (name, debug) = match ty {
                    "InputKind" => (InputKind(raw).to_string(), format!("{:?}", InputKind(raw))),
                    "EnterAction" => (
                        EnterAction(raw).to_string(),
                        format!("{:?}", EnterAction(raw)),
                    ),
                    "CapitalizationMode" => (
                        CapitalizationMode(raw).to_string(),
                        format!("{:?}", CapitalizationMode(raw)),
                    ),
                    _ => panic!("unknown input enum"),
                };
                assert_eq!(debug, name);
                let mut actual = json!({"tag":"name","type":ty,"raw":raw,"name":name});
                if ty == "EnterAction" {
                    actual["icon"] = json!(KeyLabels::get_enter_icon(EnterAction(raw)).0);
                }
                assert_eq!(actual, row);
            }
            "attributes" => {
                let v = &row["state"];
                let a =
                    InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
                        kind: InputKind(v["kind"].as_i64().unwrap() as i32),
                        enter_action: EnterAction(v["action"].as_i64().unwrap() as i32),
                        capitalization: CapitalizationMode(v["cap"].as_i64().unwrap() as i32),
                        is_multiline: v["multi"].as_bool().unwrap(),
                        allow_suggestions: v["suggest"].as_bool().unwrap(),
                        allow_auto_correct: v["correct"].as_bool().unwrap(),
                        is_private: v["privateField"].as_bool().unwrap(),
                    });
                assert_eq!(attr(&a), row["state"]);
            }
            _ => {}
        }
    }
}
#[test]
fn source_unknown_capitalization_retains_sentence_boundary_semantics() {
    for row in rows().into_iter().filter(|v| v["tag"] == "capitalization") {
        let raw = row["raw"].as_i64().unwrap() as i32;
        let text = units(&row["text"]);
        let selected = row["selected"].as_bool().unwrap();
        let t = Rc::new(TextBufferTarget::new(text.clone(), None));
        if selected {
            t.select(0, 1);
        }
        let e = EditorController::new(t.clone());
        let result = e
            .should_auto_capitalize(CapitalizationMode(raw), &PunctuationRule::shared_default())
            .unwrap();
        assert_eq!(
            json!({"tag":"capitalization","raw":raw,"text":text.units(),"selected":selected,"start":t.selection_start(),"length":t.selection_length(),"result":result}),
            row
        );
    }
}
#[test]
fn source_raw_enter_action_dispatch_handling_and_multiline_fallback() {
    // This fixture owns one registry; individual engines and targets remain fresh.
    let models = LanguageModels::new();
    for row in rows().into_iter().filter(|v| v["tag"] == "engine") {
        let raw = row["raw"].as_i64().unwrap() as i32;
        let kind = row["kind"].as_i64().unwrap() as i32;
        let multi = row["multi"].as_bool().unwrap();
        let handled = row["handled"].as_bool().unwrap();
        let e = engine(settings(), &models);
        let t = Rc::new(TextBufferTarget::new(
            "ab".into(),
            Some(Arc::new(InputAttributes::from_values(
                ferroboard_core::editor::InputAttributesValues {
                    kind: InputKind(kind),
                    enter_action: EnterAction(raw),
                    is_multiline: multi,
                    capitalization: CapitalizationMode::None,
                    ..Default::default()
                },
            ))),
        ));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let log = trace.clone();
        let weak = Rc::downgrade(&t);
        t.set_enter_action_handler(Some(Rc::new(move|a|{let t=weak.upgrade().unwrap();log.borrow_mut().push(json!({"eventKind":"handler","action":a.0,"text":t.text().units(),"start":t.selection_start()}));handled})));
        e.attach(t.clone()).unwrap();
        let log = trace.clone();
        e.text_committed().add(Rc::new(move |v| {
            log.borrow_mut()
                .push(json!({"eventKind":"text","text":v.units()}))
        }));
        for (name, event) in [
            ("keyboard", e.keyboard_changed()),
            ("state", e.state_changed()),
            ("suggestions", e.suggestions_changed()),
        ] {
            let log = trace.clone();
            event.add(Rc::new(move |_| log.borrow_mut().push(json!(name))));
        }
        let log = trace.clone();
        e.feedback_requested().add(Rc::new(move |v| {
            log.borrow_mut()
                .push(json!({"eventKind":"feedback","value":v.0}))
        }));
        e.input_key(TextKeyData::function(KeyCode::Enter, KeyType::Function, None).as_ref())
            .unwrap();
        assert_eq!(
            json!({"tag":"engine","raw":raw,"kind":kind,"multi":multi,"handled":handled,"text":t.text().units(),"start":t.selection_start(),"length":t.selection_length(),"actions":t.performed_actions().borrow().iter().map(|a|a.0).collect::<Vec<_>>(),"incognito":e.is_incognito(),"trace":*trace.borrow()}),
            row
        );
        e.dispose().unwrap();
    }
}
#[derive(Default)]
struct ProbeProvider {
    learned: RefCell<Vec<Value>>,
}
impl PartialEq for ProbeProvider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ISuggestionProvider for ProbeProvider {
    fn suggest(&self, _: Option<&SuggestionRequest>) -> SuggestionResult<Option<SuggestionList>> {
        Ok(Some(SuggestionList::new(Vec::new())))
    }
    fn learn(
        &self,
        word: Option<&Utf16Text>,
        previous: Option<&Utf16Text>,
    ) -> SuggestionResult<()> {
        self.learned
            .borrow_mut()
            .push(json!({"word":word.map(Utf16Text::units),"previous":previous.map(|v|v.units())}));
        Ok(())
    }
    fn forget(&self, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        panic!("privacy probe must not forget")
    }
}
#[test]
fn source_unknown_input_kind_learning_privacy_and_history_gates() {
    // This fixture owns one registry; individual engines and targets remain fresh.
    let models = LanguageModels::new();
    for row in rows().into_iter().filter(|v| v["tag"] == "privacy") {
        let kind = row["kind"].as_i64().unwrap() as i32;
        let profile = row["profile"].as_i64().unwrap();
        let s = settings();
        s.set_auto_correct(false);
        s.set_incognito_mode(profile & 2 != 0);
        s.set_learn_words(profile & 4 != 0);
        s.set_clipboard_history_enabled(profile & 4 != 0);
        s.set_emoji_history_enabled(profile & 4 != 0);
        let e = engine(s, &models);
        let provider = Rc::new(ProbeProvider::default());
        let custom = provider.clone();
        e.set_suggestion_provider_factory(Some(Rc::new(move |_| Ok(Some(custom.clone())))));
        let t = Rc::new(TextBufferTarget::new(
            "".into(),
            Some(Arc::new(InputAttributes::from_values(
                ferroboard_core::editor::InputAttributesValues {
                    kind: InputKind(kind),
                    enter_action: EnterAction(99),
                    capitalization: CapitalizationMode(99),
                    is_private: profile & 1 != 0,
                    ..Default::default()
                },
            ))),
        ));
        e.attach(t.clone()).unwrap();
        for ch in "word".chars() {
            e.type_text(&ch.to_string().as_str().into()).unwrap();
        }
        e.input_key(TextKeyData::char(" ".into(), None).unwrap().as_ref())
            .unwrap();
        e.record_clipboard(&"secret".into()).unwrap();
        e.input_emoji(&"🙂".into()).unwrap();
        assert_eq!(
            json!({"tag":"privacy","kind":kind,"profile":profile,"incognito":e.is_incognito(),"text":t.text().units(),"learned":*provider.learned.borrow(),"primary":e.clipboard_history().primary().map(|v|v.text.units().to_vec()),"recent":e.emoji_history().recent().lock().clone().iter().map(|v|v.as_ref().map(|v|v.units().to_vec())).collect::<Vec<_>>(),"loaded":e.language_models().is_loaded(&"en".into())}),
            row
        );
        e.dispose().unwrap();
    }
}
