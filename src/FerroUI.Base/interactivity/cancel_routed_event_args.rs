use super::{RoutedEvent, RoutedEventArgs};
use crate::{ferro_routed_event_args, FerroObject, Nullable};
use std::cell::Cell;
use std::rc::Rc;

/// Provides state information and data specific to a cancelable routed event.
#[derive(Clone, Default)]
pub struct CancelRoutedEventArgs {
    base: RoutedEventArgs,
    cancel: Rc<Cell<bool>>,
}

ferro_routed_event_args!(CancelRoutedEventArgs: RoutedEventArgs);

impl CancelRoutedEventArgs {
    /// Creates args with no routed event.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates args for a routed event.
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

    /// Whether the routed event should be canceled.
    #[inline]
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Sets whether the routed event should be canceled.
    #[inline]
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value)
    }
}
