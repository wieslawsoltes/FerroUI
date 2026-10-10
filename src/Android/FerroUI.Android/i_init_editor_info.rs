//! What the input method needs of the view: a way to say how the editor is
//! described to the input method of the system, and which connection it is
//! given, the next time the system asks the view for an input connection.

use crate::platform::input::ferro_input_connection::FerroInputConnection;
use crate::platform::input::text_edit_buffer::IInputConnectionTopLevel;
use std::rc::Rc;

/// `android.view.inputmethod.EditorInfo`, as far as the input method fills
/// it. The values are those of a new object of the system: nothing is set.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditorInfo {
    /// `inputType`.
    pub input_type: i32,
    /// `imeOptions`.
    pub ime_options: i32,
    /// `initialCapsMode`.
    pub initial_caps_mode: i32,
    /// `hintLocales`, as language tags; `None` leaves the field as it is.
    pub hint_locales: Option<Vec<String>>,
}

/// Fills the editor info for the top-level of the view and returns the
/// input connection; `None` is no connection.
pub(crate) type InitEditorInfo =
    Rc<dyn Fn(&Rc<dyn IInputConnectionTopLevel>, &mut EditorInfo) -> Option<Rc<FerroInputConnection>>>;

pub(crate) trait IInitEditorInfo {
    fn init_editor_info(&self, init: InitEditorInfo);
}
