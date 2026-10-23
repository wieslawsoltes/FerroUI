use super::{
    TextInputMethodClient, TextInputMethodClientEvents, TextInputMethodEdit, TextInputMethodState,
};
use crate::input::raw::RawTextInputEventArgs;
use crate::input::{IInputDevice, InputElement, InputManager, InputMethod, KeyboardDevice};
use crate::{Ref, WeakRef};
use std::cell::Cell;
use std::fmt;
use std::rc::{Rc, Weak};

/// Failure of a platform input operation. A rejected commit retains its
/// accepted preview so that the platform can retry or explicitly cancel it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextInputMethodError {
    Unavailable,
    Unsupported,
    StaleConnection,
    StateChanged,
    ReadOnly,
    ReentrantEdit,
    Rejected,
    Intercepted,
    InvalidRange,
}

impl fmt::Display for TextInputMethodError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unavailable => "The input owner or input services are unavailable.",
            Self::Unsupported => "The input client does not support this operation.",
            Self::StaleConnection => "The input connection or composition has been superseded.",
            Self::StateChanged => "The input client's captured state is no longer current.",
            Self::ReadOnly => "The input client is read-only.",
            Self::ReentrantEdit => "An input edit is already being published.",
            Self::Rejected => "The input client rejected the edit.",
            Self::Intercepted => "The input event was intercepted before its owner accepted it.",
            Self::InvalidRange => {
                "An input range is outside the text or splits a UTF-16 surrogate pair."
            }
        })
    }
}

impl std::error::Error for TextInputMethodError {}

/// Optional atomic composition capability of a text input client. Starting a
/// composition preserves selected text; only an accepted update creates a
/// preview. Commits use the normal raw and routed text-input pipeline.
pub trait TextInputMethodEditing {
    fn begin_composition(&self) -> Result<(), TextInputMethodError>;
    fn update_composition(
        &self,
        text: &str,
        cursor: Option<i32>,
    ) -> Result<(), TextInputMethodError>;
    fn cancel_composition(&self) -> Result<bool, TextInputMethodError>;

    /// Exact range edits are optional; legacy preedit clients remain supported.
    fn supports_exact_edits(&self) -> bool {
        false
    }
    fn edit_state(&self) -> Result<TextInputMethodState, TextInputMethodError> {
        Err(TextInputMethodError::Unsupported)
    }
    fn apply_edit(
        &self,
        _edit: &TextInputMethodEdit,
        _composing: bool,
    ) -> Result<bool, TextInputMethodError> {
        Err(TextInputMethodError::Unsupported)
    }
    fn complete_composition(&self) -> Result<bool, TextInputMethodError> {
        Err(TextInputMethodError::Unsupported)
    }
    fn select_exact(&self, _selection: super::TextSelection) -> Result<(), TextInputMethodError> {
        Err(TextInputMethodError::Unsupported)
    }
}

/// A manager-owned connection to one semantic input owner. Platform adapters
/// capture it when the client is installed. It never redirects to a newer
/// focused element, and retaining it does not retain its owner or client.
pub struct TextInputMethodConnection {
    owner: WeakRef<InputElement>,
    client: Weak<dyn TextInputMethodClient>,
    active: Cell<bool>,
    composition: Cell<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ClientStateVersion {
    events: u64,
    intrinsic: Option<u64>,
    composition: u64,
}

impl TextInputMethodConnection {
    pub(crate) fn new(
        owner: &Ref<InputElement>,
        client: &Rc<dyn TextInputMethodClient>,
    ) -> Rc<Self> {
        Rc::new(Self {
            owner: owner.downgrade(),
            client: Rc::downgrade(client),
            active: Cell::new(true),
            composition: Cell::new(0),
        })
    }

    pub(crate) fn invalidate(&self) {
        self.active.set(false);
        self.composition.set(self.composition.get().wrapping_add(1));
    }

