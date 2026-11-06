#[path = "support/headless.rs"]
mod headless;
#[path = "../../../tests/support/local_future.rs"]
mod native_future;

use ferroboard_controls::{OnScreenKeyboard, input::ScreenKeyboardInput};
use ferroboard_core::{
    clipboard::{ClipboardFuture, ClipboardServiceError, IClipboardService},
    engine::{EngineError, KeyboardEngine, LanguageModels},
    layouts::KeyboardResources,
    settings::KeyboardSettings,
    text::Utf16Text,
};
use ferroui_base::{Ref, threading::Dispatcher};
use ferroui_controls::{Control, StackPanel, TextBox, Window};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

#[derive(Default)]
struct PendingClipboard {
    read: native_future::Completion<Option<Utf16Text>>,
    write: native_future::Completion<()>,
    reads: Cell<usize>,
    writes: RefCell<Vec<Option<Utf16Text>>>,
}
impl IClipboardService for PendingClipboard {
    fn get_text_async(&self) -> ClipboardFuture<Option<Utf16Text>> {
        self.reads.set(self.reads.get() + 1);
        self.read.future()
    }
    fn set_text_async(&self, text: Option<Utf16Text>) -> ClipboardFuture<()> {
        self.writes.borrow_mut().push(text);
        self.write.future()
    }
}
fn keyboard() -> Ref<OnScreenKeyboard> {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_glide_typing(false);
    settings.set_show_suggestions(false);
    settings.set_emoji_suggestions(false);
    let engine = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap();
    OnScreenKeyboard::new(Some(engine)).unwrap()
}
fn show(first: &Ref<TextBox>, second: &Ref<TextBox>) -> Ref<Window> {
    let fields = StackPanel::new();
    fields.children().add(first.clone());
    fields.children().add(second.clone());
    let window = Window::new();
    window.set_content(Some(Control::boxed(fields)));
    window.show();
    Dispatcher::current_dispatcher().run_jobs(None);
    window
}
fn text(field: &TextBox) -> Vec<u16> {
    field.text().unwrap_or_default().encode_utf16().collect()
}

#[test]
fn native_owned_targets_match_sixty_source_pending_clipboard_outcomes() {
    headless::run(|| {
        let mut count = 0;
        for line in include_str!("../../../tests/fixtures/engine-clipboard-tasks.jsonl").lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            let operation = row["operation"].as_str().unwrap();
            let change = row["change"].as_str().unwrap();
            let outcome = row["outcome"].as_str().unwrap();
            let first = TextBox::new();
            first.set_text(Some("copy me"));
            let second = TextBox::new();
            second.set_text(Some("new"));
            ScreenKeyboardInput::set_is_private(&second, change == "private");
            let window = show(&first, &second);
            second.set_caret_index(3);
            let keyboard = keyboard();
            let engine = keyboard.engine();
            engine
                .clipboard_history()
                .add(Some(&"fallback".into()))
                .unwrap();
            keyboard.attach_to(Some(first.clone().upcast())).unwrap();
            let target = engine.target().unwrap();
            target.select(0, if operation == "paste" { 0 } else { 4 });
            if operation == "paste" {
                target.select(7, 0);
            }
            let original_target = Rc::downgrade(&target);
            drop(target);
            Dispatcher::current_dispatcher().run_jobs(None);
            let commits = Rc::new(RefCell::new(Vec::<Vec<u16>>::new()));
            let observed = commits.clone();
            engine.text_committed().add(Rc::new(move |value| {
                observed.borrow_mut().push(value.units().to_vec())
            }));
            let clipboard = Rc::new(PendingClipboard::default());
            engine.set_clipboard_service(clipboard.clone());
            let mut pending = match operation {
                "copy" => engine.copy_async(),
                "cut" => engine.cut_async(),
                _ => engine.paste_async(),
            };
            assert!(native_future::poll(pending.as_mut()).is_pending());
            assert_eq!(
                json!({"completed":false,"first":text(&first),"second":text(&second),
                "sets":clipboard.writes.borrow().iter().map(|v| v.as_ref().map(|v|v.units().to_vec())).collect::<Vec<_>>(),
                "gets":clipboard.reads.get()}),
                row["before"]
            );
            match change {
                "replace" | "private" => {
                    keyboard.attach_to(Some(second.clone().upcast())).unwrap();
                }
                "detach" => {
                    keyboard.detach().unwrap();
                }
                "move" => first.set_caret_index(7),
                _ => {}
            }
            Dispatcher::current_dispatcher().run_jobs(None);
            if matches!(change, "replace" | "private" | "detach") {
                assert!(
                    original_target.upgrade().is_none(),
                    "pending {operation} retained detached native adapter"
                );
            }
            let denied: ClipboardServiceError = Arc::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "denied",
            ));
            match outcome {
                "error" if operation == "paste" => clipboard.read.fail(denied.clone()),
                "error" => clipboard.write.fail(denied.clone()),
                "cancel" if operation == "paste" => clipboard.read.cancel(),
                "cancel" => clipboard.write.cancel(),
                _ if operation == "paste" => clipboard
                    .read
                    .complete(Some(if outcome == "empty" { "" } else { "pasted" }.into())),
                _ => clipboard.write.complete(()),
            }
            let result = native_future::ready(pending);
            let canceled = matches!(&result, Err(EngineError::ClipboardService(error)) if error.downcast_ref::<std::io::Error>().is_some_and(|e|e.kind()==std::io::ErrorKind::Interrupted));
            if outcome == "error" {
                assert!(
                    matches!(&result, Err(EngineError::ClipboardService(error)) if Arc::ptr_eq(error,&denied))
                );
            }
            let active = engine.target();
            let target_name = if active.is_none() {
                "none"
            } else if keyboard.target_element() == Some(second.clone().upcast()) {
                "second"
            } else if keyboard.target_element() == Some(first.clone().upcast()) {
                "first"
            } else {
                panic!("unexpected native target element");
            };
            let expected_commits: Vec<Value> = row["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["kind"] == "committed")
                .map(|e| e["text"].clone())
                .collect();
            assert_eq!(
                json!({"first":text(&first),"second":text(&second),"target":target_name,
                "start":active.as_ref().map_or(0,|v|v.selection_start()),
                "length":active.as_ref().map_or(0,|v|v.selection_length()),
                "incognito":engine.is_incognito(),"primary":engine.clipboard_history().primary().map(|v|v.text.units().to_vec()),
                "failed":result.is_err(),"canceled":canceled,"commits":*commits.borrow()}),
                json!({"first":row["first"],"second":row["second"],"target":row["target"],
                    "start":row["state"]["start"],"length":row["state"]["length"],
                    "incognito":row["state"]["incognito"],"primary":row["state"]["primary"],
                    "failed":!row["error"].is_null(),"canceled":row["canceled"],"commits":expected_commits}),
                "{operation} / {change} / {outcome}"
            );
            drop(active);
            keyboard.release_native_subscriptions().unwrap();
            engine.dispose().unwrap();
            assert!(original_target.upgrade().is_none());
            window.close();
            count += 1;
        }
        assert_eq!(count, 60);
    });
}

