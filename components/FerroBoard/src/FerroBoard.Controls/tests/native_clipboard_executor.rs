#[path = "support/headless.rs"]
mod headless;
#[path = "../../../tests/support/local_future.rs"]
mod native_future;
use ferroboard_controls::OnScreenKeyboard;
use ferroboard_core::{
    clipboard::*,
    editor::{ITextInputTarget, TextBufferTarget},
    keys::{KeyCode, KeyType},
    layouts::TextKeyData,
    text::Utf16Text,
};
use ferroui_base::threading::Dispatcher;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Default)]
struct Pending {
    read: native_future::Completion<Option<Utf16Text>>,
    calls: Cell<usize>,
}
impl IClipboardService for Pending {
    fn get_text_async(&self) -> ClipboardFuture<Option<Utf16Text>> {
        self.calls.set(self.calls.get() + 1);
        self.read.future()
    }
    fn set_text_async(&self, _: Option<Utf16Text>) -> ClipboardFuture<()> {
        Box::pin(std::future::ready(Ok(())))
    }
}
#[test]
fn native_dispatcher_resumes_clipboard_job_without_retaining_keyboard_owner() {
    headless::run(|| {
        let keyboard = OnScreenKeyboard::new(None).unwrap();
        let engine = keyboard.engine();
        engine.settings().set_glide_typing(false);
        engine.settings().set_show_suggestions(false);
        engine.settings().set_auto_capitalization(false);
        engine.settings().set_clipboard_history_enabled(false);
        assert!(engine.clipboard_spawner().is_some());
        let target = Rc::new(TextBufferTarget::default());
        keyboard.attach(target.clone()).unwrap();
        let provider = Rc::new(Pending::default());
        engine.set_clipboard_service(provider.clone());
        engine
            .input_key(
                TextKeyData::function(KeyCode::ClipboardPaste, KeyType::Function, None).as_ref(),
            )
            .unwrap();
        assert_eq!(provider.calls.get(), 1);
        assert!(target.text().is_empty());
        let weak = keyboard.downgrade();
        drop(keyboard);
        assert!(
            weak.upgrade().is_none(),
            "dispatcher hook must not capture the native keyboard"
        );
        provider.read.complete(Some("resumed".into()));
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(target.text(), Utf16Text::from("resumed"));
        engine.dispose().unwrap();
    });
}
#[test]
fn additional_keyboard_does_not_replace_an_explicit_clipboard_spawner() {
    headless::run(|| {
        let first = OnScreenKeyboard::new(None).unwrap();
        let engine = first.engine();
        let jobs = Rc::new(RefCell::new(Vec::<ClipboardJob>::new()));
        let queue = jobs.clone();
        let custom: ClipboardSpawner = Rc::new(move |job| queue.borrow_mut().push(job));
        engine.set_clipboard_spawner(Some(custom.clone()));
        let second = OnScreenKeyboard::new(Some(engine.clone())).unwrap();
        assert!(Rc::ptr_eq(&custom, &engine.clipboard_spawner().unwrap()));
        second.release_native_subscriptions().unwrap();
        first.release_native_subscriptions().unwrap();
    });
}
