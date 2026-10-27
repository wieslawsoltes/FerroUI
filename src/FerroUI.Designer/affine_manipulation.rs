//! Affine geometry shared by the pointer adorner and source planners.
use ferroui_base::{Matrix, Point, Rect, RelativePoint, Size, Visual};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Geometry {
    pub size: Size,
    pub to_surface: Matrix,
    pub from_surface: Matrix,
    pub pivot: Point,
    minimum: Size,
    maximum: Size,
}
impl Geometry {
    pub fn capture(visual: &Visual, surface: &Visual) -> Option<Self> {
        let to_surface = affine(visual.transform_to_visual(surface)?)?;
        let from_surface = inverse(to_surface)?;
        let size = visual.bounds().size();
        let mut origin = visual.render_transform_origin().to_pixels(size);
        if visual.has_mirror_transform() {
            origin.x = size.width - origin.x;
        }
        Some(Self {
            size,
            to_surface,
            from_surface,
            pivot: origin.transform(to_surface),
            minimum: visual
                .to_ref()
                .cast::<ferroui_base::layout::Layoutable>()
                .map_or(Size::new(1.0, 1.0), |layout| {
                    Size::new(layout.min_width().max(1.0), layout.min_height().max(1.0))
                }),
            maximum: visual
                .to_ref()
                .cast::<ferroui_base::layout::Layoutable>()
                .map_or(Size::new(f64::INFINITY, f64::INFINITY), |layout| {
                    Size::new(layout.max_width(), layout.max_height())
                }),
        })
    }
    pub fn handles(&self) -> [Point; 8] {
        local_handles(Rect::from_size(self.size)).map(|point| point.transform(self.to_surface))
    }
    pub fn rotation_handle(&self, zoom: f64) -> Point {
        let top = self.handles()[1];
        let center =
            Point::new(self.size.width * 0.5, self.size.height * 0.5).transform(self.to_surface);
        let delta = top - center;
        let length = delta.x.hypot(delta.y).max(1e-9);
        Point::new(
            top.x + delta.x / length * 24.0 / zoom,
            top.y + delta.y / length * 24.0 / zoom,
        )
    }
    pub fn resize(&self, start: Point, end: Point, handle: u8) -> Rect {
        let delta = end.transform(self.from_surface) - start.transform(self.from_surface);
        let mut result = resize(Rect::from_size(self.size), delta.x, delta.y, handle);
        let width = result.width.clamp(
            self.minimum.width,
            self.maximum.width.max(self.minimum.width),
        );
        let height = result.height.clamp(
            self.minimum.height,
            self.maximum.height.max(self.minimum.height),
        );
        if matches!(handle, 0 | 6 | 7) {
            result.x += result.width - width;
        }
        if matches!(handle, 0..=2) {
            result.y += result.height - height;
        }
        result.width = width;
        result.height = height;
        result
    }
    pub fn rotation(&self, start: Point, end: Point) -> f64 {
        let from = (start.y - self.pivot.y).atan2(start.x - self.pivot.x);
        let to = (end.y - self.pivot.y).atan2(end.x - self.pivot.x);
        (to - from).to_degrees()
    }
}

/// Canonical affine matrices keep the homogeneous row exact. A general 3x3 inverse can
/// round m33 just below one, which otherwise sends native point transforms through their
/// single-precision perspective path even though this geometry is entirely affine.
pub(crate) fn affine(matrix: Matrix) -> Option<Matrix> {
    if matrix.m13 != 0.0 || matrix.m23 != 0.0 || !matrix.m33.is_finite() || matrix.m33 == 0.0 {
        return None;
    }
    Some(Matrix::new(
        matrix.m11 / matrix.m33,
        matrix.m12 / matrix.m33,
        matrix.m21 / matrix.m33,
        matrix.m22 / matrix.m33,
        matrix.m31 / matrix.m33,
        matrix.m32 / matrix.m33,
    ))
}

pub(crate) fn inverse(matrix: Matrix) -> Option<Matrix> {
    affine(affine(matrix)?.try_invert()?)
}

pub(crate) fn local_handles(r: Rect) -> [Point; 8] {
    [
        Point::new(r.x, r.y),
        Point::new(r.x + r.width / 2.0, r.y),
        Point::new(r.right(), r.y),
        Point::new(r.right(), r.y + r.height / 2.0),
        Point::new(r.right(), r.bottom()),
        Point::new(r.x + r.width / 2.0, r.bottom()),
        Point::new(r.x, r.bottom()),
        Point::new(r.x, r.y + r.height / 2.0),
    ]
}
pub(crate) fn resize(before: Rect, dx: f64, dy: f64, handle: u8) -> Rect {
    let (mut left, mut top, mut right, mut bottom) =
        (before.x, before.y, before.right(), before.bottom());
    if matches!(handle, 0 | 6 | 7) {
        left = (left + dx).min(right - 1.0);
    }
    if matches!(handle, 2..=4) {
        right = (right + dx).max(left + 1.0);
    }
    if matches!(handle, 0..=2) {
        top = (top + dy).min(bottom - 1.0);
    }
    if matches!(handle, 4..=6) {
        bottom = (bottom + dy).max(top + 1.0);
    }
    Rect::new(left, top, right - left, bottom - top)
}