#[test]
fn native_external_text_and_utf16_range_replace_the_live_selection_after_await() {
    headless::run(|| {
        let first = TextBox::new();
        first.set_text(Some("old"));
        let window = show(&first, &TextBox::new());
        let keyboard = keyboard();
        keyboard.attach_to(Some(first.clone().upcast())).unwrap();
        let engine = keyboard.engine();
        let clipboard = Rc::new(PendingClipboard::default());
        engine.set_clipboard_service(clipboard.clone());
        let mut pending = engine.paste_async();
        assert!(native_future::poll(pending.as_mut()).is_pending());
        first.set_text(Some("a😀bc"));
        first.set_caret_index(3);
        first.set_selection_start(1);
        first.set_selection_end(3);
        Dispatcher::current_dispatcher().run_jobs(None);
        clipboard.read.complete(Some("é".into()));
        native_future::ready(pending).unwrap();
        assert_eq!(first.text().as_deref(), Some("aébc"));
        assert_eq!(engine.target().unwrap().selection_start(), 2);
        assert_eq!(engine.target().unwrap().selection_length(), 0);
        keyboard.release_native_subscriptions().unwrap();
        engine.dispose().unwrap();
        window.close();
    });
}

#[test]
fn detached_plain_and_masked_adapters_drop_while_paste_waits_and_reattach_cleanly() {
    headless::run(|| {
        for masked in [false, true] {
            let field = TextBox::new();
            if masked {
                field.set_password_char('●');
            }
            field.set_text(Some("abcd"));
            let window = show(&field, &TextBox::new());
            let keyboard = keyboard();
            keyboard.attach_to(Some(field.clone().upcast())).unwrap();
            let engine = keyboard.engine();
            let old_target = Rc::downgrade(&engine.target().unwrap());
            let clipboard = Rc::new(PendingClipboard::default());
            engine.set_clipboard_service(clipboard.clone());
            let mut pending = engine.paste_async();
            assert!(native_future::poll(pending.as_mut()).is_pending());
            keyboard.detach().unwrap();
            assert!(old_target.upgrade().is_none());
            field.set_text(Some("external"));
            Dispatcher::current_dispatcher().run_jobs(None);
            clipboard.read.complete(Some("late".into()));
            native_future::ready(pending).unwrap();
            assert!(engine.target().is_none());
            assert_eq!(field.text().as_deref(), Some("external"));
            keyboard.attach_to(Some(field.clone().upcast())).unwrap();
            engine.target().unwrap().select(8, 0);
            engine.input_text(&"x".into()).unwrap();
            assert_eq!(field.text().as_deref(), Some("externalx"));
            keyboard.release_native_subscriptions().unwrap();
            engine.dispose().unwrap();
            window.close();
        }
    });
}
