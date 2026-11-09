#[path = "../../../tests/support/local_future.rs"]
mod native_future;
use ferroboard_core::{
    clipboard::*,
    editor::*,
    engine::*,
    keys::*,
    layouts::{KeyboardResources, TextKeyData},
    settings::KeyboardSettings,
    text::Utf16Text,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
#[derive(Default)]
struct Pending {
    read: native_future::Completion<Option<Utf16Text>>,
    write: native_future::Completion<()>,
    reads: Cell<usize>,
    writes: RefCell<Vec<Option<Utf16Text>>>,
}
impl IClipboardService for Pending {
    fn get_text_async(&self) -> ClipboardFuture<Option<Utf16Text>> {
        self.reads.set(self.reads.get() + 1);
        self.read.future()
    }
    fn set_text_async(&self, text: Option<Utf16Text>) -> ClipboardFuture<()> {
        self.writes.borrow_mut().push(text);
        self.write.future()
    }
}
fn engine() -> (Rc<KeyboardEngine>, Rc<TextBufferTarget>) {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_glide_typing(false);
    settings.set_show_suggestions(false);
    settings.set_auto_capitalization(false);
    let engine = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap();
    let target = Rc::new(TextBufferTarget::new("copy me".into(), None));
    engine.attach(target.clone()).unwrap();
    (engine, target)
}
fn key(engine: &KeyboardEngine, code: i32) -> EngineResult<()> {
    engine.input_key(TextKeyData::function(code, KeyType::Function, None).as_ref())
}
#[test]
fn ready_in_memory_key_commands_stay_synchronous_without_an_executor() {
    let (engine, target) = engine();
    assert!(engine.clipboard_spawner().is_none());
    target.select(0, 4);
    key(&engine, KeyCode::ClipboardCopy).unwrap();
    assert_eq!(
        native_future::ready(engine.clipboard_service().get_text_async()).unwrap(),
        Some("copy".into())
    );
    key(&engine, KeyCode::ClipboardCut).unwrap();
    assert_eq!(target.text(), Utf16Text::from(" me"));
    key(&engine, KeyCode::ClipboardPaste).unwrap();
    assert_eq!(target.text(), Utf16Text::from("copy me"));
    engine.dispose().unwrap();
}
#[test]
fn public_future_is_lazy_and_drop_cancels_pending_edit_without_a_task_wrapper() {
    let (engine, target) = engine();
    let provider = Rc::new(Pending::default());
    engine.set_clipboard_service(provider.clone());
    target.select(0, 4);
    let mut operation = engine.cut_async();
    assert!(provider.writes.borrow().is_empty());
    assert!(native_future::poll(operation.as_mut()).is_pending());
    assert_eq!(*provider.writes.borrow(), [Some("copy".into())]);
    drop(operation);
    provider.write.complete(());
    assert_eq!(target.text(), Utf16Text::from("copy me"));
    assert!(engine.clipboard_history().primary().is_none());
    engine.dispose().unwrap();
}
#[test]
fn pending_key_command_requires_an_explicit_native_spawner() {
    let (engine, target) = engine();
    let provider = Rc::new(Pending::default());
    engine.set_clipboard_service(provider.clone());
    assert!(matches!(
        key(&engine, KeyCode::ClipboardPaste),
        Err(EngineError::ClipboardRuntimeRequired)
    ));
    assert_eq!(provider.reads.get(), 1);
    provider.read.complete(Some("late".into()));
    assert_eq!(target.text(), Utf16Text::from("copy me"));
    engine.dispose().unwrap();
}
#[test]
fn injected_spawner_resumes_after_initial_poll_and_uses_current_private_target() {
    let (engine, _) = engine();
    let provider = Rc::new(Pending::default());
    engine.set_clipboard_service(provider.clone());
    let jobs = Rc::new(RefCell::new(Vec::<ClipboardJob>::new()));
    let queue = jobs.clone();
    engine.set_clipboard_spawner(Some(Rc::new(move |job| queue.borrow_mut().push(job))));
    key(&engine, KeyCode::ClipboardPaste).unwrap();
    assert_eq!(
        provider.reads.get(),
        1,
        "first source prefix must execute before key dispatch returns"
    );
    let private = Rc::new(TextBufferTarget::new(
        "private".into(),
        Some(Arc::new(InputAttributes::from_values(
            InputAttributesValues {
                is_private: true,
                ..Default::default()
            },
        ))),
    ));
    private.select(0, 7);
    engine.attach(private.clone()).unwrap();
    provider.read.complete(Some("pasted".into()));
    let job = jobs.borrow_mut().pop().unwrap();
    native_future::ready(job);
    assert_eq!(private.text(), Utf16Text::from("pasted"));
    assert!(engine.clipboard_history().primary().is_none());
    engine.dispose().unwrap();
}
#[test]
fn native_result_errors_remain_observable_for_key_jobs_and_awaiting_callers() {
    let (engine, _) = engine();
    let provider = Rc::new(Pending::default());
    let denied: ClipboardServiceError = Arc::new(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied",
    ));
    provider.read.fail(denied.clone());
    engine.set_clipboard_service(provider);
    let errors = Rc::new(Cell::new(0));
    let count = errors.clone();
    let original = denied.clone();
    engine.clipboard_failed().add(Rc::new(move |error| {
        if let EngineError::ClipboardService(error) = error {
            assert!(Arc::ptr_eq(error, &original));
        } else {
            panic!("{error:?}");
        }
        count.set(count.get() + 1);
    }));
    key(&engine, KeyCode::ClipboardPaste).unwrap();
    assert_eq!(errors.get(), 1);
    assert!(
        matches!(native_future::ready(engine.paste_async()),Err(EngineError::ClipboardService(error)) if Arc::ptr_eq(&error,&denied))
    );
    assert_eq!(
        errors.get(),
        1,
        "awaiting caller owns its Result; no duplicate failure event"
    );
    engine.dispose().unwrap();
}
