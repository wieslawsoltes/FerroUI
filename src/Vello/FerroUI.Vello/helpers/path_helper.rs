use crate::helpers::drawing_context_helper::try_create_dashes;
use crate::vello_extensions::{to_cap, to_join, PATH_TOLERANCE};
use ferroui_base::media::IPen;
use ferroui_base::utilities::MathUtilities;
use kurbo::{BezPath, Stroke, StrokeOpts};

/// The stroke style of a pen: its thickness, caps, join, miter limit and
/// dashes.
///
/// The dash lengths and the dash offset of a pen are relative to its
/// thickness.
pub fn create_stroke(pen: &dyn IPen) -> Stroke {
    let mut stroke = Stroke::new(pen.thickness())
        .with_caps(to_cap(pen.line_cap()))
        .with_join(to_join(pen.line_join()))
        .with_miter_limit(pen.miter_limit());

    if let Some((dashes, offset)) = try_create_dashes(Some(pen)) {
        stroke = stroke.with_dashes(offset, dashes);
    }

    stroke
}

/// Creates a new path from the stroke of `path` with the given pen: the
/// outline that filling (non-zero) would paint. Returns `None` for a zero
/// thickness pen.
///
/// The figures of the outline are closed.
pub fn create_stroked_path(path: &BezPath, pen: &dyn IPen) -> Option<BezPath> {
    if MathUtilities::is_zero(pen.thickness()) {
        return None;
    }

    Some(kurbo::stroke(path.iter(), &create_stroke(pen), &StrokeOpts::default(), PATH_TOLERANCE))
}
