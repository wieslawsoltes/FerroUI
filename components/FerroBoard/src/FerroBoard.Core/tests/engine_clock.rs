//! Full unchanged-C# engine clock traces: null, exceptions, reentrancy and attachment snapshots.
#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
use ferroboard_core::{
    editor::*,
    engine::*,
    keys::*,
    layouts::{KeyboardResources, TextKeyData},
    settings::KeyboardSettings,
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

fn state(engine: &KeyboardEngine) -> Value {
    let target = engine.target();
    let editor = engine.editor();
    let clock = engine.clock();
    let same =
        editor
            .as_ref()
            .is_some_and(|editor| match (clock.as_ref(), editor.clock().as_ref()) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            });
    json!({
        "attached": target.is_some(),
        "text": target.as_ref().map(|t| t.text()).unwrap_or_default().units(),
        "start": target.as_ref().map_or(0, |t| t.selection_start()),
        "length": target.as_ref().map_or(0, |t| t.selection_length()),
        "shift": engine.shift_state().0,
        "clockNull": clock.is_none(),
        "editorClockNull": editor.as_ref().is_some_and(|e| e.clock().is_none()),
        "sameClock": same,
        "undo": editor.as_ref().is_some_and(|e| e.can_undo()),
        "redo": editor.as_ref().is_some_and(|e| e.can_redo()),
    })
}

#[derive(Debug)]
struct ProbeException;

fn apply(
    engine: &Rc<KeyboardEngine>,
    target: &Rc<TextBufferTarget>,
    action: &str,
) -> EngineResult<()> {
    let shift = TextKeyData::function(KeyCode::Shift, KeyType::Modifier, None);
    match action {
        "down" => engine.on_key_down(shift.as_ref()),
        "up" => engine.on_key_up(shift.as_ref()),
        "cancel" => {
            engine.on_key_cancel(shift.as_ref());
            Ok(())
        }
        "caps" => engine
            .input_key(TextKeyData::function(KeyCode::CapsLock, KeyType::Modifier, None).as_ref()),
        "character" => engine.type_text(&"a".into()),
        "space" => engine.input_key(TextKeyData::space_key().as_ref()),
        "raw" => engine.input_text(&"x".into()),
        "empty" => engine.input_text(&"".into()),
        "delete" => engine.input_key(
            TextKeyData::function(KeyCode::Delete, KeyType::EnterEditing, None).as_ref(),
        ),
        "same" | "attach" => engine.attach(target.clone()),
        "detach" => engine.detach(),
        "dispose" => engine.dispose(),
        _ => panic!("unexpected source operation {action}"),
    }
}

