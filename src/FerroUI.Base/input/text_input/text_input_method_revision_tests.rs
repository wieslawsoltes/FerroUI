//! Revision capture must tolerate custom clients that synchronously publish
//! events or replace their input connection from the optional state getter.
use super::*;
use crate::input::text_input::{
    TextInputMethodComposition, TextInputMethodConnection, TextInputMethodEdit,
    TextInputMethodEditing, TextInputMethodError, TextInputMethodState,
};

type Callback = Box<dyn Fn()>;

struct RevisionClient {
    events: TextInputMethodClientEvents,
    revision: Cell<Option<u64>>,
    text: RefCell<String>,
    selection: Cell<TextSelection>,
    updates: Cell<usize>,
    on_revision: RefCell<Option<Callback>>,
    after_update: RefCell<Option<Callback>>,
}

impl RevisionClient {
    fn new(revision: Option<u64>) -> Rc<Self> {
        Rc::new(Self {
            events: TextInputMethodClientEvents::new(),
            revision: Cell::new(revision),
            text: RefCell::new(String::new()),
            selection: Cell::new(TextSelection::default()),
            updates: Cell::new(0),
            on_revision: RefCell::new(None),
            after_update: RefCell::new(None),
        })
    }

    fn accepted_change(&self) {
        if let Some(revision) = self.revision.get() {
            self.revision.set(Some(revision.wrapping_add(1)));
        } else {
            self.raise_surrounding_text_changed();
        }
    }

    fn arm_revision_callback(self: &Rc<Self>, after_update: bool, callback: Callback) {
        if after_update {
            let callback = RefCell::new(Some(callback));
            let client = self.clone();
            *self.after_update.borrow_mut() = Some(Box::new(move || {
                *client.on_revision.borrow_mut() = callback.borrow_mut().take();
            }));
        } else {
            *self.on_revision.borrow_mut() = Some(callback);
        }
    }
}

impl TextInputMethodClient for RevisionClient {
    fn events(&self) -> &TextInputMethodClientEvents {
        &self.events
    }
    fn input_method_editing(&self) -> Option<&dyn TextInputMethodEditing> {
        Some(self)
    }
    fn input_method_state_revision(&self) -> Option<u64> {
        let callback = self.on_revision.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        self.revision.get()
    }
    fn text_view_visual(&self) -> Option<Ref<Visual>> {
        None
    }
    fn supports_preedit(&self) -> bool {
        true
    }
    fn supports_surrounding_text(&self) -> bool {
        true
    }
    fn surrounding_text(&self) -> String {
        self.text.borrow().clone()
    }
    fn cursor_rectangle(&self) -> Rect {
        Rect::default()
    }
    fn selection(&self) -> TextSelection {
        self.selection.get()
    }
    fn set_selection(&self, selection: TextSelection) {
        self.selection.set(selection);
        self.accepted_change();
    }
}

impl TextInputMethodEditing for RevisionClient {
    fn begin_composition(&self) -> Result<(), TextInputMethodError> {
        Ok(())
    }
    fn update_composition(
        &self,
        text: &str,
        _cursor: Option<i32>,
    ) -> Result<(), TextInputMethodError> {
        *self.text.borrow_mut() = text.to_owned();
        self.updates.set(self.updates.get() + 1);
        self.accepted_change();
        let callback = self.after_update.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        Ok(())
    }
    fn cancel_composition(&self) -> Result<bool, TextInputMethodError> {
        self.text.borrow_mut().clear();
        self.accepted_change();
        Ok(true)
    }
    fn supports_exact_edits(&self) -> bool {
        true
    }
    fn edit_state(&self) -> Result<TextInputMethodState, TextInputMethodError> {
        Ok(TextInputMethodState {
            text: self.surrounding_text(),
            selection: self.selection(),
        })
    }
    fn select_exact(&self, selection: TextSelection) -> Result<(), TextInputMethodError> {
        self.set_selection(selection);
        Ok(())
    }
}

struct RevisionFixture {
    _root: TestRoot,
    owner: Ref<TestControl>,
    client: Rc<RevisionClient>,
    connection: Rc<TextInputMethodConnection>,
}

impl RevisionFixture {
    fn new(revision: Option<u64>) -> Self {
        let owner = TestControl::focusable("revision owner");
        let root = TestRoot::with_child(&owner);
        let client = RevisionClient::new(revision);
        let erased: Rc<dyn TextInputMethodClient> = client.clone();
        let connection = TextInputMethodConnection::new(&owner.clone().upcast(), &erased);
        client.events.set_connection(Some(&connection));
        assert!(connection.is_current());
        Self {
            _root: root,
            owner,
            client,
            connection,
        }
    }
}

