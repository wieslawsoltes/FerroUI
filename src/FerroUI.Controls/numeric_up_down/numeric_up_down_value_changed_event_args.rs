use ferroui_base::ferro_routed_event_args;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::utilities::Decimal;

/// Provides data for the `ValueChanged` event of a numeric up-down control.
#[derive(Clone)]
pub struct NumericUpDownValueChangedEventArgs {
    base: RoutedEventArgs,
    old_value: Option<Decimal>,
    new_value: Option<Decimal>,
}

ferro_routed_event_args!(NumericUpDownValueChangedEventArgs: RoutedEventArgs);

impl NumericUpDownValueChangedEventArgs {
    /// Creates args for a routed event and a change of the value.
    pub fn new<T: ?Sized>(
        routed_event: Option<&RoutedEvent<T>>,
        old_value: Option<Decimal>,
        new_value: Option<Decimal>,
    ) -> Self {
        let base = RoutedEventArgs::new();
        base.set_routed_event(routed_event);
        Self { base, old_value, new_value }
    }

    /// The value before the change.
    #[inline]
    pub fn old_value(&self) -> Option<Decimal> {
        self.old_value
    }

    /// The value after the change.
    #[inline]
    pub fn new_value(&self) -> Option<Decimal> {
        self.new_value
    }
}
