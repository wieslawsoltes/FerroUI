use ferroui_base::ferro_routed_event_args;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::{FerroObject, Nullable};

/// Provides data specific to a `TextBox::text_changed` event.
#[derive(Clone, Default)]
pub struct TextChangedEventArgs {
    base: RoutedEventArgs,
}

ferro_routed_event_args!(TextChangedEventArgs: RoutedEventArgs);

impl TextChangedEventArgs {
    /// Initializes a new instance of the `TextChangedEventArgs` class.
    ///
    /// `routed_event` is the routed event associated with these event args.
    pub fn with_event<T: ?Sized>(routed_event: &RoutedEvent<T>) -> Self {
        Self { base: RoutedEventArgs::with_event(routed_event) }
    }

    /// Initializes a new instance of the `TextChangedEventArgs` class.
    ///
    /// `routed_event` is the routed event associated with these event args,
    /// `source` the source object that raised the routed event.
    pub fn with_event_and_source<T: ?Sized>(
        routed_event: &RoutedEvent<T>,
        source: impl Into<Nullable<FerroObject>>,
    ) -> Self {
        Self { base: RoutedEventArgs::with_event_and_source(routed_event, source) }
    }
}