    /// The original input owner, only while this connection remains valid.
    pub fn owner(&self) -> Option<Ref<InputElement>> {
        self.active
            .get()
            .then(|| self.owner.upgrade())
            .flatten()
            .filter(|owner| {
                owner.is_attached_to_visual_tree()
                    && owner.is_effectively_enabled()
                    && InputMethod::get_is_input_method_enabled(owner)
            })
    }

    pub fn is_current(&self) -> bool {
        self.owner().is_some() && self.client.upgrade().is_some()
    }

    fn client(&self) -> Result<Rc<dyn TextInputMethodClient>, TextInputMethodError> {
        if !self.is_current() {
            return Err(TextInputMethodError::StaleConnection);
        }
        self.client
            .upgrade()
            .ok_or(TextInputMethodError::StaleConnection)
    }

    fn state_version(
        &self,
        client: &dyn TextInputMethodClient,
    ) -> Result<ClientStateVersion, TextInputMethodError> {
        if !self.is_current() {
            return Err(TextInputMethodError::StaleConnection);
        }
        let composition = self.composition.get();
        let events = client.events();
        let version = events.state_version();
        let intrinsic = client.input_method_state_revision();
        // A custom client's getter can publish events, replace the connection,
        // or synchronously start another composition. Never capture that newer
        // state as though it belonged to the operation that called the getter.
        if !self.is_current() || self.composition.get() != composition {
            return Err(TextInputMethodError::StaleConnection);
        }
        if events.state_version() != version {
            return Err(TextInputMethodError::StateChanged);
        }
        Ok(ClientStateVersion {
            events: version,
            intrinsic,
            composition,
        })
    }

    fn check_state(
        &self,
        client: &dyn TextInputMethodClient,
        expected: ClientStateVersion,
    ) -> Result<(), TextInputMethodError> {
        if self.state_version(client)? != expected {
            return Err(TextInputMethodError::StateChanged);
        }
        Ok(())
    }

    pub fn supports_composition(&self) -> bool {
        self.client()
            .is_ok_and(|client| client.input_method_editing().is_some())
    }

    pub fn supports_exact_edits(&self) -> bool {
        self.client().is_ok_and(|client| {
            client
                .input_method_editing()
                .is_some_and(TextInputMethodEditing::supports_exact_edits)
        })
    }

    pub fn edit_state(&self) -> Result<TextInputMethodState, TextInputMethodError> {
        let client = self.client()?;
        client
            .input_method_editing()
            .ok_or(TextInputMethodError::Unsupported)?
            .edit_state()
    }

    /// Applies a committed exact edit through the same raw/routed interception
    /// path as ordinary text. The owner's acknowledgement is authoritative.
    pub fn apply_edit(
        self: &Rc<Self>,
        edit: &TextInputMethodEdit,
        timestamp: u64,
    ) -> Result<bool, TextInputMethodError> {
        let client = self.client()?;
        let version = self.state_version(&*client)?;
        let editing = client
            .input_method_editing()
            .ok_or(TextInputMethodError::Unsupported)?;
        if !editing.supports_exact_edits() {
            return Err(TextInputMethodError::Unsupported);
        }
        edit.apply_to(&editing.edit_state()?.text)?;
        self.check_state(&*client, version)?;
        self.dispatch(
            text_for_edit(edit),
            timestamp,
            version,
            CommitAction::Edit(edit.clone()),
        )
    }

    pub fn select_exact(
        &self,
        selection: super::TextSelection,
    ) -> Result<(), TextInputMethodError> {
        let client = self.client()?;
        client
            .input_method_editing()
            .ok_or(TextInputMethodError::Unsupported)?
            .select_exact(selection)?;
        self.client()?;
        Ok(())
    }

