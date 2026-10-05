use super::{IKeyModifiersEventArgs, IPointer, KeyModifiers, PointerEventArgs};
use crate::interactivity::{RoutedEvent, RoutedEventArgs};
use crate::{ferro_routed_event_args, Point, Visual};
use std::rc::Rc;

/// Provides data for the tapped, double-tapped and right-tapped events.
#[derive(Clone)]
pub struct TappedEventArgs {
    base: RoutedEventArgs,
    last_pointer_event_args: PointerEventArgs,
}

ferro_routed_event_args!(TappedEventArgs: RoutedEventArgs);

impl TappedEventArgs {
    /// Creates tapped event args from the pointer event that completed the
    /// gesture.
    pub fn new<T: ?Sized>(routed_event: Option<&RoutedEvent<T>>, last_pointer_event_args: &PointerEventArgs) -> Self {
        let base = RoutedEventArgs::new();
        base.set_routed_event(routed_event);
        Self { base, last_pointer_event_args: last_pointer_event_args.clone() }
    }

    /// The pointer that performed the gesture.
    pub fn pointer(&self) -> &Rc<dyn IPointer> {
        self.last_pointer_event_args.pointer()
    }

    /// The key modifiers held down when the gesture completed.
    pub fn key_modifiers(&self) -> KeyModifiers {
        self.last_pointer_event_args.key_modifiers()
    }

    /// The platform time stamp of the pointer event that completed the
    /// gesture.
    pub fn timestamp(&self) -> u64 {
        self.last_pointer_event_args.timestamp()
    }

    /// Gets the pointer position relative to a control.
    pub fn get_position(&self, relative_to: Option<&Visual>) -> Point {
        self.last_pointer_event_args.get_position(relative_to)
    }
}

impl IKeyModifiersEventArgs for TappedEventArgs {
    fn key_modifiers(&self) -> KeyModifiers {
        self.last_pointer_event_args.key_modifiers()
    }
}
