use super::{
    TextInputMethodConnection, TextInputMethodEditing, TextInputMethodError,
    TextInputMethodRangeGeometry,
};
use crate::input::InputElement;
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{Rect, Ref, Visual};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The start and the end of a selection, in UTF-16 code units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextSelection {
    pub start: i32,
    pub end: i32,
}

impl TextSelection {
    pub const fn new(start: i32, end: i32) -> Self {
        Self { start, end }
    }
}

/// An action of the context menu of a text input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ContextMenuAction {
    Copy = 0,
    Cut = 1,
    Paste = 2,
    SelectAll = 3,
}

type Event = Rc<HandlerList<dyn Fn()>>;

fn subscribe(event: &Event, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
    let token = event.add(handler);
    let event = event.clone();
    Disposable::create(move || {
        event.remove(token);
    })
}

fn raise(event: &Event) {
    if event.is_empty() {
        return;
    }
    for (_, handler) in event.snapshot().iter() {
        handler();
    }
}

/// The events of a [`TextInputMethodClient`]: the state every client
/// embeds and returns from [`TextInputMethodClient::events`].
#[derive(Default)]
pub struct TextInputMethodClientEvents {
    pub(super) connection: RefCell<Option<Weak<TextInputMethodConnection>>>,
    state_version: Cell<u64>,
    text_view_visual_changed: Event,
    cursor_rectangle_changed: Event,
    surrounding_text_changed: Event,
    selection_changed: Event,
    reset_requested: Event,
    input_pane_activation_requested: Event,
}

impl TextInputMethodClientEvents {
    pub(crate) fn state_version(&self) -> u64 {
        self.state_version.get()
    }

    fn state_changed(&self) {
        self.state_version
            .set(self.state_version.get().wrapping_add(1));
    }

    /// Creates the events of a client, without any subscription.
    pub fn new() -> Self {
        Self::default()
    }
}

/// The text editing side of a text input method: what a text editing
/// control gives the input method of the platform to work with.
///
/// Implementations embed a [`TextInputMethodClientEvents`] and provide the
/// members describing the text; the events and the methods raising them are
/// provided. Clients are compared by reference.
pub trait TextInputMethodClient {
    /// The events of the client.
    fn events(&self) -> &TextInputMethodClientEvents;

    /// The semantic control that receives input and supplies options/geometry.
    /// Existing clients default to the element queried by the input manager.
    fn input_method_owner(&self) -> Option<Ref<InputElement>> {
        None
    }

    /// Whether a focus destination belongs to this client's active editing UI.
    /// The client must check actual object identity and current ownership.
    fn retains_input_method_focus(&self, _focused: &InputElement) -> bool {
        false
    }

    /// The current manager connection; adapters capture it during `set_client`.
    /// The manager connects only a client that implements
    /// [`input_method_editing`](Self::input_method_editing); for any other client
    /// this is `None`, and the platform backends keep their upstream behaviour.
    fn input_method_connection(&self) -> Option<Rc<TextInputMethodConnection>> {
        self.events().connection()
    }

    /// Atomic composition, when implemented by this editing control. Returning
    /// `Some` opts the client in to the exact-edit connection
    /// ([`TextInputMethodConnection`]): its owner's options are published again when
    /// they change, and a reset gives it a new connection.
    fn input_method_editing(&self) -> Option<&dyn TextInputMethodEditing> {
        None
    }

    /// Optional identity of the current editing state, including changes whose
    /// public text/selection notifications are still deferred. A supporting
    /// client advances this revision for changes to its document, selection or
    /// composition state; reading it must describe the current state.
    ///
    /// This getter is observational: it may update a private cache used to
    /// produce the revision, but must not initiate edits, input operations or
    /// ownership transitions. The connection defensively rejects observable
    /// reentrancy; it cannot identify an unreported edit made by the getter
    /// itself when the getter returns that edit's new revision.
    ///
    /// Input sessions compare both this revision and the public event version.
    /// Existing clients that return `None` continue to use their events alone.
    fn input_method_state_revision(&self) -> Option<u64> {
        None
    }

    /// The visual that's showing the text, or `None` when the client has
    /// none (a text box without a template).
    fn text_view_visual(&self) -> Option<Ref<Visual>>;

