//! Conversions between the types of the framework and those of the Wayland
//! protocol (the port of `WaylandConversionExtensions.cs`).

use crate::server::persistent::xdg_popup_positioner_params::XdgPopupPositionerParams;
use ferroui_controls::primitives::popup_positioning::{
    PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment, PopupPositionerParameters,
};
use wayland_protocols::xdg::shell::client::xdg_positioner::{Anchor, ConstraintAdjustment, Gravity};

/// Translates the [`PopupPositionerParameters`] of the framework (used
/// by `IPopupPositioner::update`) into the worker-side
/// [`XdgPopupPositionerParams`].
///
/// Coordinate convention: the anchor rectangle
/// arrives in the parent's client-area logical coordinates (consistent with
/// the managed positioner's usage of "parent client area"). On
/// Wayland CSD surfaces the entire buffer is the "client area" — including
/// the shadow margins — so the rect is passed through as buffer-relative
/// logical coords. The worker-side popup performs the
/// buffer→geometry origin shift and clamps the rect into the parent's
/// window-geometry rectangle (see [`XdgPopupPositionerParams`]).
///
/// # Panics
/// Panics for an anchor or a gravity that names opposite edges (the
/// `ArgumentOutOfRangeException` of the reference).
pub fn positioner_parameters_to_wayland(parameters: &PopupPositionerParameters) -> XdgPopupPositionerParams {
    XdgPopupPositionerParams {
        size: parameters.size,
        deflate: parameters.deflate,
        anchor_rect: parameters.anchor_rectangle,
        anchor: anchor_to_wayland(parameters.anchor()),
        gravity: gravity_to_wayland(parameters.gravity()),
        constraint_adjustment: constraint_adjustment_to_wayland(parameters.constraint_adjustment),
        offset: parameters.offset,
    }
}

/// [`PopupAnchor`] is a bitfield (Top=1, Bottom=2, Left=4, Right=8,
/// corners are bit combinations); the protocol's [`Anchor`]
/// is a dense enum (None=0..BottomRight=8). Translate explicitly per
/// combination rather than reinterpret-casting.
///
/// # Panics
/// Panics for a combination that is no edge and no corner.
pub fn anchor_to_wayland(a: PopupAnchor) -> Anchor {
    if a == PopupAnchor::NONE {
        Anchor::None
    } else if a == PopupAnchor::TOP {
        Anchor::Top
    } else if a == PopupAnchor::BOTTOM {
        Anchor::Bottom
    } else if a == PopupAnchor::LEFT {
        Anchor::Left
    } else if a == PopupAnchor::RIGHT {
        Anchor::Right
    } else if a == PopupAnchor::TOP_LEFT {
        Anchor::TopLeft
    } else if a == PopupAnchor::TOP_RIGHT {
        Anchor::TopRight
    } else if a == PopupAnchor::BOTTOM_LEFT {
        Anchor::BottomLeft
    } else if a == PopupAnchor::BOTTOM_RIGHT {
        Anchor::BottomRight
    } else {
        panic!("Unsupported PopupAnchor combination: {a:?} (Parameter 'a')")
    }
}

/// Same shape concern as [`anchor_to_wayland`]:
/// [`PopupGravity`] is a bitfield, [`Gravity`] is dense.
///
/// # Panics
/// Panics for a combination that is no edge and no corner.
pub fn gravity_to_wayland(g: PopupGravity) -> Gravity {
    if g == PopupGravity::NONE {
        Gravity::None
    } else if g == PopupGravity::TOP {
        Gravity::Top
    } else if g == PopupGravity::BOTTOM {
        Gravity::Bottom
    } else if g == PopupGravity::LEFT {
        Gravity::Left
    } else if g == PopupGravity::RIGHT {
        Gravity::Right
    } else if g == PopupGravity::TOP_LEFT {
        Gravity::TopLeft
    } else if g == PopupGravity::TOP_RIGHT {
        Gravity::TopRight
    } else if g == PopupGravity::BOTTOM_LEFT {
        Gravity::BottomLeft
    } else if g == PopupGravity::BOTTOM_RIGHT {
        Gravity::BottomRight
    } else {
        panic!("Unsupported PopupGravity combination: {g:?} (Parameter 'g')")
    }
}

