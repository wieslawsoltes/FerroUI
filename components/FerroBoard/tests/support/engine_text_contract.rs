//! Shared oracle driver: Core-only dependencies; native consumers supply real metadata calls/hooks.
#[path = "fixture_clock.rs"]
mod fixture_clock;
use ferroboard_core::{
    editor::*,
    emoji::EmojiError,
    engine::*,
    layouts::{KeyDataError, KeyboardResources},
    settings::KeyboardSettings,
    text::Utf16Text,
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    rc::Rc,
    sync::Arc,
};
pub type Hook = Rc<dyn Fn(&Utf16Text)>;
type TargetHook = Rc<dyn Fn(&(dyn ITextInputTarget + 'static))>;
pub type Cleanup = Box<dyn FnOnce()>;
#[derive(Debug)]
pub struct HandlerFailure;
#[derive(Clone, Debug)]
pub struct Failure {
    pub category: &'static str,
    pub parameter: Option<&'static str>,
}
pub fn core_error(error: EngineError) -> Failure {
    match error {
        EngineError::NullReference | EngineError::Emoji(EmojiError::NullReference) => Failure {
            category: "NullReferenceException",
            parameter: None,
        },
        EngineError::Key(KeyDataError::InvalidCodePoint(_))
        | EngineError::Emoji(EmojiError::InvalidScalar) => Failure {
            category: "ArgumentOutOfRangeException",
            parameter: Some("utf32"),
        },
        other => panic!("Unexpected source failure: {other:?}"),
    }
}
pub trait Driver {
    fn invoke(
        e: &Rc<KeyboardEngine>,
        method: &str,
        text: Option<&Utf16Text>,
    ) -> Result<(), Failure>;
    fn subscribe(e: &Rc<KeyboardEngine>, hook: Hook) -> Cleanup;
}
pub struct Core;
impl Driver for Core {
    fn invoke(
        e: &Rc<KeyboardEngine>,
        method: &str,
        text: Option<&Utf16Text>,
    ) -> Result<(), Failure> {
        match method {
            "InputText" => e.input_text_nullable(text),
            "TypeText" => e.type_text_nullable(text),
            "InputEmoji" => e.input_emoji_nullable(text),
            _ => unreachable!(),
        }
        .map_err(core_error)
    }
    fn subscribe(e: &Rc<KeyboardEngine>, hook: Hook) -> Cleanup {
        let token = e.text_committed().add(hook);
        let weak = Rc::downgrade(e);
        Box::new(move || {
            if let Some(e) = weak.upgrade() {
                assert!(e.text_committed().remove(token));
            }
        })
    }
}
fn text(v: &Value) -> Option<Utf16Text> {
    v.as_array().map(|a| {
        Utf16Text::from_units(
            a.iter()
                .map(|n| u16::try_from(n.as_u64().unwrap()).unwrap())
                .collect::<Vec<_>>(),
        )
    })
}
fn snapshot(
    e: &Rc<KeyboardEngine>,
    first: &Rc<TextBufferTarget>,
    second: &Rc<TextBufferTarget>,
) -> Value {
    let target = e.target();
    let original: Rc<dyn ITextInputTarget> = first.clone();
    let label = if let Some(t) = target {
        if Rc::ptr_eq(&t, &original) {
            "first"
        } else {
            "second"
        }
    } else {
        "none"
    };
    let editor = e.editor();
    let learned = e
        .language_models()
        .user_dictionaries()
        .iter()
        .map(|(_, v)| v.words().len())
        .sum::<usize>();
    json!({"target":label,"first":first.text().units(),"firstStart":first.selection_start(),"firstLength":first.selection_length(),"second":second.text().units(),"secondStart":second.selection_start(),"secondLength":second.selection_length(),"undo":editor.as_ref().is_some_and(|e|e.can_undo()),"redo":editor.as_ref().is_some_and(|e|e.can_redo()),"recent":e.emoji_history().recent().lock().clone().iter().map(|v|v.as_ref().map(Utf16Text::units)).collect::<Vec<_>>(),"incognito":e.is_incognito(),"historyEnabled":e.settings().emoji_history_enabled(),"shift":e.shift_state().0,"learned":learned,"suggestions":e.suggestions().len().unwrap()})
}
fn record(
    log: &RefCell<Vec<Value>>,
    e: &Rc<KeyboardEngine>,
    first: &Rc<TextBufferTarget>,
    second: &Rc<TextBufferTarget>,
    kind: &str,
    value: Value,
) {
    let state = snapshot(e, first, second);
    log.borrow_mut()
        .push(json!({"kind":kind,"value":value,"state":state}));
}
fn outcome<D: Driver>(
    e: &Rc<KeyboardEngine>,
    method: &str,
    text: Option<&Utf16Text>,
) -> Option<Failure> {
    match catch_unwind(AssertUnwindSafe(|| D::invoke(e, method, text))) {
        Ok(v) => v.err(),
        Err(panic) => {
            if panic.is::<HandlerFailure>() {
                Some(Failure {
                    category: "ProbeFailure",
                    parameter: None,
                })
            } else {
                resume_unwind(panic)
            }
        }
    }
}
pub fn verify<D: Driver>(corpus: &str) {
    let mut count = 0;
    for line in corpus.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let profile = row["profile"].as_str().unwrap();
        let reaction = row["reaction"].as_str().unwrap().to_owned();
        let settings = Rc::new(KeyboardSettings::new());
        settings.set_glide_typing(false);
        settings.set_auto_capitalization(false);
        settings.set_show_suggestions(false);
        settings.set_auto_correct(false);
        settings.set_next_word_prediction(false);
        settings.set_emoji_suggestions(false);
        settings.set_clipboard_suggestion(false);
        settings.set_learn_words(false);
        settings.set_incognito_mode(profile == "incognito");
        settings.set_emoji_history_enabled(profile != "history-off");
        let e = KeyboardEngine::new(
            Some(settings.clone()),
            Some(Rc::new(KeyboardResources::create_empty())),
            None,
        )
        .unwrap();
        e.set_clock(Rc::new(|| fixture_clock::time(30_000_000)));
        let first = Rc::new(TextBufferTarget::new(
            "Q".into(),
            Some(Arc::new(InputAttributes::from_values(
                InputAttributesValues {
                    is_private: profile == "private",
                    kind: if profile == "password" {
                        InputKind::Password
                    } else {
                        InputKind::Text
                    },
                    ..Default::default()
                },
            ))),
        ));
        let second = Rc::new(TextBufferTarget::new(
            "other".into(),
            Some(Arc::new(InputAttributes::from_values(
                InputAttributesValues {
                    is_private: reaction == "private-replace",
                    ..Default::default()
                },
            ))),
        ));
        if row["attached"] == true {
            e.attach(first.clone()).unwrap();
        }
        let log = Rc::new(RefCell::new(Vec::<Value>::new()));
        let once = Rc::new(Cell::new(true));
        let tail_token = Rc::new(Cell::new(0));
        let mut cleanup = Vec::<Cleanup>::new();
        type EventSelector = fn(&KeyboardEngine) -> &ferroui_system::events::Event<KeyboardEngine>;
        for (kind, event) in [
            (
                "keyboard",
                KeyboardEngine::keyboard_changed as EventSelector,
            ),
            ("state", KeyboardEngine::state_changed as EventSelector),
            (
                "suggestions",
                KeyboardEngine::suggestions_changed as EventSelector,
            ),
        ] {
            let a = first.clone();
            let b = second.clone();
            let log = log.clone();
            let weak = Rc::downgrade(&e);
            let token = event(&e).add(Rc::new(move |_| {
                record(&log, &weak.upgrade().unwrap(), &a, &b, kind, Value::Null)
            }));
            let weak = Rc::downgrade(&e);
            cleanup.push(Box::new(move || {
                if let Some(e) = weak.upgrade() {
                    assert!(event(&e).remove(token));
                }
            }));
        }
        {
            let a = first.clone();
            let b = second.clone();
            let log = log.clone();
            let weak = Rc::downgrade(&e);
            let history = e.emoji_history();
            let token = history.changed().add(Rc::new(move |_| {
                record(
                    &log,
                    &weak.upgrade().unwrap(),
                    &a,
                    &b,
                    "history",
                    Value::Null,
                )
            }));
            cleanup.push(Box::new(move || {
                assert!(history.changed().remove(token));
            }));
        }
        for (is_first, edited, kind) in [
            (true, true, "first-edited"),
            (true, false, "first-changed"),
            (false, true, "second-edited"),
            (false, false, "second-changed"),
        ] {
            let t = if is_first {
                first.clone()
            } else {
                second.clone()
            };
            let a = first.clone();
            let b = second.clone();
            let log = log.clone();
            let weak = Rc::downgrade(&e);
            let hook: TargetHook =
                Rc::new(move |_| record(&log, &weak.upgrade().unwrap(), &a, &b, kind, Value::Null));
            let token = if edited {
                t.edited().add(hook)
            } else {
                t.changed().add(hook)
            };
            cleanup.push(Box::new(move || {
                assert!(if edited {
                    t.edited().remove(token)
                } else {
                    t.changed().remove(token)
                });
            }));
        }
        {
            let a = first.clone();
            let b = second.clone();
            let log = log.clone();
            let weak = Rc::downgrade(&e);
            let once = once.clone();
            let reaction = reaction.clone();
            let token = e.feedback_requested().add(Rc::new(move |kind| {
                let e = weak.upgrade().unwrap();
                record(&log, &e, &a, &b, "feedback", json!(kind.0));
                if reaction.starts_with("feedback-") && once.replace(false) {
                    match reaction.as_str() {
                        "feedback-detach" => e.detach().unwrap(),
                        "feedback-throw" => std::panic::panic_any(HandlerFailure),
                        "feedback-history-off" => e.settings().set_emoji_history_enabled(false),
                        _ => unreachable!(),
                    };
                    record(&log, &e, &a, &b, "after-feedback-reaction", Value::Null);
                }
            }));
            let weak = Rc::downgrade(&e);
            cleanup.push(Box::new(move || {
                if let Some(e) = weak.upgrade() {
                    assert!(e.feedback_requested().remove(token));
                }
            }));
        }
        {
            let a = first.clone();
            let b = second.clone();
            let log = log.clone();
            let weak = Rc::downgrade(&e);
            let once = once.clone();
            let tail = tail_token.clone();
            let reaction = reaction.clone();
            cleanup.push(D::subscribe(
                &e,
                Rc::new(move |value| {
                    let e = weak.upgrade().unwrap();
                    record(&log, &e, &a, &b, "head", json!(value.units()));
                    if reaction != "none"
                        && !reaction.starts_with("feedback-")
                        && once.replace(false)
                    {
                        match reaction.as_str() {
                            "nested" => {
                                D::invoke(&e, "InputText", Some(&Utf16Text::from("N"))).unwrap()
                            }
                            "detach" => e.detach().unwrap(),
                            "replace" | "private-replace" => e.attach(b.clone()).unwrap(),
                            "throw" => std::panic::panic_any(HandlerFailure),
                            "remove-tail" => {
                                assert!(e.text_committed().remove(tail.get()));
                            }
                            "history-off" => e.settings().set_emoji_history_enabled(false),
                            _ => unreachable!(),
                        };
                        record(&log, &e, &a, &b, "after-text-reaction", Value::Null);
                    }
                }),
            ));
        }
        {
            let a = first.clone();
            let b = second.clone();
            let log = log.clone();
            let weak = Rc::downgrade(&e);
            let token = e.text_committed().add(Rc::new(move |v| {
                record(
                    &log,
                    &weak.upgrade().unwrap(),
                    &a,
                    &b,
                    "tail",
                    json!(v.units()),
                )
            }));
            tail_token.set(token);
            let weak = Rc::downgrade(&e);
            cleanup.push(Box::new(move || {
                if let Some(e) = weak.upgrade() {
                    e.text_committed().remove(token);
                }
            }));
        }
        let input = text(&row["input"]);
        let failure = outcome::<D>(&e, row["method"].as_str().unwrap(), input.as_ref());
        let after = snapshot(&e, &first, &second);
        let events = log.borrow().clone();
        log.borrow_mut().clear();
        first.move_caret_externally(0);
        let recovery_failure = outcome::<D>(&e, "InputText", Some(&Utf16Text::from("R")));
        assert!(recovery_failure.is_none(), "recovery: {recovery_failure:?}");
        let actual = json!({"method":row["method"],"input":row["input"],"attached":row["attached"],"profile":row["profile"],"reaction":row["reaction"],"error":failure.as_ref().map(|v|v.category),"parameter":failure.as_ref().and_then(|v|v.parameter),"after":after,"events":events,"recovery":snapshot(&e,&first,&second),"recoveryEvents":*log.borrow()});
        assert_eq!(
            actual, row,
            "engine text row {count}: {} / {reaction}",
            row["method"]
        );
        for action in cleanup {
            action();
        }
        e.dispose().unwrap();
        count += 1;
    }
    assert_eq!(count, 300);
}
