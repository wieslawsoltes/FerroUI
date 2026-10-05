use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{Rect, Ref, Visual};
use std::rc::Rc;

/// The start and the end of a selection, in characters.
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
    text_view_visual_changed: Event,
    cursor_rectangle_changed: Event,
    surrounding_text_changed: Event,
    selection_changed: Event,
    reset_requested: Event,
    input_pane_activation_requested: Event,
}

impl TextInputMethodClientEvents {
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

    /// The visual that's showing the text.
    fn text_view_visual(&self) -> Ref<Visual>;

    /// Indicates if the text input method client is capable of displaying
    /// non-committed input on the cursor position.
    fn supports_preedit(&self) -> bool;

    /// Indicates if the text input method client supports sending
    /// surrounding text.
    fn supports_surrounding_text(&self) -> bool;

    /// Returns the text around the cursor, usually the current paragraph.
    fn surrounding_text(&self) -> String;

    /// Gets the cursor rectangle relative to the text view visual.
    fn cursor_rectangle(&self) -> Rect;

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
        raise(&self.events().surrounding_text_changed)
    }

    /// Raises the event telling that the selection has changed.
    fn raise_selection_changed(&self) {
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
