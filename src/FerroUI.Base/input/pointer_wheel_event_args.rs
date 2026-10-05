use super::{IPointer, InputElement, KeyModifiers, PointerEventArgs, PointerPointProperties};
use crate::{ferro_routed_event_args, FerroObject, Nullable, Point, Vector, Visual};
use std::rc::Rc;

/// Provides data for the pointer wheel changed event.
#[derive(Clone)]
pub struct PointerWheelEventArgs {
    base: PointerEventArgs,
    delta: Vector,
}

ferro_routed_event_args!(PointerWheelEventArgs: PointerEventArgs);

impl PointerWheelEventArgs {
    /// Creates pointer wheel event args.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
                Some(InputElement::pointer_wheel_changed_event()),
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

    /// The amount the wheel was scrolled by.
    #[inline]
    pub fn delta(&self) -> Vector {
        self.delta
    }
}
