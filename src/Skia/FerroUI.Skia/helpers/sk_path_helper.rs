use crate::helpers::drawing_context_helper::try_create_dash_effect;
use crate::sk_paint_cache::SkPaintCache;
use crate::skia_sharp_extensions::{to_sk_stroke_cap, to_sk_stroke_join};
use ferroui_base::media::IPen;
use ferroui_base::utilities::MathUtilities;
use skia_safe::path::Verb;
use skia_safe::{path_utils, Path, PathBuilder};

/// Creates a new path that is a closed version of the source path: every
/// contour that returns to its starting point is closed explicitly.
pub fn create_closed_path(path: &Path) -> Path {
    let iter = skia_safe::path::Iter::new(path, true);
    let mut rv = PathBuilder::new();
    let mut weights = path.conic_weights().iter();

    for (verb, points) in iter {
        match verb {
            Verb::Move => {
                rv.move_to(points[0]);
            }
            Verb::Line => {
                rv.line_to(points[1]);
            }
            Verb::Close => {
                rv.close();
            }
            Verb::Quad => {
                rv.quad_to(points[1], points[2]);
            }
            Verb::Cubic => {
                rv.cubic_to(points[1], points[2], points[3]);
            }
            Verb::Conic => {
                // Conic weights are stored in verb order.
                rv.conic_to(points[1], points[2], weights.next().copied().unwrap_or(1.0));
            }
            _ => {}
        }
    }

    rv.detach()
}

/// Creates a new path from the stroke of `path` with the given pen: the
/// outline that filling would paint. Returns `None` for a zero thickness pen.
pub fn create_stroked_path(path: &Path, pen: &dyn IPen) -> Option<Path> {
    if MathUtilities::is_zero(pen.thickness()) {
        return None;
    }

    let mut paint = SkPaintCache::get();
    paint.set_stroke(true);
    paint.set_stroke_width(pen.thickness() as f32);
    paint.set_stroke_cap(to_sk_stroke_cap(pen.line_cap()));
    paint.set_stroke_join(to_sk_stroke_join(pen.line_join()));
    paint.set_stroke_miter(pen.miter_limit() as f32);

    if let Some(dash_effect) = try_create_dash_effect(Some(pen)) {
        paint.set_path_effect(dash_effect);
    }

    let mut result = PathBuilder::new();
    path_utils::fill_path_with_paint(path, &paint, &mut result, None, None);

    SkPaintCache::return_reset(paint);

    Some(result.detach())
}
