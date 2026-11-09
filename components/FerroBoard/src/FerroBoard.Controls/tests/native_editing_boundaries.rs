#[path = "support/headless.rs"]
mod headless;
#[path = "../../../tests/support/local_future.rs"]
mod native_future;

use ferroboard_controls::input::{
    NativeInputError, PasswordBoxInputTarget, ScreenKeyboardInput, TextBoxInputTarget,
};
use ferroboard_core::{
    clipboard::{ClipboardFuture, IClipboardService},
    editor::{EnterAction, ITextInputTarget},
    engine::{KeyboardEngine, LanguageModels},
    keys::{KeyCode, KeyType},
    layouts::{KeyboardResources, TextKeyData},
    settings::KeyboardSettings,
    text::Utf16Text,
};
use ferroui_base::{Ref, threading::Dispatcher};
use ferroui_controls::{Border, Control, TextBox, Window};
use serde_json::Value;
use std::{cell::Cell, rc::Rc};

fn show(control: &Ref<Border>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(500.0);
    window.set_height(300.0);
    window.set_content(Some(Control::boxed(control.clone())));
    window.show();
    Dispatcher::current_dispatcher().run_jobs(None);
    window
}
fn engine(target: Rc<dyn ITextInputTarget>) -> Rc<KeyboardEngine> {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_auto_capitalization(false);
    settings.set_auto_correct(false);
    settings.set_show_suggestions(false);
    settings.set_glide_typing(false);
    settings.set_clipboard_history_enabled(false);
    let engine = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap();
    engine.attach(target).unwrap();
    engine
}
fn units(value: &Value) -> Utf16Text {
    Utf16Text::from_units(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| u16::try_from(v.as_u64().unwrap()).unwrap())
            .collect::<Vec<_>>(),
    )
}

#[test]
fn native_replacement_boundaries_match_frozen_source_with_explicit_encoding_limits() {
    headless::run(|| {
        let mut successes = 0;
        let mut source_errors = 0;
        let mut encoding_limits = 0;
        for line in include_str!("../../../tests/fixtures/native-editing-boundaries.jsonl").lines()
        {
            let row: Value = serde_json::from_str(line).unwrap();
            let initial = units(&row["initial"]);
            let expected = units(&row["text"]);
            let insertion = units(&row["insert"]);
            let field = TextBox::new();
            field.set_text(Some(&initial.to_utf8().unwrap()));
            field.set_max_length(row["max"].as_i64().unwrap() as i32);
            let password = row["password"].as_bool().unwrap();
            if password {
                field.set_password_char('●');
            }
            let plain = (!password).then(|| TextBoxInputTarget::new(field.clone(), None).unwrap());
            let secret =
                password.then(|| PasswordBoxInputTarget::new(field.clone(), None).unwrap());
            let target: Rc<dyn ITextInputTarget> = match (&plain, &secret) {
                (Some(target), None) => target.as_input_target(),
                (None, Some(target)) => target.clone(),
                _ => unreachable!(),
            };
            target.select(1, 0);
            Dispatcher::current_dispatcher().run_jobs(None);
            let changes = Rc::new(Cell::new(0));
            let count = changes.clone();
            target
                .changed()
                .add(Rc::new(move |_| count.set(count.get() + 1)));
            let start = row["start"].as_i64().unwrap() as i32;
            let length = row["length"].as_i64().unwrap() as i32;
            let result = match (&plain, &secret) {
                (Some(target), None) => target.try_replace(start, length, &insertion),
                (None, Some(target)) => target.try_replace(start, length, &insertion),
                _ => unreachable!(),
            };
            if !row["error"].is_null() {
                assert_eq!(row["error"], "ArgumentOutOfRangeException", "{row}");
                assert_eq!(result, Err(NativeInputError::SubstringOutOfRange), "{row}");
                assert_eq!(target.text(), initial, "{row}");
                assert_eq!(target.selection_start(), 1, "{row}");
                source_errors += 1;
            } else if expected.to_utf8().is_err() {
                assert_eq!(result, Err(NativeInputError::UnpairedSurrogate), "{row}");
                assert_eq!(
                    target.text(),
                    initial,
                    "encoding failure must be atomic: {row}"
                );
                assert_eq!(target.selection_start(), 1, "{row}");
                encoding_limits += 1;
            } else {
                result.unwrap();
                assert_eq!(target.text(), expected, "{row}");
                assert_eq!(
                    target.selection_start(),
                    row["caret"].as_i64().unwrap() as i32,
                    "{row}"
                );
                assert_eq!(
                    target.selection_length(),
                    row["selection"].as_i64().unwrap() as i32,
                    "{row}"
                );
                successes += 1;
            }
            Dispatcher::current_dispatcher().run_jobs(None);
            assert_eq!(
                changes.get(),
                0,
                "own edits/errors must not become external edits: {row}"
            );
        }
        assert_eq!((successes, source_errors, encoding_limits), (19, 1, 4));
    });
}