#[test]
fn unchanged_engine_clock_null_throw_reentrant_attachment_and_event_traces() {
    let rows: Vec<Value> = include_str!("../../../tests/fixtures/engine-clock.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let resources = Rc::new(KeyboardResources::create_default().unwrap());
    let mut compared = 0;
    for placement in ["beforeAttach", "afterAttach", "editorOnly", "detached"] {
        for mode in ["null", "throw", "value", "selfNull", "selfReplace"] {
            for trace in 0..8 {
                let settings = Rc::new(KeyboardSettings::default());
                settings.set_auto_capitalization(false);
                settings.set_show_suggestions(false);
                settings.set_emoji_suggestions(false);
                settings.set_glide_typing(false);
                settings.set_auto_correct(false);
                settings.set_learn_words(false);
                let engine = KeyboardEngine::new_explicit(
                    settings,
                    resources.clone(),
                    LanguageModels::new(),
                )
                .unwrap();
                let target = Rc::new(TextBufferTarget::new("ab".into(), None));
                engine.set_clock(Rc::new(|| fixture_clock::time(100_000_000)));
                let replacement: EditorClock = Rc::new(|| fixture_clock::time(100_000_001));
                let calls = Rc::new(Cell::new(0));
                let failure = Arc::new(ProbeException);
                let counter = calls.clone();
                let marker = failure.clone();
                let weak = Rc::downgrade(&engine);
                let callback: EditorClock = Rc::new(move || {
                    counter.set(counter.get() + 1);
                    if mode == "throw" {
                        std::panic::panic_any(marker.clone());
                    }
                    if mode == "selfNull" {
                        weak.upgrade().unwrap().set_clock_nullable(None);
                    }
                    if mode == "selfReplace" {
                        weak.upgrade().unwrap().set_clock(replacement.clone());
                    }
                    fixture_clock::time(100_000_000 + counter.get())
                });
                let assigned = (mode != "null").then_some(callback);
                if matches!(placement, "beforeAttach" | "detached") {
                    engine.set_clock_nullable(assigned.clone());
                }
                if placement != "detached" {
                    engine.attach(target.clone()).unwrap();
                }
                if placement == "afterAttach" {
                    engine.set_clock_nullable(assigned.clone());
                }
                if placement == "editorOnly" {
                    engine
                        .editor()
                        .unwrap()
                        .set_clock_nullable(assigned.clone());
                }
                let events = Rc::new(RefCell::new(Vec::<Value>::new()));
                for (name, event) in [
                    ("keyboard", engine.keyboard_changed()),
                    ("state", engine.state_changed()),
                    ("suggestions", engine.suggestions_changed()),
                ] {
                    let events = events.clone();
                    let calls = calls.clone();
                    event.add(Rc::new(move |engine| {
                        events
                            .borrow_mut()
                            .push(json!({"name":name,"calls":calls.get(),"state":state(engine)}));
                    }));
                }
                let weak = Rc::downgrade(&engine);
                let commits = events.clone();
                let count = calls.clone();
                engine.text_committed().add(Rc::new(move |text| {
                    commits.borrow_mut().push(json!({"name":"committed","calls":count.get(),"text":text.units(),"state":state(&weak.upgrade().unwrap())}));
                }));
                let weak = Rc::downgrade(&engine);
                let feedbacks = events.clone();
                let count = calls.clone();
                engine.feedback_requested().add(Rc::new(move |feedback| {
                    feedbacks.borrow_mut().push(json!({"name":"feedback","calls":count.get(),"feedback":feedback.0,"state":state(&weak.upgrade().unwrap())}));
                }));
                for row in rows.iter().filter(|row| {
                    row["placement"] == placement && row["mode"] == mode && row["trace"] == trace
                }) {
                    events.borrow_mut().clear();
                    let action = row["action"].as_str().unwrap();
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        apply(&engine, &target, action)
                    }));
                    let (error, same_error) = match result {
                        Ok(Ok(())) => (None, false),
                        Ok(Err(
                            EngineError::NullReference
                            | EngineError::Editor(EditorError::NullReference),
                        )) => (Some("NullReferenceException"), false),
                        Ok(Err(error)) => panic!("unexpected engine error {error:?} for {row}"),
                        Err(payload) => {
                            let Some(marker) = payload.downcast_ref::<Arc<ProbeException>>() else {
                                std::panic::resume_unwind(payload)
                            };
                            (Some("ProbeException"), Arc::ptr_eq(marker, &failure))
                        }
                    };
                    let actual = json!({"placement":placement,"mode":mode,"trace":trace,"action":action,"calls":calls.get(),"error":error,"sameError":same_error,"state":state(&engine),"events":*events.borrow()});
                    assert_eq!(actual, *row, "{placement}/{mode}/{trace}/{action}");
                    compared += 1;
                }
                engine.dispose().unwrap();
            }
        }
    }
    assert_eq!(compared, 620);
}

