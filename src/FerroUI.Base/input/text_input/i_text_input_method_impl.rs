use super::{TextInputMethodClient, TextInputOptions};
use crate::Rect;
use std::rc::Rc;

/// The input method of the platform, as seen by an input root.
///
/// This contract is not stable: it follows the needs of the platform
/// backends.
pub trait ITextInputMethodImpl {
    /// Sets the text editing client the input method talks to, or detaches
    /// it from the current one.
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>);

    /// Sets the rectangle of the text cursor, in the coordinates of the
    /// root.
    fn set_cursor_rect(&self, rect: Rect);

    /// Sets the options of the text input of the current client.
    fn set_options(&self, options: &TextInputOptions);

    /// Resets the state of the input method.
    fn reset(&self);
}