/// Native local-render matrix, including RTL mirroring and relative/absolute origins.
pub(crate) fn local_render(
    size: Size,
    origin: RelativePoint,
    mirror: bool,
    render: Matrix,
) -> Matrix {
    let point = origin.to_pixels(size);
    let offset = Matrix::create_translation(point.x, point.y);
    let mirror = if mirror {
        Matrix::new(-1.0, 0.0, 0.0, 1.0, size.width, 0.0)
    } else {
        Matrix::IDENTITY
    };
    mirror * -offset * render * offset
}

/// Resize the layout rectangle while keeping the visual location of the opposite edge fixed.
pub(crate) fn resized_layout(
    before: Rect,
    after_local: Rect,
    origin: RelativePoint,
    mirror: bool,
    render: Matrix,
) -> Rect {
    let old = local_render(before.size(), origin, mirror, render);
    let new = local_render(after_local.size(), origin, mirror, render);
    let wanted = Point::new(after_local.x, after_local.y).transform(old);
    let actual = Point::default().transform(new);
    Rect::new(
        before.x + wanted.x - actual.x,
        before.y + wanted.y - actual.y,
        after_local.width,
        after_local.height,
    )
}

/// Solve the render matrix for a desired local-to-parent matrix without assuming a centered
/// origin, orthogonal ancestor basis, positive determinant, or uniform scale.
pub(crate) fn render_for_local(
    local: Matrix,
    bounds: Rect,
    origin: RelativePoint,
    mirror: bool,
) -> Option<Matrix> {
    let point = origin.to_pixels(bounds.size());
    let offset = Matrix::create_translation(point.x, point.y);
    let mirror = if mirror {
        Matrix::new(-1.0, 0.0, 0.0, 1.0, bounds.width, 0.0)
    } else {
        Matrix::IDENTITY
    };
    Some(
        offset
            * inverse(mirror)?
            * local
            * Matrix::create_translation(-bounds.x, -bounds.y)
            * -offset,
    )
}

pub(crate) fn matrix_text(matrix: Matrix) -> String {
    format!(
        "{},{},{},{},{},{}",
        matrix.m11, matrix.m12, matrix.m21, matrix.m22, matrix.m31, matrix.m32
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::RelativeUnit;
    fn close(a: Point, b: Point) {
        assert!(
            (a.x - b.x).abs() < 1e-7 && (a.y - b.y).abs() < 1e-7,
            "{a:?} != {b:?}"
        );
    }
    #[test]
    fn resize_preserves_affine_quad_with_custom_origins_and_mirrors() {
        let before = Rect::new(40.0, 25.0, 80.0, 45.0);
        let after = Rect::new(-12.0, 8.0, 111.0, 33.0);
        for origin in [
            RelativePoint::new(0.2, 0.8, RelativeUnit::Relative),
            RelativePoint::new(7.0, -9.0, RelativeUnit::Absolute),
        ] {
            for mirror in [false, true] {
                let render = Matrix::new(-1.7, 0.4, 0.7, 0.9, 5.0, -3.0);
                let parent = Matrix::new(0.3, 1.4, -1.2, 0.6, 75.0, -30.0);
                let layout = resized_layout(before, after, origin, mirror, render);
                let old = local_render(before.size(), origin, mirror, render)
                    * Matrix::create_translation(before.x, before.y)
                    * parent;
                let new = local_render(layout.size(), origin, mirror, render)
                    * Matrix::create_translation(layout.x, layout.y)
                    * parent;
                for p in [Point::new(0.0, 0.0), Point::new(after.width, after.height)] {
                    close(
                        Point::new(p.x + after.x, p.y + after.y).transform(old),
                        p.transform(new),
                    );
                }
            }
        }
    }
    #[test]
    fn rotation_solves_nonuniform_skewed_and_mirrored_parent() {
        let bounds = Rect::new(30.0, 15.0, 90.0, 50.0);
        let origin = RelativePoint::new(0.1, 0.9, RelativeUnit::Relative);
        let render = Matrix::new(1.1, 0.2, -0.3, 0.6, 3.0, 4.0);
        let parent = Matrix::new(-2.0, 0.6, 0.8, 0.7, 150.0, 80.0);
        let old = local_render(bounds.size(), origin, true, render)
            * Matrix::create_translation(bounds.x, bounds.y);
        let rotation = Matrix::create_rotation_at(0.63, Point::new(180.0, 55.0));
        let wanted = old * parent * rotation * inverse(parent).unwrap();
        let result = render_for_local(wanted, bounds, origin, true).unwrap();
        let actual = local_render(bounds.size(), origin, true, result)
            * Matrix::create_translation(bounds.x, bounds.y)
            * parent;
        for p in [
            Point::default(),
            Point::new(90.0, 50.0),
            Point::new(17.0, 32.0),
        ] {
            close(p.transform(actual), p.transform(old * parent * rotation));
        }
    }
}