    /// Starts a new immutable platform session without deleting the current
    /// selection. Any previous session on this connection is superseded.
    pub fn begin_composition(
        self: &Rc<Self>,
    ) -> Result<TextInputMethodComposition, TextInputMethodError> {
        let client = self.client()?;
        let editing = client
            .input_method_editing()
            .ok_or(TextInputMethodError::Unsupported)?;
        let id = self.composition.get().wrapping_add(1);
        self.composition.set(id);
        editing.begin_composition()?;
        let version = self.state_version(&*client)?;
        if !self.is_current() || self.composition.get() != id {
            return Err(TextInputMethodError::StaleConnection);
        }
        Ok(TextInputMethodComposition(Rc::new(CompositionState {
            connection: self.clone(),
            id,
            version: Cell::new(version),
            finished: Cell::new(false),
        })))
    }

    /// Sends ordinary committed input to a client with the atomic editing
    /// capability. A platform composing text should use its captured
    /// composition's `commit` instead.
    pub fn commit_text(
        self: &Rc<Self>,
        text: &str,
        timestamp: u64,
    ) -> Result<bool, TextInputMethodError> {
        let client = self.client()?;
        client
            .input_method_editing()
            .ok_or(TextInputMethodError::Unsupported)?;
        self.dispatch_commit(text, timestamp, self.state_version(&*client)?)
    }

    fn dispatch_commit(
        self: &Rc<Self>,
        text: &str,
        timestamp: u64,
        version: ClientStateVersion,
    ) -> Result<bool, TextInputMethodError> {
        self.dispatch(text, timestamp, version, CommitAction::Text)
    }

    fn dispatch(
        self: &Rc<Self>,
        text: &str,
        timestamp: u64,
        version: ClientStateVersion,
        action: CommitAction,
    ) -> Result<bool, TextInputMethodError> {
        let owner = self.owner().ok_or(TextInputMethodError::StaleConnection)?;
        let root = owner
            .get_input_root()
            .ok_or(TextInputMethodError::Unavailable)?;
        let keyboard = KeyboardDevice::instance().ok_or(TextInputMethodError::Unavailable)?;
        let manager = InputManager::instance().ok_or(TextInputMethodError::Unavailable)?;
        let commit = Rc::new(TextInputMethodCommit {
            connection: self.clone(),
            version,
            result: Cell::new(None),
            action,
        });
        let device: Rc<dyn IInputDevice> = keyboard;
        let event = Rc::new(RawTextInputEventArgs::for_input_method(
            device,
            timestamp,
            root,
            text,
            commit.clone(),
        ));
        manager.process_input(event);
        commit
            .result
            .get()
            .unwrap_or(Err(TextInputMethodError::Intercepted))
    }
}

/// The accepted input request attached to raw and routed text input. The owner
/// checks `is_current` immediately before editing, then acknowledges the real
/// result; an application handling the event does not imply an accepted edit.
pub struct TextInputMethodCommit {
    connection: Rc<TextInputMethodConnection>,
    version: ClientStateVersion,
    result: Cell<Option<Result<bool, TextInputMethodError>>>,
    action: CommitAction,
}

enum CommitAction {
    Text,
    Edit(TextInputMethodEdit),
    CompleteComposition,
}

fn text_for_edit(edit: &TextInputMethodEdit) -> &str {
    &edit.text
}

impl TextInputMethodCommit {
    pub fn edit(&self) -> Option<&TextInputMethodEdit> {
        match &self.action {
            CommitAction::Edit(edit) => Some(edit),
            _ => None,
        }
    }
    pub fn completes_composition(&self) -> bool {
        matches!(self.action, CommitAction::CompleteComposition)
    }
    pub fn owner(&self) -> Option<Ref<InputElement>> {
        self.connection.owner()
    }

    pub fn is_current(&self) -> bool {
        self.connection
            .client()
            .is_ok_and(|client| self.connection.check_state(&*client, self.version).is_ok())
    }

