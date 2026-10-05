use super::{InputElement, PointerEventArgs, PointerType};
use crate::interactivity::RoutedEventArgs;
use crate::{ferro_routed_event_args, Point};

/// Provides data for the holding event.
#[derive(Clone)]
pub struct HoldingRoutedEventArgs {
    base: RoutedEventArgs,
    holding_state: HoldingState,
    position: Point,
    pointer_type: PointerType,
    pointer_event_args: PointerEventArgs,
}

ferro_routed_event_args!(HoldingRoutedEventArgs: RoutedEventArgs);

impl HoldingRoutedEventArgs {
    pub(crate) fn new(
        holding_state: HoldingState,
        position: Point,
        pointer_type: PointerType,
        pointer_event_args: &PointerEventArgs,
    ) -> Self {
        Self {
            base: RoutedEventArgs::with_event(InputElement::holding_event()),
            holding_state,
            position,
            pointer_type,
            pointer_event_args: pointer_event_args.clone(),
        }
    }

    /// The state of the holding gesture.
    #[inline]
    pub fn holding_state(&self) -> HoldingState {
        self.holding_state
    }

    /// The location of the touch, mouse, or pen/stylus contact.
    #[inline]
    pub fn position(&self) -> Point {
        self.position
    }

    /// The pointer type of the input source.
    #[inline]
    pub fn pointer_type(&self) -> PointerType {
        self.pointer_type
    }

    pub fn pointer_event_args(&self) -> &PointerEventArgs {
        &self.pointer_event_args
    }
}

/// The state of a holding gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HoldingState {
    /// A single contact has been detected and a time threshold is crossed
    /// without the contact being lifted, another contact detected, or
    /// another gesture started.
    Started,
    /// The single contact is lifted.
    Completed,
    /// An additional contact is detected or a subsequent gesture (such as a
    /// slide) is detected.
    Canceled,
}
