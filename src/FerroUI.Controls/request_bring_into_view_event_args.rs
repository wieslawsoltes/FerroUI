use std::cell::Cell;
use std::rc::Rc;

use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::{ferro_routed_event_args, Rect, Ref, Visual};

/// Provides data for the `RequestBringIntoView` event.
#[derive(Clone, Default)]
pub struct RequestBringIntoViewEventArgs {
    base: RoutedEventArgs,
    /// The target object that should be brought into view.
    pub target_object: Option<Ref<Visual>>,
    target_rect: Rc<Cell<Rect>>,
}

ferro_routed_event_args!(RequestBringIntoViewEventArgs: RoutedEventArgs);

impl RequestBringIntoViewEventArgs {
    /// Creates args with no routed event.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates args for a routed event.
    pub fn with_event<T: ?Sized>(routed_event: &RoutedEvent<T>) -> Self {
        Self { base: RoutedEventArgs::with_event(routed_event), ..Self::default() }
    }

    /// The rectangle in the target object's coordinate space that should be
    /// brought into view.
    pub fn target_rect(&self) -> Rect {
        self.target_rect.get()
    }

    /// Sets the rectangle in the target object's coordinate space that should
    /// be brought into view. Handlers may adjust it while the event routes.
    pub fn set_target_rect(&self, value: Rect) {
        self.target_rect.set(value)
    }
}