/// [`PopupPositionerConstraintAdjustment`] currently has the
/// same numeric layout as [`ConstraintAdjustment`],
/// but that's not a contractual guarantee — translate flag-by-flag
/// rather than reinterpret-casting.
pub fn constraint_adjustment_to_wayland(a: PopupPositionerConstraintAdjustment) -> ConstraintAdjustment {
    let mut r = ConstraintAdjustment::None;
    if a.contains(PopupPositionerConstraintAdjustment::SLIDE_X) {
        r |= ConstraintAdjustment::SlideX;
    }
    if a.contains(PopupPositionerConstraintAdjustment::SLIDE_Y) {
        r |= ConstraintAdjustment::SlideY;
    }
    if a.contains(PopupPositionerConstraintAdjustment::FLIP_X) {
        r |= ConstraintAdjustment::FlipX;
    }
    if a.contains(PopupPositionerConstraintAdjustment::FLIP_Y) {
        r |= ConstraintAdjustment::FlipY;
    }
    if a.contains(PopupPositionerConstraintAdjustment::RESIZE_X) {
        r |= ConstraintAdjustment::ResizeX;
    }
    if a.contains(PopupPositionerConstraintAdjustment::RESIZE_Y) {
        r |= ConstraintAdjustment::ResizeY;
    }
    r
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use ferroui_base::{Point, Rect, Size, Thickness};

    #[test]
    fn edges_and_corners_map_to_the_dense_values_of_the_protocol() {
        let anchors = [
            (PopupAnchor::NONE, Anchor::None, 0),
            (PopupAnchor::TOP, Anchor::Top, 1),
            (PopupAnchor::BOTTOM, Anchor::Bottom, 2),
            (PopupAnchor::LEFT, Anchor::Left, 3),
            (PopupAnchor::RIGHT, Anchor::Right, 4),
            (PopupAnchor::TOP_LEFT, Anchor::TopLeft, 5),
            (PopupAnchor::BOTTOM_LEFT, Anchor::BottomLeft, 6),
            (PopupAnchor::TOP_RIGHT, Anchor::TopRight, 7),
            (PopupAnchor::BOTTOM_RIGHT, Anchor::BottomRight, 8),
        ];
        for (anchor, expected, wire) in anchors {
            assert_eq!(anchor_to_wayland(anchor), expected);
            assert_eq!(u32::from(expected), wire);
        }
        let gravities = [
            (PopupGravity::NONE, Gravity::None, 0),
            (PopupGravity::TOP, Gravity::Top, 1),
            (PopupGravity::BOTTOM, Gravity::Bottom, 2),
            (PopupGravity::LEFT, Gravity::Left, 3),
            (PopupGravity::RIGHT, Gravity::Right, 4),
            (PopupGravity::TOP_LEFT, Gravity::TopLeft, 5),
            (PopupGravity::BOTTOM_LEFT, Gravity::BottomLeft, 6),
            (PopupGravity::TOP_RIGHT, Gravity::TopRight, 7),
            (PopupGravity::BOTTOM_RIGHT, Gravity::BottomRight, 8),
        ];
        for (gravity, expected, wire) in gravities {
            assert_eq!(gravity_to_wayland(gravity), expected);
            assert_eq!(u32::from(expected), wire);
        }
    }

    #[test]
    #[should_panic(expected = "Unsupported PopupAnchor combination")]
    fn an_anchor_of_opposite_edges_is_refused() {
        anchor_to_wayland(PopupAnchor::VERTICAL_MASK);
    }

    #[test]
    #[should_panic(expected = "Unsupported PopupGravity combination")]
    fn a_gravity_of_opposite_edges_is_refused() {
        gravity_to_wayland(PopupGravity::LEFT | PopupGravity::RIGHT);
    }

    #[test]
    fn constraint_adjustments_are_translated_flag_by_flag() {
        assert_eq!(constraint_adjustment_to_wayland(PopupPositionerConstraintAdjustment::NONE), ConstraintAdjustment::None);
        assert_eq!(
            constraint_adjustment_to_wayland(PopupPositionerConstraintAdjustment::SLIDE_X | PopupPositionerConstraintAdjustment::FLIP_Y),
            ConstraintAdjustment::SlideX | ConstraintAdjustment::FlipY
        );
        assert_eq!(
            constraint_adjustment_to_wayland(PopupPositionerConstraintAdjustment::ALL),
            ConstraintAdjustment::SlideX
                | ConstraintAdjustment::SlideY
                | ConstraintAdjustment::FlipX
                | ConstraintAdjustment::FlipY
                | ConstraintAdjustment::ResizeX
                | ConstraintAdjustment::ResizeY
        );
        assert_eq!(constraint_adjustment_to_wayland(PopupPositionerConstraintAdjustment::ALL).bits(), 63);
    }

    #[test]
    fn the_parameters_go_through_with_their_enumerations_translated() {
        let mut parameters = PopupPositionerParameters::default();
        parameters.size = Size::new(200.0, 100.0);
        parameters.deflate = Thickness::new(4.0, 4.0, 4.0, 4.0);
        parameters.anchor_rectangle = Rect::new(10.0, 20.0, 30.0, 40.0);
        parameters.set_anchor(PopupAnchor::BOTTOM_LEFT);
        parameters.set_gravity(PopupGravity::BOTTOM_RIGHT);
        parameters.constraint_adjustment = PopupPositionerConstraintAdjustment::FLIP_Y;
        parameters.offset = Point::new(1.0, 2.0);
        let translated = positioner_parameters_to_wayland(&parameters);
        assert_eq!(translated.size, Size::new(200.0, 100.0));
        assert_eq!(translated.deflate, Thickness::new(4.0, 4.0, 4.0, 4.0));
        assert_eq!(translated.anchor_rect, Rect::new(10.0, 20.0, 30.0, 40.0));
        assert_eq!(translated.anchor, Anchor::BottomLeft);
        assert_eq!(translated.gravity, Gravity::BottomRight);
        assert_eq!(translated.constraint_adjustment, ConstraintAdjustment::FlipY);
        assert_eq!(translated.offset, Point::new(1.0, 2.0));
    }
}
