use super::{IKeyModifiersEventArgs, InputElement, KeyModifiers, NavigationMethod};
use crate::interactivity::{RoutedEvent, RoutedEventArgs};
use crate::{ferro_routed_event_args, Ref};

/// Holds arguments for the `GotFocus` and `LostFocus` events.
#[derive(Clone)]
pub struct FocusChangedEventArgs {
    base: RoutedEventArgs,

    /// The element that focus has moved to.
    pub new_focused_element: Option<Ref<InputElement>>,

    /// The element that previously had focus.
    pub old_focused_element: Option<Ref<InputElement>>,

    /// Indicates how the change in focus occurred.
    pub navigation_method: NavigationMethod,

    /// Any key modifiers active at the time of focus.
    pub key_modifiers: KeyModifiers,
}

ferro_routed_event_args!(FocusChangedEventArgs: RoutedEventArgs);

impl FocusChangedEventArgs {
    /// Creates args for a routed event.
    pub fn new<T: ?Sized>(routed_event: &RoutedEvent<T>) -> Self {
        Self {
            base: RoutedEventArgs::with_event(routed_event),
            new_focused_element: None,
            old_focused_element: None,
            navigation_method: NavigationMethod::Unspecified,
            key_modifiers: KeyModifiers::NONE,
        }
    }
}

impl IKeyModifiersEventArgs for FocusChangedEventArgs {
    fn key_modifiers(&self) -> KeyModifiers {
        self.key_modifiers
    }
}
