use crate::ferro_routed_event_args;
use crate::input::InputMethod;
use crate::interactivity::RoutedEventArgs;

/// Provides data for the event that asks the input system to query the
/// focused element for its text input method client again.
#[derive(Clone)]
pub struct TextInputMethodClientRequeryRequestedEventArgs {
    base: RoutedEventArgs,
}

ferro_routed_event_args!(TextInputMethodClientRequeryRequestedEventArgs: RoutedEventArgs);

impl TextInputMethodClientRequeryRequestedEventArgs {
    /// Creates the event args, without a routed event.
    pub fn new() -> Self {
        Self { base: RoutedEventArgs::new() }
    }

    /// Creates the event args for
    /// [`InputMethod::text_input_method_client_requery_requested_event`].
    pub fn with_event() -> Self {
        Self { base: RoutedEventArgs::with_event(InputMethod::text_input_method_client_requery_requested_event()) }
    }
}

impl Default for TextInputMethodClientRequeryRequestedEventArgs {
    fn default() -> Self {
        Self::new()
    }
}
