use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::{ferro_routed_event_args, FerroObject, Nullable};
use std::cell::Cell;
use std::rc::Rc;

/// Provides data for the `Closing` routed event of the drawer page.
///
/// The cancel flag is shared: a clone is another handle to the same args.
#[derive(Clone, Default)]
pub struct DrawerClosingEventArgs {
    base: RoutedEventArgs,
    cancel: Rc<Cell<bool>>,
}

ferro_routed_event_args!(DrawerClosingEventArgs: RoutedEventArgs);

impl DrawerClosingEventArgs {
    /// Creates args with no routed event.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates args for the routed event associated with these event data.
    pub fn with_event<T: ?Sized>(routed_event: &RoutedEvent<T>) -> Self {
        Self { base: RoutedEventArgs::with_event(routed_event), cancel: Rc::new(Cell::new(false)) }
    }

    /// Creates args for a routed event with a source.
    pub fn with_event_and_source<T: ?Sized>(
        routed_event: &RoutedEvent<T>,
        source: impl Into<Nullable<FerroObject>>,
    ) -> Self {
        Self { base: RoutedEventArgs::with_event_and_source(routed_event, source), cancel: Rc::new(Cell::new(false)) }
    }

    /// Whether the closing should be cancelled.
    #[inline]
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Sets whether the closing should be cancelled.
    #[inline]
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value)
    }
}
