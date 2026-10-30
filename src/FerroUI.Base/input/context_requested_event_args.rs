use super::{HoldingRoutedEventArgs, InputElement, PointerEventArgs, PointerType};
use crate::interactivity::RoutedEventArgs;
use crate::{ferro_routed_event_args, Point, Rect, Visual, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;

/// Provides event data for the context requested event.
#[derive(Clone)]
pub struct ContextRequestedEventArgs {
    base: RoutedEventArgs,
    pointer_event_args: Option<PointerEventArgs>,
    is_holding: bool,
    placement_rect: Rc<RefCell<Option<(WeakRef<Visual>, Rect)>>>,
}

ferro_routed_event_args!(ContextRequestedEventArgs: RoutedEventArgs);

impl Default for ContextRequestedEventArgs {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextRequestedEventArgs {
    /// Creates args for a context request that has no position, e.g. one
    /// made with the keyboard.
    pub fn new() -> Self {
        Self {
            base: RoutedEventArgs::with_event(InputElement::context_requested_event()),
            pointer_event_args: None,
            is_holding: false,
            placement_rect: Rc::new(RefCell::new(None)),
        }
    }

    /// Creates args for a context request made with a pointer.
    pub fn from_pointer_event_args(pointer_event_args: &PointerEventArgs) -> Self {
        Self { pointer_event_args: Some(pointer_event_args.clone()), ..Self::new() }
    }

    /// Creates args from other context requested args, keeping the position.
    pub fn from_context_requested_event_args(context_requested_event_args: &ContextRequestedEventArgs) -> Self {
        Self {
            pointer_event_args: context_requested_event_args.pointer_event_args.clone(),
            placement_rect: context_requested_event_args.placement_rect.clone(),
            ..Self::new()
        }
    }

    pub(crate) fn from_holding(hold_routed_event_args: &HoldingRoutedEventArgs) -> Self {
        Self {
            pointer_event_args: Some(hold_routed_event_args.pointer_event_args().clone()),
            is_holding: true,
            ..Self::new()
        }
    }

    /// Gets the point on which the context was requested, relative to the
    /// specified element (or to the root when `None`).
    ///
    /// Returns `None` if the context was not requested with a pointer, e.g.
    /// when it was requested with the keyboard.
    pub fn try_get_position(&self, relative_to: Option<&InputElement>) -> Option<Point> {
        self.pointer_event_args.as_ref().map(|e| e.get_position(relative_to.map(|element| &****element)))
    }

    /// Supplies an explicit anchor for this menu request in `relative_to`'s
    /// device-independent coordinates. A zero-size rectangle anchors at a point.
    ///
    /// This does not change whether the request originated from a pointer or
    /// keyboard. The owner is weak, and shared event handles see the same anchor.
    pub fn set_placement_rect(&self, relative_to: &Visual, rect: Rect) {
        *self.placement_rect.borrow_mut() = Some((relative_to.to_ref().downgrade(), rect));
    }

    /// Gets this request's explicit anchor in the recipient's coordinates.
    /// Returns `None` when no anchor was supplied, its owner expired, or the
    /// recipient and owner do not share a visual coordinate system.
    pub fn try_get_placement_rect(&self, relative_to: &Visual) -> Option<Rect> {
        let (owner, rect) = self.placement_rect.borrow().clone()?;
        let owner = owner.upgrade()?;
        owner.transform_to_visual(relative_to).map(|transform| rect.transform_to_aabb(transform))
    }

    /// Restores the ordinary pointer or keyboard placement for this request.
    pub fn clear_placement_rect(&self) {
        self.placement_rect.borrow_mut().take();
    }

    /// Whether the context was requested by holding a pointer down.
    ///
    /// Internal in the reference, where the controls assembly sees it.
    pub fn is_holding(&self) -> bool {
        self.is_holding
    }

    #[allow(dead_code)]
    pub fn pointer_type(&self) -> Option<PointerType> {
        self.pointer_event_args.as_ref().map(|e| e.pointer().type_())
    }
}
