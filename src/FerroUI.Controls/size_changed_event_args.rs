use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::layout::LayoutHelper;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{ferro_routed_event_args, FerroObject, Nullable, Size};

/// Provides data specific to a `SizeChanged` event.
#[derive(Clone, Default)]
pub struct SizeChangedEventArgs {
    base: RoutedEventArgs,
    new_size: Size,
    previous_size: Size,
}

ferro_routed_event_args!(SizeChangedEventArgs: RoutedEventArgs);

impl SizeChangedEventArgs {
    /// Creates args for a routed event.
    pub fn with_event<T: ?Sized>(routed_event: &RoutedEvent<T>) -> Self {
        Self { base: RoutedEventArgs::with_event(routed_event), ..Self::default() }
    }

    /// Creates args for a routed event with a source.
    pub fn with_event_and_source<T: ?Sized>(
        routed_event: &RoutedEvent<T>,
        source: impl Into<Nullable<FerroObject>>,
    ) -> Self {
        Self { base: RoutedEventArgs::with_event_and_source(routed_event, source), ..Self::default() }
    }

    /// Creates args for a routed event with a source and the previous and
    /// new sizes.
    pub fn new<T: ?Sized>(
        routed_event: &RoutedEvent<T>,
        source: impl Into<Nullable<FerroObject>>,
        previous_size: Size,
        new_size: Size,
    ) -> Self {
        Self { base: RoutedEventArgs::with_event_and_source(routed_event, source), new_size, previous_size }
    }

    /// Whether the height of the new size is considered different to the
    /// previous size height.
    ///
    /// This will take into account layout epsilon and will not be true if
    /// both heights are considered equivalent for layout purposes. Remember
    /// there can be small variations in the calculations between layout
    /// cycles due to rounding and precision even when the size has not
    /// otherwise changed.
    pub fn height_changed(&self) -> bool {
        !MathUtilities::are_close_eps(self.new_size.height, self.previous_size.height, LayoutHelper::LAYOUT_EPSILON)
    }

    /// The new size (or bounds) of the object.
    pub fn new_size(&self) -> Size {
        self.new_size
    }

    /// The previous size (or bounds) of the object.
    pub fn previous_size(&self) -> Size {
        self.previous_size
    }

    /// Whether the width of the new size is considered different to the
    /// previous size width.
    ///
    /// See [`height_changed`](Self::height_changed).
    pub fn width_changed(&self) -> bool {
        !MathUtilities::are_close_eps(self.new_size.width, self.previous_size.width, LayoutHelper::LAYOUT_EPSILON)
    }
}