fn target_state(
    engine: &KeyboardEngine,
    first: &Rc<TextBufferTarget>,
    second: &Rc<TextBufferTarget>,
) -> Value {
    let target = engine.target();
    let first_handle: Rc<dyn ITextInputTarget> = first.clone();
    let second_handle: Rc<dyn ITextInputTarget> = second.clone();
    json!({
        "attached":target.is_some(),
        "firstTarget":target.as_ref().is_some_and(|v|Rc::ptr_eq(v,&first_handle)),
        "secondTarget":target.as_ref().is_some_and(|v|Rc::ptr_eq(v,&second_handle)),
        "firstText":first.text().units(),"secondText":second.text().units(),
        "start":target.as_ref().map_or(0,|v|v.selection_start()),
        "shift":engine.shift_state().0,"undo":engine.editor().is_some_and(|v|v.can_undo())
    })
}

#[test]
fn clock_callbacks_detaching_or_replacing_targets_preserve_original_editor_and_event_order() {
    for line in include_str!("../../../tests/fixtures/engine-clock-reentrancy.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_auto_capitalization(false);
        settings.set_show_suggestions(false);
        settings.set_emoji_suggestions(false);
        settings.set_glide_typing(false);
        settings.set_auto_correct(false);
        settings.set_learn_words(false);
        let engine = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        let first = Rc::new(TextBufferTarget::new("ab".into(), None));
        let second = Rc::new(TextBufferTarget::new("cd".into(), None));
        engine.set_clock(Rc::new(|| fixture_clock::time(100_000_000)));
        engine.attach(first.clone()).unwrap();
        let action = row["action"].as_str().unwrap();
        if action == "up" {
            apply(&engine, &first, "down").unwrap();
        }
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let replace = row["mode"] == "replace";
        let weak = Rc::downgrade(&engine);
        let next = second.clone();
        engine.set_clock(Rc::new(move || {
            count.set(count.get() + 1);
            if count.get() == 1 {
                let engine = weak.upgrade().unwrap();
                engine.detach().unwrap();
                if replace {
                    engine.attach(next.clone()).unwrap();
                }
            }
            fixture_clock::time(100_000_000 + count.get())
        }));
        if action == "raw" {
            engine.editor().unwrap().set_clock_nullable(engine.clock());
        }
        let events = Rc::new(RefCell::new(Vec::<Value>::new()));
        for (name, event) in [
            ("keyboard", engine.keyboard_changed()),
            ("state", engine.state_changed()),
            ("suggestions", engine.suggestions_changed()),
        ] {
            let events = events.clone();
            let count = calls.clone();
            let first = first.clone();
            let second = second.clone();
            event.add(Rc::new(move |engine|events.borrow_mut().push(json!({"name":name,"calls":count.get(),"state":target_state(engine,&first,&second)}))));
        }
        let weak = Rc::downgrade(&engine);
        let commits = events.clone();
        let count = calls.clone();
        let a = first.clone();
        let b = second.clone();
        engine.text_committed().add(Rc::new(move |text|commits.borrow_mut().push(json!({"name":"committed","calls":count.get(),"text":text.units(),"state":target_state(&weak.upgrade().unwrap(),&a,&b)}))));
        let weak = Rc::downgrade(&engine);
        let feedbacks = events.clone();
        let count = calls.clone();
        let a = first.clone();
        let b = second.clone();
        engine.feedback_requested().add(Rc::new(move |feedback|feedbacks.borrow_mut().push(json!({"name":"feedback","calls":count.get(),"feedback":feedback.0,"state":target_state(&weak.upgrade().unwrap(),&a,&b)}))));
        let error = match apply(&engine, &first, action) {
            Ok(()) => None,
            Err(EngineError::NullReference | EngineError::Editor(EditorError::NullReference)) => {
                Some("NullReferenceException")
            }
            Err(EngineError::NoEditor) => Some("NoEditor"),
            Err(error) => panic!("unexpected {error:?} for {row}"),
        };
        assert_eq!(
            json!({"mode":row["mode"],"action":action,"calls":calls.get(),"error":error,"state":target_state(&engine,&first,&second),"events":*events.borrow()}),
            row
        );
        engine.dispose().unwrap();
    }
}
