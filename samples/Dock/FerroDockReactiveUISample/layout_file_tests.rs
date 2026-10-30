//! Commands exercise real model serialization with controlled asynchronous file outcomes.
use crate::{
    services::{ILayoutFileStorage, LayoutFileContents, LayoutFileFuture},
    App,
};
use ferrodock_model::core::{DockSerializationError, SerializationResult};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
    task::{Poll, Waker},
};

#[derive(Default)]
pub(crate) struct TestLayoutFiles {
    pub(crate) opens: RefCell<VecDeque<SerializationResult<Option<LayoutFileContents>>>>,
    pub(crate) saves: RefCell<VecDeque<SerializationResult<Option<String>>>>,
    pub(crate) written: RefCell<Vec<Vec<u8>>>,
    ready: Rc<Cell<bool>>,
    waker: Rc<RefCell<Option<Waker>>>,
    pub(crate) open_count: Cell<usize>,
}
impl TestLayoutFiles {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            ready: Rc::new(Cell::new(true)),
            ..Self::default()
        })
    }
    pub(crate) fn open_bytes(&self, bytes: Vec<u8>) {
        self.opens
            .borrow_mut()
            .push_back(Ok(Some(LayoutFileContents {
                name: "layout.json".into(),
                bytes,
            })));
    }
    fn resume(&self) {
        self.ready.set(true);
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
        pump();
    }
}
impl ILayoutFileStorage for TestLayoutFiles {
    fn open_layout(&self) -> LayoutFileFuture<Option<LayoutFileContents>> {
        self.open_count.set(self.open_count.get() + 1);
        let mut result = Some(self.opens.borrow_mut().pop_front().unwrap_or(Ok(None)));
        let ready = self.ready.clone();
        let waker = self.waker.clone();
        Box::pin(std::future::poll_fn(move |cx| {
            if ready.get() {
                Poll::Ready(result.take().expect("one completion"))
            } else {
                *waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }))
    }
    fn save_layout(&self, bytes: Vec<u8>) -> LayoutFileFuture<Option<String>> {
        self.written.borrow_mut().push(bytes);
        let result = self.saves.borrow_mut().pop_front().unwrap_or(Ok(None));
        Box::pin(std::future::ready(result))
    }
}
fn pump() {
    for _ in 0..8 {
        ferroui_system::threading::tasks::run_pending_wakes();
    }
}
fn failure(message: &str) -> DockSerializationError {
    DockSerializationError::new(std::io::Error::other(message.to_owned()))
}

#[test]
fn file_commands_roundtrip_standard_stream_bytes_and_original_sample_models() {
    let model = App::create_main_view_model();
    let storage = TestLayoutFiles::new();
    model.attach_layout_files(storage.clone());
    let actions = model.layout_files().unwrap();
    storage
        .saves
        .borrow_mut()
        .push_back(Ok(Some("layout.json".into())));
    assert!(actions.save_layout().can_execute(None));
    actions.save_layout().execute(None);
    pump();
    assert_eq!(actions.status(), "Saved layout.json");
    assert!(!actions.is_busy());
    let bytes = storage.written.borrow()[0].clone();
    assert!(bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    let original = model.layout().unwrap();
    storage.open_bytes(bytes);
    actions.load_layout().execute(None);
    pump();
    assert_eq!(actions.status(), "Loaded layout.json");
    assert!(!Rc::ptr_eq(&original, &model.layout().unwrap()));
    assert!(model
        .save_layout_json()
        .unwrap()
        .contains("CustomDocumentDock"));
    assert!(actions.save_layout().can_execute(None));
    assert!(actions.load_layout().can_execute(None));
}

#[test]
fn file_cancellation_read_parse_and_write_failures_keep_the_current_layout() {
    let model = App::create_main_view_model();
    let storage = TestLayoutFiles::new();
    model.attach_layout_files(storage.clone());
    let actions = model.layout_files().unwrap();
    let original = model.layout().unwrap();
    actions.load_layout().execute(None);
    pump();
    assert_eq!(actions.status(), "Open cancelled.");
    actions.save_layout().execute(None);
    pump();
    assert_eq!(actions.status(), "Save cancelled.");
    storage
        .opens
        .borrow_mut()
        .push_back(Err(failure("Read denied")));
    actions.load_layout().execute(None);
    pump();
    assert_eq!(actions.status(), "Loading layout failed: Read denied");
    storage.open_bytes(b"{".to_vec());
    actions.load_layout().execute(None);
    pump();
    assert!(actions.status().starts_with("Loading layout failed:"));
    storage
        .saves
        .borrow_mut()
        .push_back(Err(failure("Disk full")));
    actions.save_layout().execute(None);
    pump();
    assert_eq!(actions.status(), "Saving layout failed: Disk full");
    assert!(Rc::ptr_eq(&original, &model.layout().unwrap()));
    assert!(!actions.is_busy());
    assert!(actions.load_layout().can_execute(None));
}

#[test]
fn pending_picker_gates_both_commands_and_null_load_clears_the_actual_layout() {
    let model = App::create_main_view_model();
    let storage = TestLayoutFiles::new();
    model.attach_layout_files(storage.clone());
    let actions = model.layout_files().unwrap();
    let original = model.layout().unwrap();
    storage.ready.set(false);
    storage.open_bytes(b"null".to_vec());
    actions.load_layout().execute(None);
    pump();
    assert!(actions.is_busy());
    assert_eq!(actions.status(), "Opening layout…");
    assert!(!actions.save_layout().can_execute(None));
    assert!(!actions.load_layout().can_execute(None));
    actions.load_layout().execute(None);
    actions.save_layout().execute(None);
    pump();
    assert_eq!(storage.open_count.get(), 1);
    assert!(storage.written.borrow().is_empty());
    assert!(Rc::ptr_eq(&original, &model.layout().unwrap()));
    storage.resume();
    assert!(!actions.is_busy());
    assert!(model.layout().is_none());
    assert_eq!(actions.status(), "Loaded layout.json");
    assert!(actions.save_layout().can_execute(None));
    assert_eq!(model.save_layout_json().unwrap(), "null");
}

#[test]
fn file_stream_load_uses_standard_encoding_detection_and_unconfigured_actions_are_disabled() {
    let model = App::create_main_view_model();
    let mut utf16 = vec![0xff, 0xfe];
    for u in "null".encode_utf16() {
        utf16.extend(u.to_le_bytes());
    }
    model.load_layout_bytes(utf16).unwrap();
    assert!(model.layout().is_none());
    let compatibility = crate::view_models::MainWindowViewModel::new();
    assert!(compatibility.layout_files().is_none());
    assert!(compatibility.save_layout_bytes().is_err());
    assert!(compatibility.load_layout_bytes(b"null".to_vec()).is_err());
    compatibility.attach_layout_files(TestLayoutFiles::new());
    let actions = compatibility.layout_files().unwrap();
    assert!(!actions.save_layout().can_execute(None));
    assert!(!actions.load_layout().can_execute(None));
}