    pub fn complete(&self, result: Result<bool, TextInputMethodError>) {
        if self.result.get().is_none() {
            self.result.set(Some(result));
        }
    }
}

struct CompositionState {
    connection: Rc<TextInputMethodConnection>,
    id: u64,
    version: Cell<ClientStateVersion>,
    finished: Cell<bool>,
}

/// A platform composition bound to one connection and one accepted client state.
/// Clones refer to the same session. Late callbacks fail after reset, focus
/// handoff, external edits, or a newer native composition.
#[derive(Clone)]
pub struct TextInputMethodComposition(Rc<CompositionState>);

impl TextInputMethodComposition {
    fn check_lifetime(&self) -> Result<(), TextInputMethodError> {
        if self.0.finished.get() || self.0.connection.composition.get() != self.0.id {
            return Err(TextInputMethodError::StaleConnection);
        }
        Ok(())
    }

    fn client(&self) -> Result<Rc<dyn TextInputMethodClient>, TextInputMethodError> {
        self.check_lifetime()?;
        let client = self.0.connection.client()?;
        let version = self.0.version.get();
        self.0.connection.check_state(&*client, version)?;
        self.check_lifetime()?;
        if self.0.version.get() != version {
            return Err(TextInputMethodError::StateChanged);
        }
        Ok(client)
    }

    pub fn is_current(&self) -> bool {
        self.client().is_ok()
    }

    fn finish_operation(
        &self,
        client: &dyn TextInputMethodClient,
    ) -> Result<(), TextInputMethodError> {
        self.check_lifetime()?;
        let version = self.0.connection.state_version(client)?;
        self.check_lifetime()?;
        self.0.version.set(version);
        Ok(())
    }

    pub fn update(&self, text: &str, cursor: Option<i32>) -> Result<(), TextInputMethodError> {
        let client = self.client()?;
        client
            .input_method_editing()
            .ok_or(TextInputMethodError::Unsupported)?
            .update_composition(text, cursor)?;
        self.finish_operation(&*client)
    }

    /// Applies the platform's delta to the current preview without committing
    /// history. The resulting selection is part of this operation.
    pub fn apply_edit(&self, edit: &TextInputMethodEdit) -> Result<bool, TextInputMethodError> {
        let client = self.client()?;
        let result = client
            .input_method_editing()
            .ok_or(TextInputMethodError::Unsupported)?
            .apply_edit(edit, true)?;
        self.finish_operation(&*client)?;
        Ok(result)
    }

    pub fn select_exact(
        &self,
        selection: super::TextSelection,
    ) -> Result<(), TextInputMethodError> {
        let client = self.client()?;
        client
            .input_method_editing()
            .ok_or(TextInputMethodError::Unsupported)?
            .select_exact(selection)?;
        self.finish_operation(&*client)
    }

    /// Commits exactly the accepted preview, including empty deletions and
    /// arbitrary directed selection, as one undo transaction.
    pub fn complete(&self, timestamp: u64) -> Result<bool, TextInputMethodError> {
        let client = self.client()?;
        let result = self.0.connection.dispatch(
            "",
            timestamp,
            self.0.version.get(),
            CommitAction::CompleteComposition,
        )?;
        self.finish_operation(&*client)?;
        self.0.finished.set(true);
        Ok(result)
    }

    pub fn commit(&self, text: &str, timestamp: u64) -> Result<bool, TextInputMethodError> {
        let client = self.client()?;
        let result = self
            .0
            .connection
            .dispatch_commit(text, timestamp, self.0.version.get())?;
        self.finish_operation(&*client)?;
        self.0.finished.set(true);
        Ok(result)
    }

    pub fn cancel(&self) -> Result<bool, TextInputMethodError> {
        let client = self.client()?;
        let result = client
            .input_method_editing()
            .ok_or(TextInputMethodError::Unsupported)?
            .cancel_composition()?;
        self.finish_operation(&*client)?;
        self.0.finished.set(true);
        Ok(result)
    }
}

impl TextInputMethodClientEvents {
    pub(crate) fn set_connection(&self, connection: Option<&Rc<TextInputMethodConnection>>) {
        *self.connection.borrow_mut() = connection.map(Rc::downgrade);
    }

    pub(crate) fn connection(&self) -> Option<Rc<TextInputMethodConnection>> {
        self.connection.borrow().as_ref().and_then(Weak::upgrade)
    }
}
