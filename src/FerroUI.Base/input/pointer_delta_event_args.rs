use super::{IPointer, KeyModifiers, PointerEventArgs, PointerPointProperties};
use crate::interactivity::RoutedEvent;
use crate::{ferro_routed_event_args, FerroObject, Nullable, Point, Vector, Visual};
use std::rc::Rc;

/// Provides data for the touchpad gesture events (magnify, rotate, swipe).
#[derive(Clone)]
pub struct PointerDeltaEventArgs {
    base: PointerEventArgs,
    delta: Vector,
}

ferro_routed_event_args!(PointerDeltaEventArgs: PointerEventArgs);

impl PointerDeltaEventArgs {
    /// Creates pointer delta event args.
    #[allow(clippy::too_many_arguments)]
    pub fn new<T: ?Sized>(
        routed_event: Option<&RoutedEvent<T>>,
        source: impl Into<Nullable<FerroObject>>,
        pointer: Rc<dyn IPointer>,
        root_visual: &Visual,
        root_visual_position: Point,
        timestamp: u64,
        properties: PointerPointProperties,
        modifiers: KeyModifiers,
        delta: Vector,
    ) -> Self {
        Self {
            base: PointerEventArgs::new(
                routed_event,
                source,
                pointer,
                Some(root_visual),
                root_visual_position,
                timestamp,
                properties,
                modifiers,
            ),
            delta,
        }
    }

    /// The amount of the gesture.
    #[inline]
    pub fn delta(&self) -> Vector {
        self.delta
    }
}
