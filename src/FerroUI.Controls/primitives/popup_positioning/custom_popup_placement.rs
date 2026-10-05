use super::{PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment};
use ferroui_base::{Point, Rect, Ref, Size, Thickness, Visual};

/// Defines custom placement parameters for a
/// [`CustomPopupPlacementCallback`](super::CustomPopupPlacementCallback)
/// callback.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomPopupPlacement {
    gravity: PopupGravity,
    anchor: PopupAnchor,
    popup_size: Size,
    deflate: Thickness,
    target: Ref<Visual>,

    /// See [`PopupPositionerParameters::anchor_rectangle`](super::PopupPositionerParameters::anchor_rectangle).
    pub anchor_rectangle: Rect,

    /// See [`PopupPositionerParameters::constraint_adjustment`](super::PopupPositionerParameters::constraint_adjustment).
    pub constraint_adjustment: PopupPositionerConstraintAdjustment,

    /// See [`PopupPositionerParameters::offset`](super::PopupPositionerParameters::offset).
    pub offset: Point,
}

impl CustomPopupPlacement {
    // Used by the popup position request consumer.
    pub(crate) fn new(popup_size: Size, deflate: Thickness, target: Ref<Visual>) -> Self {
        Self {
            gravity: PopupGravity::NONE,
            anchor: PopupAnchor::NONE,
            popup_size,
            deflate,
            target,
            anchor_rectangle: Rect::default(),
            constraint_adjustment: PopupPositionerConstraintAdjustment::NONE,
            offset: Point::default(),
        }
    }

    /// The [`Size`] of the popup control.
    pub fn popup_size(&self) -> Size {
        self.popup_size
    }

    /// Gets how far to deflate [`popup_size`](Self::popup_size) when
    /// positioning the popup.
    ///
    /// This is normally set to the margin of the popup's child. This allows
    /// out-of-bounds effects like drop shadows to be drawn without affecting
    /// the final position of the child.
    pub fn deflate(&self) -> Thickness {
        self.deflate
    }

    /// Placement target of the popup.
    pub fn target(&self) -> Ref<Visual> {
        self.target.clone()
    }

    /// See [`PopupPositionerParameters::anchor`](super::PopupPositionerParameters::anchor).
    pub fn anchor(&self) -> PopupAnchor {
        self.anchor
    }

    /// See [`PopupPositionerParameters::set_anchor`](super::PopupPositionerParameters::set_anchor).
    ///
    /// # Panics
    ///
    /// Panics when opposite edges are specified.
    pub fn set_anchor(&mut self, value: PopupAnchor) {
        value.validate_edge();
        self.anchor = value;
    }

    /// See [`PopupPositionerParameters::gravity`](super::PopupPositionerParameters::gravity).
    pub fn gravity(&self) -> PopupGravity {
        self.gravity
    }

    /// See [`PopupPositionerParameters::set_gravity`](super::PopupPositionerParameters::set_gravity).
    ///
    /// # Panics
    ///
    /// Panics when opposite edges are specified.
    pub fn set_gravity(&mut self, value: PopupGravity) {
        value.validate_gravity();
        self.gravity = value;
    }
}
