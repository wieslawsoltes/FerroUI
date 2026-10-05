use crate::ScrollViewer;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::{ferro_routed_event_args, Vector};

/// Describes a change in scrolling state.
#[derive(Clone)]
pub struct ScrollChangedEventArgs {
    base: RoutedEventArgs,
    extent_delta: Vector,
    offset_delta: Vector,
    viewport_delta: Vector,
}

ferro_routed_event_args!(ScrollChangedEventArgs: RoutedEventArgs);

impl ScrollChangedEventArgs {
    /// Creates args for the `ScrollChanged` event of the scroll viewer.
    pub fn new(extent_delta: Vector, offset_delta: Vector, viewport_delta: Vector) -> Self {
        Self::with_event(
            Some(ScrollViewer::scroll_changed_event()),
            extent_delta,
            offset_delta,
            viewport_delta,
        )
    }

    /// Creates args for a routed event.
    pub fn with_event<T: ?Sized>(
        routed_event: Option<&RoutedEvent<T>>,
        extent_delta: Vector,
        offset_delta: Vector,
        viewport_delta: Vector,
    ) -> Self {
        let base = match routed_event {
            Some(routed_event) => RoutedEventArgs::with_event(routed_event),
            None => RoutedEventArgs::new(),
        };
        Self {
            base,
            extent_delta,
            offset_delta,
            viewport_delta,
        }
    }

    /// Gets the change to the value of the `Extent` of the scroll viewer.
    #[inline]
    pub fn extent_delta(&self) -> Vector {
        self.extent_delta
    }

    /// Gets the change to the value of the `Offset` of the scroll viewer.
    #[inline]
    pub fn offset_delta(&self) -> Vector {
        self.offset_delta
    }

    /// Gets the change to the value of the `Viewport` of the scroll viewer.
    #[inline]
    pub fn viewport_delta(&self) -> Vector {
        self.viewport_delta
    }
}