#[derive(Default)]
struct PendingClipboard(native_future::Completion<Option<Utf16Text>>);
impl IClipboardService for PendingClipboard {
    fn get_text_async(&self) -> ClipboardFuture<Option<Utf16Text>> {
        self.0.future()
    }
    fn set_text_async(&self, _: Option<Utf16Text>) -> ClipboardFuture<()> {
        Box::pin(std::future::ready(Ok(())))
    }
}

#[test]
fn pending_paste_observes_live_read_only_and_disabled_ancestor_guards() {
    headless::run(|| {
        for disable_ancestor in [false, true] {
            let field = TextBox::new();
            field.set_text(Some("abcd"));
            let border = Border::new();
            border.set_child(field.clone());
            let window = show(&border);
            let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
            target.select(1, 2);
            Dispatcher::current_dispatcher().run_jobs(None);
            let engine = engine(target.as_input_target());
            let clipboard = Rc::new(PendingClipboard::default());
            engine.set_clipboard_service(clipboard.clone());
            let mut paste = engine.paste_async();
            assert!(native_future::poll(paste.as_mut()).is_pending());
            if disable_ancestor {
                border.set_is_enabled(false);
            } else {
                field.set_is_read_only(true);
            }
            clipboard.0.complete(Some("late".into()));
            native_future::ready(paste).unwrap();
            assert_eq!(target.text(), Utf16Text::from("abcd"));
            assert_eq!(
                (target.selection_start(), target.selection_length()),
                (1, 2)
            );
            border.set_is_enabled(true);
            field.set_is_read_only(false);
            engine.input_text(&"x".into()).unwrap();
            assert_eq!(target.text(), Utf16Text::from("axd"));
            engine.dispose().unwrap();
            window.close();
        }
    });
}

#[test]
fn masked_native_targets_block_live_read_only_and_disabled_edits() {
    headless::run(|| {
        for policy in 0..3 {
            let field = TextBox::new();
            field.set_password_char('●');
            field.set_text(Some("abcd"));
            let border = Border::new();
            border.set_child(field.clone());
            let window = show(&border);
            let target = PasswordBoxInputTarget::new(field.clone(), None).unwrap();
            target.select(1, 0);
            let changes = Rc::new(Cell::new(0));
            let observed = changes.clone();
            target
                .changed()
                .add(Rc::new(move |_| observed.set(observed.get() + 1)));
            match policy {
                0 => field.set_is_read_only(true),
                1 => field.set_is_enabled(false),
                _ => border.set_is_enabled(false),
            }
            target.try_replace(1, 1, &"x".into()).unwrap();
            assert_eq!(
                target.text(),
                Utf16Text::from("abcd"),
                "blocked policy {policy}"
            );
            assert_eq!(target.selection_start(), 1);
            target.try_replace_nullable(0, 0, None).unwrap();
            target
                .try_replace(0, 0, &Utf16Text::from_units(vec![0xd800]))
                .unwrap();
            Dispatcher::current_dispatcher().run_jobs(None);
            assert_eq!(changes.get(), 0);
            border.set_is_enabled(true);
            field.set_is_enabled(true);
            field.set_is_read_only(false);
            target.try_replace(1, 1, &"x".into()).unwrap();
            assert_eq!(target.text(), Utf16Text::from("axcd"));
            assert_eq!(target.selection_start(), 2);
            target.detach();
            window.close();
        }
    });
}

