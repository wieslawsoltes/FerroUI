use super::text_input::TextInputMethodCommit;
use crate::ferro_routed_event_args;
use crate::interactivity::RoutedEventArgs;
use std::rc::Rc;

/// Provides information specific to a text input event.
#[derive(Clone, Default)]
pub struct TextInputEventArgs {
    base: RoutedEventArgs,
    pub(crate) input_method_commit: Option<Rc<TextInputMethodCommit>>,
    pub(crate) from_input_method: bool,

    /// The text that was input.
    pub text: Option<String>,
}

ferro_routed_event_args!(TextInputEventArgs: RoutedEventArgs);

impl TextInputEventArgs {
    /// Indicates an input-method commit even after the platform cleared preedit.
    pub fn is_from_input_method(&self) -> bool {
        self.from_input_method
    }

    /// The captured platform commit. A handling editor acknowledges its real result.
    pub fn input_method_commit(&self) -> Option<&Rc<TextInputMethodCommit>> {
        self.input_method_commit.as_ref()
    }

    /// Creates args with no text and no routed event.
    pub fn new() -> Self {
        Self::default()
    }
}