    /// Indicates if the text input method client is capable of displaying
    /// non-committed input on the cursor position.
    fn supports_preedit(&self) -> bool;

    /// Indicates if the text input method client supports sending
    /// surrounding text.
    fn supports_surrounding_text(&self) -> bool;

    /// Returns the text around the cursor, usually the current paragraph.
    fn surrounding_text(&self) -> String;

    /// Gets the cursor rectangle relative to the semantic input owner. The
    /// text-view visual is observed for layout changes; it may be a descendant.
    fn cursor_rectangle(&self) -> Rect;

    /// Queries an ordered range of the visible UTF-16 story, including any
    /// composition preview. The result covers the requested range without
    /// changing selection, scrolling, composition or history. Read-only range
    /// endpoints can lie inside a shaped cluster or a surrogate pair: use the
    /// formatter's cluster bounds and insertion-edge rules for those positions.
    ///
    /// `Unsupported` preserves the platform's legacy caret fallback. A client
    /// that implements this capability returns `Unavailable` for hidden or
    /// unformatted text, `InvalidRange` for invalid offsets, and `StateChanged`
    /// for reentrant invalidation; these must never fall back to another caret.
    fn text_range_geometry(
        &self,
        _range: TextSelection,
    ) -> Result<TextInputMethodRangeGeometry, TextInputMethodError> {
        Err(TextInputMethodError::Unsupported)
    }

    /// Gets the selection that is currently active.
    fn selection(&self) -> TextSelection;

    /// Sets the selection that is currently active.
    fn set_selection(&self, value: TextSelection);

    /// Sets the non-committed input string.
    fn set_preedit_text(&self, _preedit_text: Option<&str>) {}

    /// Executes a context menu action.
    fn execute_context_menu_action(&self, _action: ContextMenuAction) {}

    /// Sets the non-committed input string and the cursor offset in that
    /// string.
    fn set_preedit_text_with_cursor(&self, preedit_text: Option<&str>, _cursor_pos: Option<i32>) {
        self.set_preedit_text(preedit_text)
    }

    /// Fires when the text view visual has changed. Disposing the returned
    /// handle unsubscribes.
    fn text_view_visual_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        subscribe(&self.events().text_view_visual_changed, handler)
    }

    /// Fires when the cursor rectangle has changed. Disposing the returned
    /// handle unsubscribes.
    fn cursor_rectangle_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        subscribe(&self.events().cursor_rectangle_changed, handler)
    }

    /// Fires when the surrounding text has changed. Disposing the returned
    /// handle unsubscribes.
    fn surrounding_text_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        subscribe(&self.events().surrounding_text_changed, handler)
    }

    /// Fires when the selection has changed. Disposing the returned handle
    /// unsubscribes.
    fn selection_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        subscribe(&self.events().selection_changed, handler)
    }

    /// Fires when client wants to reset the input state. Disposing the
    /// returned handle unsubscribes.
    fn reset_requested(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        subscribe(&self.events().reset_requested, handler)
    }

    /// Fires when the client requires the input pane (on-screen keyboard)
    /// to be shown. Disposing the returned handle unsubscribes.
    fn input_pane_activation_requested(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        subscribe(&self.events().input_pane_activation_requested, handler)
    }

    /// Raises the event telling that the text view visual has changed.
    fn raise_text_view_visual_changed(&self) {
        raise(&self.events().text_view_visual_changed)
    }

    /// Raises the event telling that the cursor rectangle has changed.
    fn raise_cursor_rectangle_changed(&self) {
        raise(&self.events().cursor_rectangle_changed)
    }

    /// Raises the event telling that the surrounding text has changed.
    fn raise_surrounding_text_changed(&self) {
        self.events().state_changed();
        raise(&self.events().surrounding_text_changed)
    }

    /// Raises the event telling that the selection has changed.
    fn raise_selection_changed(&self) {
        self.events().state_changed();
        raise(&self.events().selection_changed)
    }

    /// Raises the event asking for the input pane to be shown.
    fn raise_input_pane_activation_requested(&self) {
        raise(&self.events().input_pane_activation_requested)
    }

    /// Asks for the input state to be reset.
    fn request_reset(&self) {
        raise(&self.events().reset_requested)
    }
}
