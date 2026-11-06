#[path = "../../../tests/support/fixture_clipboard_time.rs"]
mod fixture_clipboard_time;
#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
#[path = "../../../tests/support/fixture_duration.rs"]
mod fixture_duration;
#[path = "../../../tests/support/local_future.rs"]
mod native_future;
use ferroboard_core::{editor::*, engine::*, keys::*, layouts::*, settings::*, text::*};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/engine-input.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn units(t: &Utf16Text) -> Value {
    json!(t.units())
}
fn state(e: &KeyboardEngine, t: &TextBufferTarget) -> Value {
    let target = e.target();
    let editor = e.editor();
    let settings = e.settings();
    let users = e
        .language_models()
        .user_dictionaries()
        .iter()
        .map(|(lang, user)| {
            (
                lang.to_utf8().unwrap(),
                serde_json::from_str::<Value>(&user.export()).unwrap(),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    json!({"attached":target.is_some(),"text":units(&target.as_ref().map(|t|t.text()).unwrap_or_default()),"start":target.as_ref().map_or(0,|t|t.selection_start()),"length":target.as_ref().map_or(0,|t|t.selection_length()),"mode":e.mode().0,"shift":e.shift_state().0,"panel":e.ui_mode().0,"half":e.is_char_half_width(),"kata":e.is_kana_kata(),"gliding":e.is_gliding(),"canGlide":e.can_glide(),"version":e.keyboard().unwrap().version(),"selecting":editor.as_ref().is_some_and(|e|e.is_selecting()),"undo":editor.as_ref().is_some_and(|e|e.can_undo()),"redo":editor.as_ref().is_some_and(|e|e.can_redo()),"incognito":e.is_incognito(),"suggestions":e.suggestions().snapshot().unwrap().into_iter().map(|v|v.expect("ordinary source fixture has nonnull candidates")).map(|s|json!({"text":s.text().as_ref().map(units),"kind":s.kind().0,"confidence":format!("{:016x}",s.confidence().to_bits()),"auto":s.is_auto_commit(),"description":s.description().as_ref().map(units),"removable":s.is_removable()})).collect::<Vec<_>>(),"primary":e.clipboard_history().primary().map(|c|units(&c.text)),"recent":e.emoji_history().recent().lock().clone().iter().map(|v|v.as_ref().map(units)).collect::<Vec<_>>(),"actions":if target.is_some(){t.performed_actions().borrow().iter().map(|a|(a).0).collect::<Vec<_>>()}else{Vec::new()},"users":users,"settings":{"floating":settings.floating(),"oneHanded":settings.one_handed_mode().0,"split":settings.split_keyboard(),"incognito":settings.incognito_mode(),"correct":settings.auto_correct(),"smartbar":settings.smartbar_enabled()}})
}
fn key(code: i32) -> TextKey {
    match code {
        KeyCode::Space => TextKeyData::space_key(),
        KeyCode::Enter | KeyCode::Delete => {
            TextKeyData::function(code, KeyType::EnterEditing, None)
        }
        KeyCode::Shift => TextKeyData::function(code, KeyType::Modifier, None),
        _ => TextKeyData::function(code, KeyType::Function, None),
    }
}
fn source_error(e: EngineError) -> String {
    match e {
        EngineError::Key(KeyDataError::InvalidCodePoint(_) | KeyDataError::InvalidSurrogate(_)) => {
            "ArgumentOutOfRangeException"
        }
        EngineError::Editor(EditorError::AbsoluteValueOverflow) => "OverflowException",
        EngineError::Editor(EditorError::SubstringOutOfRange | EditorError::InvalidClampRange) => {
            "ArgumentOutOfRangeException"
        }
        e => panic!("unexpected engine error {e:?}"),
    }
    .into()
}
fn point(e: &KeyboardEngine, label: &str) -> Result<(f64, f64), String> {
    let keyboard = e.keyboard().unwrap();
    for k in keyboard.character_keys().unwrap() {
        if ordinal_ignore_case_equal(&k.output().unwrap().unwrap(), &label.into()) {
            let b = k.touch_bounds();
            return Ok((b.center_x(), b.center_y()));
        }
    }
    Err("InvalidOperationException".into())
}
fn apply(
    e: &Rc<KeyboardEngine>,
    t: &Rc<TextBufferTarget>,
    now: &Cell<i64>,
    op: &[Value],
) -> Result<(), String> {
    let result = match op[0].as_str().unwrap() {
        "attach" => e.attach(t.clone()),
        "detach" => e.detach(),
        "layout" => e.update_layout(800.0, 240.0, None),
        "type" => e.type_text(&op[1].as_str().unwrap().into()),
        "raw" => e.input_text(&op[1].as_str().unwrap().into()),
        "units" => e.type_text(&Utf16Text::from_units(
            op[1]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )),
        "key" => e.input_key(key(op[1].as_i64().unwrap() as i32).as_ref()),
        "down" => e.on_key_down(key(op[1].as_i64().unwrap() as i32).as_ref()),
        "up" => e.on_key_up(key(op[1].as_i64().unwrap() as i32).as_ref()),
        "repeat" => e.on_key_repeat(key(op[1].as_i64().unwrap() as i32).as_ref()),
        "cancel" => {
            e.on_key_cancel(key(op[1].as_i64().unwrap() as i32).as_ref());
            Ok(())
        }
        "ticks" => {
            now.set(now.get() + op[1].as_i64().unwrap());
            Ok(())
        }
        "external" => {
            t.set_text(Some(op[1].as_str().unwrap().into()));
            Ok(())
        }
        "caret" => {
            t.move_caret_externally(op[1].as_i64().unwrap() as i32);
            Ok(())
        }
        "setting" => {
            let value = op[2].as_bool().unwrap();
            let settings = e.settings();
            match op[1].as_str().unwrap() {
                "AutoCapitalization" => settings.set_auto_capitalization(value),
                "AutoCorrect" => settings.set_auto_correct(value),
                "IncognitoMode" => settings.set_incognito_mode(value),
                "LearnWords" => settings.set_learn_words(value),
                "ClipboardHistoryEnabled" => settings.set_clipboard_history_enabled(value),
                "EmojiHistoryEnabled" => settings.set_emoji_history_enabled(value),
                "GlideTyping" => settings.set_glide_typing(value),
                name => panic!("unknown setting {name}"),
            };
            Ok(())
        }
        "window" => {
            e.set_double_space_window(fixture_duration::duration(op[1].as_i64().unwrap()));
            Ok(())
        }
        "subtype" => e.set_subtypes([e.find_subtype(&op[1].as_str().unwrap().into()).unwrap()]),
        "commit" => {
            let kind = SuggestionKind(op[1].as_i64().unwrap() as i32);
            let suggestion = if op.len() > 2 {
                Rc::new(Suggestion::from_values(SuggestionValues {
                    confidence: 1.0,
                    ..SuggestionValues::new(op[2].as_str().unwrap().into(), kind)
                }))
            } else {
                e.suggestions()
                    .snapshot()
                    .unwrap()
                    .into_iter()
                    .map(|v| v.expect("ordinary source fixture has nonnull candidates"))
                    .find(|s| s.kind() == kind)
                    .ok_or("InvalidOperationException")?
                    .clone()
            };
            e.commit_suggestion(&suggestion)
        }
        "forget" => e.remove_suggestion(&Suggestion::new(
            op[1].as_str().unwrap().into(),
            SuggestionKind::Typed,
        )),
        "emoji" => e.input_emoji(&op[1].as_str().unwrap().into()),
        "record" => e.record_clipboard(&op[1].as_str().unwrap().into()),
        "copy" => {
            native_future::ready(e.copy_async()).unwrap();
            Ok(())
        }
        "cut" => {
            native_future::ready(e.cut_async()).unwrap();
            Ok(())
        }
        "paste" => {
            native_future::ready(e.paste_async()).unwrap();
            Ok(())
        }
        "swipe" => e.execute_swipe_action(
            SwipeAction(op[1].as_i64().unwrap() as i32),
            op[2].as_i64().unwrap() as i32,
        ),
        "precise-begin" => e.begin_precise_selection(),
        "precise-update" => {
            e.update_precise_selection(op[1].as_i64().unwrap() as i32, op[2].as_bool().unwrap())
        }
        "precise-end" => e.end_precise_selection(op[1].as_bool().unwrap()),
        "precise-cancel" => e.cancel_precise_selection(),
        "glide-words" => e
            .glide_classifier()
            .set_words(
                [
                    ("hello", 200),
                    ("help", 150),
                    ("held", 120),
                    ("hero", 100),
                    ("world", 220),
                    ("word", 190),
                    ("good", 180),
                    ("go", 160),
                ]
                .map(|(w, f)| (w.into(), f)),
            )
            .map_err(EngineError::from),
        "glide-begin" => {
            let (x, y) = point(e, op[1].as_str().unwrap())?;
            e.begin_glide(x, y)
        }
        "glide-path" => {
            for c in op[1].as_str().unwrap().chars() {
                let (x, y) = point(e, &c.to_string())?;
                for _ in 0..7 {
                    e.add_glide_point(x, y).map_err(source_error)?;
                }
            }
            Ok(())
        }
        "glide-finish" => e.finish_glide(op[1].as_bool().unwrap()),
        "refresh" => e.update_suggestions(),
        "feedback-long" => {
            e.feedback_long_press();
            Ok(())
        }
        "feedback-step" => {
            e.feedback_gesture_step();
            Ok(())
        }
        name => panic!("unknown op {name}"),
    };
    result.map_err(source_error)
}
fn attrs(row: &Value) -> InputAttributes {
    InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
        kind: match row["kind"].as_i64().unwrap() {
            0 => InputKind::Text,
            7 => InputKind::Password,
            8 => InputKind::NumericPassword,
            9 => InputKind::Search,
            n => panic!("kind {n}"),
        },
        is_private: row["privacy"].as_bool().unwrap(),
        is_multiline: row["multiline"].as_bool().unwrap(),
        allow_suggestions: row["allow"].as_bool().unwrap(),
        allow_auto_correct: row["correct"].as_bool().unwrap(),
        ..Default::default()
    })
}
#[test]
fn complete_input_functions_swipes_composers_privacy_and_glide_match_source_traces() {
    let rows = rows();
    let mut names = Vec::new();
    for row in &rows {
        if row["kind"] == "op" && !names.contains(&row["name"]) {
            names.push(row["name"].clone());
        }
    }
    let mut retired_negative_windows = 0;
    let mut retired_tail_rows = 0;
    for name in names {
        let cases = rows
            .iter()
            .filter(|r| r["kind"] == "op" && r["name"] == name)
            .collect::<Vec<_>>();
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_glide_typing(false);
        let e = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        let now = Rc::new(Cell::new(cases[0]["now"].as_i64().unwrap()));
        let clock = now.clone();
        e.set_clock(Rc::new(move || fixture_clock::time(clock.get())));
        let clock = now.clone();
        e.clipboard_history().set_clock(Rc::new(move || {
            fixture_clipboard_time::timestamp(clock.get(), 0).unwrap()
        }));
        let t = Rc::new(TextBufferTarget::new(
            Utf16Text::default(),
            Some(Arc::new(attrs(&cases[0]["attrs"]))),
        ));
        let log = Rc::new(RefCell::new(Vec::<Value>::new()));
        for (kind, event) in [
            ("keyboard", e.keyboard_changed()),
            ("state", e.state_changed()),
            ("suggestions", e.suggestions_changed()),
            ("hide", e.hide_requested()),
            ("subtypes", e.subtype_picker_requested()),
            ("settings", e.settings_requested()),
            ("resize", e.resize_mode_requested()),
            ("voice", e.voice_input_requested()),
        ] {
            let log = log.clone();
            let t = t.clone();
            event.add(Rc::new(move |e| {
                log.borrow_mut()
                    .push(json!({"kind":kind,"state":state(e,&t)}));
            }));
        }
        let weak = Rc::downgrade(&e);
        let commits = log.clone();
        let target = t.clone();
        e.text_committed().add(Rc::new(move |text| {
            let e = weak.upgrade().unwrap();
            commits
                .borrow_mut()
                .push(json!({"kind":"committed","text":units(text),"state":state(&e,&target)}));
        }));
        let weak = Rc::downgrade(&e);
        let feedback = log.clone();
        let target = t.clone();
        e.feedback_requested().add(Rc::new(move |kind| {
            let e = weak.upgrade().unwrap();
            feedback
                .borrow_mut()
                .push(json!({"kind":"feedback","feedback":kind.0,"state":state(&e,&target)}));
        }));
        let case_count = cases.len();
        for (index, row) in cases.into_iter().enumerate() {
            log.borrow_mut().clear();
            let op = row["op"].as_array().unwrap();
            // Retire only the negative-duration setter and its dependent tail.
            // All earlier boundary assertions in this source trace still execute.
            if op[0] == "window" && op[1].as_i64().unwrap() < 0 {
                retired_negative_windows += 1;
                retired_tail_rows += case_count - index;
                break;
            }
            let error = apply(&e, &t, &now, op).err();
            assert_eq!(
                error,
                row["error"].as_str().map(str::to_owned),
                "{name} #{index} {op:?}"
            );
            assert_eq!(now.get(), row["now"], "{name} #{index} {op:?}");
            assert_eq!(state(&e, &t), row["state"], "{name} #{index} {op:?}");
            assert_eq!(
                json!(*log.borrow()),
                row["events"],
                "events {name} #{index} {op:?}"
            );
        }
        e.dispose().unwrap();
    }
    assert_eq!(retired_negative_windows, 4);
    assert_eq!(retired_tail_rows, 8);
}

use ferroboard_core::clipboard::IClipboardService;
use ferroboard_core::clipboard::{ClipboardFuture, ClipboardServiceError};
struct PendingClipboard {
    get: native_future::Completion<Option<Utf16Text>>,
    set: native_future::Completion<()>,
    sets: RefCell<Vec<Utf16Text>>,
    gets: Cell<i32>,
}
impl Default for PendingClipboard {
    fn default() -> Self {
        Self {
            get: native_future::Completion::default(),
            set: native_future::Completion::default(),
            sets: RefCell::new(Vec::new()),
            gets: Cell::new(0),
        }
    }
}
impl IClipboardService for PendingClipboard {
    fn get_text_async(&self) -> ClipboardFuture<Option<Utf16Text>> {
        self.gets.set(self.gets.get() + 1);
        self.get.future()
    }
    fn set_text_async(&self, text: Option<Utf16Text>) -> ClipboardFuture<()> {
        self.sets.borrow_mut().push(text.unwrap());
        self.set.future()
    }
}
#[test]
fn source_clipboard_await_target_replacement_detachment_denial_and_cancellation() {
    for line in include_str!("../../../tests/fixtures/engine-clipboard-tasks.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let operation = row["operation"].as_str().unwrap();
        let change = row["change"].as_str().unwrap();
        let outcome = row["outcome"].as_str().unwrap();
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_glide_typing(false);
        settings.set_show_suggestions(false);
        settings.set_emoji_suggestions(false);
        let e = KeyboardEngine::new_explicit(
            settings,
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        e.set_clock(Rc::new(|| fixture_clock::time(639028656000000000)));
        let clock = e.clock().unwrap();
        e.clipboard_history().set_clock(Rc::new(move || {
            fixture_clipboard_time::timestamp(fixture_clock::ticks(clock()), 0).unwrap()
        }));
        e.clipboard_history().add(Some(&"fallback".into())).unwrap();
        let first = Rc::new(TextBufferTarget::new("copy me".into(), None));
        let second = Rc::new(TextBufferTarget::new(
            "new".into(),
            Some(Arc::new(InputAttributes::from_values(
                ferroboard_core::editor::InputAttributesValues {
                    is_private: change == "private",
                    ..Default::default()
                },
            ))),
        ));
        e.attach(first.clone()).unwrap();
        if operation != "paste" {
            first.select(0, 4);
        }
        let service = Rc::new(PendingClipboard::default());
        e.set_clipboard_service(service.clone());
        let events = Rc::new(RefCell::new(Vec::<Value>::new()));
        for (kind, event) in [
            ("keyboard", e.keyboard_changed()),
            ("state", e.state_changed()),
            ("suggestions", e.suggestions_changed()),
        ] {
            let events = events.clone();
            let first = first.clone();
            event.add(Rc::new(move |e| {
                events
                    .borrow_mut()
                    .push(json!({"kind":kind,"state":state(e,&first)}))
            }));
        }
        let weak = Rc::downgrade(&e);
        let commits = events.clone();
        let target = first.clone();
        e.text_committed().add(Rc::new(move |text| {
            let e = weak.upgrade().unwrap();
            commits
                .borrow_mut()
                .push(json!({"kind":"committed","text":units(text),"state":state(&e,&target)}));
        }));
        let mut task = match operation {
            "copy" => e.copy_async(),
            "cut" => e.cut_async(),
            _ => e.paste_async(),
        };
        let pending = native_future::poll(task.as_mut());
        let before = json!({"completed":pending.is_ready(),"first":units(&first.text()),"second":units(&second.text()),"sets":service.sets.borrow().iter().map(units).collect::<Vec<_>>(),"gets":service.gets.get()});
        match change {
            "replace" | "private" => e.attach(second.clone()).unwrap(),
            "detach" => e.detach().unwrap(),
            "move" => first.move_caret_externally(7),
            _ => {}
        }
        let denied: ClipboardServiceError = Arc::new(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));
        match outcome {
            "error" => {
                if operation == "paste" {
                    service.get.fail(denied.clone());
                } else {
                    service.set.fail(denied.clone());
                }
            }
            "cancel" => {
                if operation == "paste" {
                    service.get.cancel();
                } else {
                    service.set.cancel();
                }
            }
            _ => {
                if operation == "paste" {
                    service
                        .get
                        .complete(Some(if outcome == "empty" { "" } else { "pasted" }.into()));
                } else {
                    service.set.complete(());
                }
            }
        }
        let result = native_future::ready(task);
        let canceled = match &result {
            Err(ferroboard_core::engine::EngineError::ClipboardService(error)) => error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::Interrupted),
            _ => false,
        };
        let error = if canceled {
            Some("TaskCanceledException") // Reference fixture category only, not a Rust runtime type.
        } else if let Err(ferroboard_core::engine::EngineError::ClipboardService(error)) = &result {
            assert!(Arc::ptr_eq(error, &denied));
            Some("InvalidOperationException") // Normalize the source oracle error category.
        } else {
            assert!(result.is_ok(), "{result:?}");
            None
        };
        let target = match e.target() {
            None => "none",
            Some(t) if Rc::ptr_eq(&t, &(first.clone() as Rc<dyn ITextInputTarget>)) => "first",
            _ => "second",
        };
        let actual = json!({"operation":operation,"change":change,"outcome":outcome,"before":before,"error":error,"completed":true,"faulted":result.is_err() && !canceled,"canceled":canceled,"first":units(&first.text()),"second":units(&second.text()),"target":target,"state":state(&e,&first),"events":*events.borrow()});
        assert_eq!(actual, row, "{operation} {change} {outcome}");
        e.dispose().unwrap();
    }
}