#[test]
fn pending_masked_paste_observes_policy_at_completion_and_recovers() {
    headless::run(|| {
        for disable_ancestor in [false, true] {
            let field = TextBox::new();
            field.set_password_char('●');
            field.set_text(Some("abcd"));
            let border = Border::new();
            border.set_child(field.clone());
            let window = show(&border);
            let target = PasswordBoxInputTarget::new(field.clone(), None).unwrap();
            target.select(1, 0);
            let engine = engine(target.clone());
            let clipboard = Rc::new(PendingClipboard::default());
            engine.set_clipboard_service(clipboard.clone());
            let mut paste = engine.paste_async();
            assert!(native_future::poll(paste.as_mut()).is_pending());
            if disable_ancestor {
                border.set_is_enabled(false);
            } else {
                field.set_is_read_only(true);
            }
            clipboard.0.complete(Some("late".into()));
            native_future::ready(paste).unwrap();
            assert_eq!(target.text(), Utf16Text::from("abcd"));
            assert_eq!(target.selection_start(), 1);
            border.set_is_enabled(true);
            field.set_is_read_only(false);
            engine.input_text(&"x".into()).unwrap();
            assert_eq!(target.text(), Utf16Text::from("axbcd"));
            engine.dispose().unwrap();
            target.detach();
            window.close();
        }
    });
}

#[test]
fn rejected_unicode_edit_restores_engine_external_selection_handling() {
    headless::run(|| {
        let field = TextBox::new();
        field.set_text(Some("ab"));
        field.set_max_length(3);
        let target = TextBoxInputTarget::new(field.clone(), None).unwrap();
        target.select(1, 0);
        Dispatcher::current_dispatcher().run_jobs(None);
        let engine = engine(target.as_input_target());
        assert!(engine.input_text(&"😀".into()).is_err());
        assert_eq!(target.text(), Utf16Text::from("ab"));
        assert_eq!(target.selection_start(), 1);
        let editor = engine.editor().unwrap();
        editor.toggle_selection_mode().unwrap();
        assert!(editor.is_selecting());
        field.set_caret_index(0);
        assert!(
            !editor.is_selecting(),
            "failed edit must restore target-event processing"
        );
        engine.input_text(&"é".into()).unwrap();
        assert_eq!(target.text(), Utf16Text::from("éab"));
        Dispatcher::current_dispatcher().run_jobs(None);
        editor.toggle_selection_mode().unwrap();
        assert!(editor.is_selecting());
        field.set_text(Some("external"));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert!(!editor.is_selecting());
        assert_eq!(target.text(), Utf16Text::from("external"));
        engine.dispose().unwrap();
    });
}

#[test]
fn enter_handler_precedence_and_multiline_fallback_use_native_target_contract() {
    headless::run(|| {
        for multiline in [false, true] {
            for handled in [false, true] {
                let field = TextBox::new();
                field.set_text(Some("ab"));
                field.set_accepts_return(multiline);
                ScreenKeyboardInput::set_enter_action(&field, Some(Rc::new(EnterAction::Send)));
                let calls = Rc::new(Cell::new(0));
                let count = calls.clone();
                let target = TextBoxInputTarget::new(
                    field.clone(),
                    Some(Rc::new(move |action| {
                        assert_eq!(action, EnterAction::Send);
                        count.set(count.get() + 1);
                        handled
                    })),
                )
                .unwrap();
                target.select(1, 0);
                let engine = engine(target.as_input_target());
                engine
                    .input_key(
                        TextKeyData::function(KeyCode::Enter, KeyType::Function, None).as_ref(),
                    )
                    .unwrap();
                assert_eq!(calls.get(), 1);
                let expected = if multiline && !handled { "a\nb" } else { "ab" };
                assert_eq!(target.text(), Utf16Text::from(expected));
                engine.dispose().unwrap();
            }
        }
    });
}
