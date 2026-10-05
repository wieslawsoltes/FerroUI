use super::TextInputMethodClient;
use crate::ferro_routed_event_args;
use crate::interactivity::RoutedEventArgs;
use std::cell::RefCell;
use std::rc::Rc;

/// Provides data for the event the input system raises on the focused
/// element to find its text input method client.
#[derive(Clone)]
pub struct TextInputMethodClientRequestedEventArgs {
    base: RoutedEventArgs,
    client: Rc<RefCell<Option<Rc<dyn TextInputMethodClient>>>>,
}

ferro_routed_event_args!(TextInputMethodClientRequestedEventArgs: RoutedEventArgs);

impl TextInputMethodClientRequestedEventArgs {
    /// Creates the event args, without a routed event and without a
    /// client.
    pub fn new() -> Self {
        Self { base: RoutedEventArgs::new(), client: Rc::new(RefCell::new(None)) }
    }

    /// The text input method client of the element. A text editing control
    /// handling the event sets it.
    pub fn client(&self) -> Option<Rc<dyn TextInputMethodClient>> {
        self.client.borrow().clone()
    }

    /// Sets the text input method client of the element.
    pub fn set_client(&self, value: Option<Rc<dyn TextInputMethodClient>>) {
        let old = self.client.replace(value);
        drop(old);
    }
}

impl Default for TextInputMethodClientRequestedEventArgs {
    fn default() -> Self {
        Self::new()
    }
}
