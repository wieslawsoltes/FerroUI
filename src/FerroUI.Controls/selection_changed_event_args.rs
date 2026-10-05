use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::{ferro_routed_event_args, BoxedValue};

/// Provides data for the `SelectionChanged` event.
#[derive(Clone, Default)]
pub struct SelectionChangedEventArgs {
    base: RoutedEventArgs,
    added_items: Vec<Option<BoxedValue>>,
    removed_items: Vec<Option<BoxedValue>>,
}

ferro_routed_event_args!(SelectionChangedEventArgs: RoutedEventArgs);

impl SelectionChangedEventArgs {
    /// Initializes a new instance of the [`SelectionChangedEventArgs`]
    /// class.
    ///
    /// `routed_event` is the event being raised, `removed_items` the items
    /// removed from the selection and `added_items` the items added to the
    /// selection.
    pub fn new<T: ?Sized>(
        routed_event: Option<&RoutedEvent<T>>,
        removed_items: Vec<Option<BoxedValue>>,
        added_items: Vec<Option<BoxedValue>>,
    ) -> Self {
        let base = match routed_event {
            Some(routed_event) => RoutedEventArgs::with_event(routed_event),
            None => RoutedEventArgs::new(),
        };
        Self { base, added_items, removed_items }
    }

    /// Gets the items that were added to the selection.
    pub fn added_items(&self) -> &[Option<BoxedValue>] {
        &self.added_items
    }

    /// Gets the items that were removed from the selection.
    pub fn removed_items(&self) -> &[Option<BoxedValue>] {
        &self.removed_items
    }
}
