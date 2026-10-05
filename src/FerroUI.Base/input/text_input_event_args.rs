use crate::ferro_routed_event_args;
use crate::interactivity::RoutedEventArgs;

/// Provides information specific to a text input event.
#[derive(Clone, Default)]
pub struct TextInputEventArgs {
    base: RoutedEventArgs,

    /// The text that was input.
    pub text: Option<String>,
}

ferro_routed_event_args!(TextInputEventArgs: RoutedEventArgs);

impl TextInputEventArgs {
    /// Creates args with no text and no routed event.
    pub fn new() -> Self {
        Self::default()
    }
}
