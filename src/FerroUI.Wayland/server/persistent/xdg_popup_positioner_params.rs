//! The parameters of the positioner of a popup as the worker takes them (the
//! port of `XdgPopupPositionerParams.cs`).

use ferroui_base::{Point, Rect, Size, Thickness};
use wayland_protocols::xdg::shell::client::xdg_positioner::{Anchor, ConstraintAdjustment, Gravity};

/// Flat, immutable description of an xdg_positioner request. Built UI-side
/// in the parent's **buffer-relative** logical coordinate system —
/// i.e. relative to the true top-left of the parent surface, including any
/// CSD shadow margins. The worker is responsible for translating into the
/// parent's xdg_surface window-geometry coordinate system (subtracting the
/// parent's shadow extents) and clamping the anchor rectangle to that
/// geometry, since the protocol requires that "the anchor rectangle may
/// not extend outside the window geometry of the positioned child's parent
/// surface". Doing the translation worker-side keeps the UI oblivious
/// to the wayland buffer-vs-geometry distinction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XdgPopupPositionerParams {
    /// The size of the popup, in logical surface-local coordinates. Both width and height must
    /// be positive.
    pub size: Size,
    /// The popup child's margin, in logical pixels, excluded from the surface's window geometry.
    pub deflate: Thickness,
    /// The anchor rectangle in the parent's **buffer-relative** logical coordinates (true
    /// top-left, includes shadow).
    pub anchor_rect: Rect,
    /// The edge/corner of the anchor rect that the popup is anchored to. The wire enum of the
    /// bindings: the bitfield of the framework is translated explicitly UI-side.
    pub anchor: Anchor,
    /// The direction the popup expands from the anchor point. Same translation note as
    /// `anchor`.
    pub gravity: Gravity,
    /// How the compositor may adjust the popup if it doesn't fit on screen. The flags of the
    /// wire; those of the framework are translated flag-by-flag UI-side, never
    /// reinterpret-cast.
    pub constraint_adjustment: ConstraintAdjustment,
    /// Additional offset applied to the resolved popup position (after anchor + gravity have
    /// been resolved). Already relative to the popup itself, not the parent — no shadow
    /// translation applies.
    pub offset: Point,
}
