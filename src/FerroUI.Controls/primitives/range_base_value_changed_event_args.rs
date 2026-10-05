use ferroui_base::ferro_routed_event_args;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::{FerroObject, Nullable};

/// Provides data specific to a `RangeBase::value_changed` event.
#[derive(Clone, Default)]
pub struct RangeBaseValueChangedEventArgs {
    base: RoutedEventArgs,
    old_value: f64,
    new_value: f64,
}

ferro_routed_event_args!(RangeBaseValueChangedEventArgs: RoutedEventArgs);

impl RangeBaseValueChangedEventArgs {
    /// Creates the args from the old and the new value of the range value
    /// property and the routed event associated with the args.
    pub fn new<T: ?Sized>(old_value: f64, new_value: f64, routed_event: Option<&RoutedEvent<T>>) -> Self {
        let base = match routed_event {
            Some(routed_event) => RoutedEventArgs::with_event(routed_event),
            None => RoutedEventArgs::new(),
        };
        Self { base, old_value, new_value }
    }

    /// Creates the args from the old and the new value of the range value
    /// property, the routed event associated with the args and the source
    /// object that raised the routed event.
    pub fn with_source<T: ?Sized>(
        old_value: f64,
        new_value: f64,
        routed_event: Option<&RoutedEvent<T>>,
        source: impl Into<Nullable<FerroObject>>,
    ) -> Self {
        let args = Self::new(old_value, new_value, routed_event);
        args.set_source(source);
        args
    }

    /// The old value of the range value property.
    pub fn old_value(&self) -> f64 {
        self.old_value
    }

    /// The new value of the range value property.
    pub fn new_value(&self) -> f64 {
        self.new_value
    }
}
