use super::{CustomPopupPlacementCallback, PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment};
use crate::PlacementMode;
use ferroui_base::{Point, Rect, Ref, Visual};

/// A request to position a popup relative to a target.
#[derive(Clone)]
pub struct PopupPositionRequest {
    target: Ref<Visual>,
    placement: PlacementMode,
    offset: Point,
    anchor: PopupAnchor,
    gravity: PopupGravity,
    constraint_adjustment: PopupPositionerConstraintAdjustment,
    anchor_rect: Option<Rect>,
    placement_callback: Option<CustomPopupPlacementCallback>,
}

// The constructors are used by the popup, as in the reference implementation.
impl PopupPositionRequest {
    pub(crate) fn new(target: Ref<Visual>, placement: PlacementMode) -> Self {
        Self {
            target,
            placement,
            offset: Point::default(),
            anchor: PopupAnchor::NONE,
            gravity: PopupGravity::NONE,
            constraint_adjustment: PopupPositionerConstraintAdjustment::NONE,
            anchor_rect: None,
            placement_callback: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with(
        target: Ref<Visual>,
        placement: PlacementMode,
        offset: Point,
        anchor: PopupAnchor,
        gravity: PopupGravity,
        constraint_adjustment: PopupPositionerConstraintAdjustment,
        anchor_rect: Option<Rect>,
        placement_callback: Option<CustomPopupPlacementCallback>,
    ) -> Self {
        Self {
            offset,
            anchor,
            gravity,
            constraint_adjustment,
            anchor_rect,
            placement_callback,
            ..Self::new(target, placement)
        }
    }
}

impl PopupPositionRequest {
    /// The placement target.
    pub fn target(&self) -> Ref<Visual> {
        self.target.clone()
    }

    /// The placement mode.
    pub fn placement(&self) -> PlacementMode {
        self.placement
    }

    /// The offset of the popup from the computed position.
    pub fn offset(&self) -> Point {
        self.offset
    }

    /// The anchor point on the anchor rectangle.
    pub fn anchor(&self) -> PopupAnchor {
        self.anchor
    }

    /// The direction in which the popup opens.
    pub fn gravity(&self) -> PopupGravity {
        self.gravity
    }

    /// How the position is adjusted when the popup is constrained.
    pub fn constraint_adjustment(&self) -> PopupPositionerConstraintAdjustment {
        self.constraint_adjustment
    }

    /// The anchor rectangle within the target, if one is set.
    pub fn anchor_rect(&self) -> Option<Rect> {
        self.anchor_rect
    }

    /// The custom placement callback, if one is set.
    pub fn placement_callback(&self) -> Option<CustomPopupPlacementCallback> {
        self.placement_callback.clone()
    }
}
