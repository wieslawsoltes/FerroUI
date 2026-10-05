use crate::{Size, Vector};

/// Interface implemented by scrollable controls.
///
/// Upstream this contract lives with the control primitives; it is defined
/// here because directional navigation is its only consumer in the base
/// library so far. An input element exposes it through the
/// `as_scrollable` virtual member of `InputElement`.
pub trait IScrollable {
    /// The extent of the scrollable content, in logical units.
    fn extent(&self) -> Size;

    /// The current scroll offset, in logical units.
    fn offset(&self) -> Vector;

    /// Sets the current scroll offset, in logical units.
    fn set_offset(&self, value: Vector);

    /// The size of the viewport, in logical units.
    fn viewport(&self) -> Size;

    /// Whether the content can be scrolled horizontally.
    fn can_horizontally_scroll(&self) -> bool;

    /// Whether the content can be scrolled vertically.
    fn can_vertically_scroll(&self) -> bool;
}