#[test]
fn input_method_revision_refreshes_accepted_updates_and_rejects_unpublished_changes() {
    let f = RevisionFixture::new(Some(0));
    let session = f.connection.begin_composition().unwrap();
    session.update("first", None).unwrap();
    assert!(session.is_current());
    session.update("second", None).unwrap();
    assert!(session.is_current());
    session.select_exact(TextSelection::new(3, 1)).unwrap();
    assert!(session.is_current());
    assert_eq!(f.client.revision.get(), Some(3));
    f.client.revision.set(Some(4));
    assert_eq!(
        session.update("stale", None),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(f.client.updates.get(), 2);
    assert_eq!(f.client.surrounding_text(), "second");
}

#[test]
fn input_method_revision_none_retains_event_only_client_compatibility() {
    let f = RevisionFixture::new(None);
    let session = f.connection.begin_composition().unwrap();
    session.update("first", None).unwrap();
    session.update("second", None).unwrap();
    assert!(session.is_current());
    f.client.raise_selection_changed();
    assert_eq!(
        session.update("stale", None),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(f.client.surrounding_text(), "second");
}

#[test]
fn input_method_revision_getter_cannot_capture_a_reentrant_event_as_a_new_baseline() {
    let f = RevisionFixture::new(Some(0));
    let client = f.client.clone();
    *f.client.on_revision.borrow_mut() = Some(Box::new(move || {
        client.raise_selection_changed();
    }));
    assert!(matches!(
        f.connection.begin_composition(),
        Err(TextInputMethodError::StateChanged)
    ));

    for after_update in [false, true] {
        let f = RevisionFixture::new(Some(0));
        let session = f.connection.begin_composition().unwrap();
        let client = f.client.clone();
        f.client.arm_revision_callback(
            after_update,
            Box::new(move || client.raise_selection_changed()),
        );
        assert_eq!(
            session.update("candidate", None),
            Err(TextInputMethodError::StateChanged)
        );
        assert_eq!(f.client.updates.get(), usize::from(after_update));
        assert!(!session.is_current());
    }
}

#[test]
fn input_method_revision_getter_cannot_revive_a_replaced_connection() {
    for after_update in [false, true] {
        let f = RevisionFixture::new(Some(0));
        let session = f.connection.begin_composition().unwrap();
        let client: Rc<dyn TextInputMethodClient> = f.client.clone();
        let replacement = TextInputMethodConnection::new(&f.owner.clone().upcast(), &client);
        let previous = f.connection.clone();
        let next = replacement.clone();
        f.client.arm_revision_callback(
            after_update,
            Box::new(move || {
                previous.invalidate();
                client.events().set_connection(Some(&next));
            }),
        );
        assert_eq!(
            session.update("candidate", None),
            Err(TextInputMethodError::StaleConnection)
        );
        assert_eq!(f.client.updates.get(), usize::from(after_update));
        assert!(!session.is_current());
        assert!(replacement.is_current());
        assert!(Rc::ptr_eq(
            &f.client.input_method_connection().unwrap(),
            &replacement
        ));
    }
}

#[test]
fn input_method_revision_getter_cannot_overwrite_a_newer_composition() {
    for after_update in [false, true] {
        let f = RevisionFixture::new(Some(0));
        let session = f.connection.begin_composition().unwrap();
        let next = Rc::new(RefCell::new(None::<TextInputMethodComposition>));
        let saved = next.clone();
        let connection = f.connection.clone();
        f.client.arm_revision_callback(
            after_update,
            Box::new(move || {
                *saved.borrow_mut() = Some(connection.begin_composition().unwrap());
            }),
        );
        assert_eq!(
            session.update("old", None),
            Err(TextInputMethodError::StaleConnection)
        );
        assert!(!session.is_current());
        let next = next.borrow_mut().take().unwrap();
        assert!(next.is_current());
        next.update("new", None).unwrap();
        assert!(next.is_current());
        assert_eq!(f.client.surrounding_text(), "new");
    }
}

#[test]
fn input_method_exact_request_rejects_a_revision_getter_event_before_dispatch() {
    let f = RevisionFixture::new(Some(0));
    let client = f.client.clone();
    *f.client.on_revision.borrow_mut() = Some(Box::new(move || {
        client.raise_selection_changed();
    }));
    assert_eq!(
        f.connection.apply_edit(
            &TextInputMethodEdit::new(
                TextSelection::new(0, 0),
                "candidate",
                TextSelection::new(9, 9),
            ),
            0,
        ),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(f.client.surrounding_text(), "");
}
