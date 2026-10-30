//! Designer chrome uses native vector icons, never a platform symbol-font dependency.
//!
//! Every icon opts into a scoped template which fits the complete geometry into its
//! requested box, preserving aspect ratio and one logical pixel of optical padding.
//! Application PathIcons keep their application's own theme/template semantics.
use ferroui_base::Ref;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::Geometry;
use ferroui_controls::PathIcon;

pub(crate) const MORE: &str = "F1 M2 7A1 1 0 1 1 2 9A1 1 0 1 1 2 7Z M8 7A1 1 0 1 1 8 9A1 1 0 1 1 8 7Z M14 7A1 1 0 1 1 14 9A1 1 0 1 1 14 7Z";
pub(crate) const ADD: &str = "F1 M7 1H9V7H15V9H9V15H7V9H1V7H7Z";
pub(crate) const REMOVE: &str = "F1 M1 7H15V9H1Z";
pub(crate) const CLOSE: &str =
    "F1 M3 1.6L8 6.6L13 1.6L14.4 3L9.4 8L14.4 13L13 14.4L8 9.4L3 14.4L1.6 13L6.6 8L1.6 3Z";
pub(crate) const MOVE_UP: &str = "F1 M8 1L15 8L13.6 9.4L9 4.8V15H7V4.8L2.4 9.4L1 8Z";
pub(crate) const MOVE_DOWN: &str = "F1 M7 1H9V11.2L13.6 6.6L15 8L8 15L1 8L2.4 6.6L7 11.2Z";
pub(crate) const EXPRESSION: &str = "F1 M8 1L15 8L8 15L1 8Z";
pub(crate) const LOCAL_VALUE: &str = "F1 M8 1A7 7 0 1 1 8 15A7 7 0 1 1 8 1Z";

/// Create an icon with an explicit drawing box. The geometry's declared fill rule is
/// retained, including project-owned asset paths; built-in compound paths use nonzero
/// fill so overlapping contours join instead of punching accidental transparent seams.
pub(crate) fn vector_icon(data: &str, size: f64) -> Ref<PathIcon> {
    geometry_icon(
        Geometry::parse(data).expect("valid designer vector icon"),
        size,
    )
}

pub(crate) fn geometry_icon(data: Ref<Geometry>, size: f64) -> Ref<PathIcon> {
    let icon = PathIcon::new();
    icon.classes().add("designer-icon");
    icon.set_data(data);
    icon.set_width(size);
    icon.set_height(size);
    icon.set_horizontal_alignment(HorizontalAlignment::Center);
    icon.set_vertical_alignment(VerticalAlignment::Center);
    icon.set_is_hit_test_visible(false);
    icon
}
