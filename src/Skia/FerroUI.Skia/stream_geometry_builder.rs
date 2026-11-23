use crate::geometry_impl::{try_get_geometry_impl, FillPath};
use crate::simple_geometry_impl::SimpleGeometryImpl;
use crate::skia_sharp_extensions::to_sk_point;
use ferroui_base::media::{FillRule, SweepDirection};
use ferroui_base::platform::{IGeometryContext, IGeometryImpl, IStreamGeometryBuilder};
use ferroui_base::{Point, Size};
use skia_safe::path_builder::ArcSize;
use skia_safe::{Path, PathBuilder, PathDirection, PathFillType};
use std::sync::Arc;

/// The fill being built by a [`StreamGeometryBuilder`].
enum FillBuilder {
    /// No fill path exists yet.
    None,
    /// The fill is the stroke.
    SameAsStroke,
    /// The fill is built separately from the stroke.
    Separate(PathBuilder),
}

/// Builds a Skia geometry from drawing commands.
///
/// Skia paths are immutable, so the builder works on path builders seeded
/// from the paths of its source geometry, and creates the geometry from
/// their snapshots.
pub struct StreamGeometryBuilder {
    stroke: PathBuilder,
    fill: FillBuilder,
    is_filled: bool,
    start_point: Point,
    is_figure_broken: bool,
    built: bool,
}

impl StreamGeometryBuilder {
    /// Creates a builder that starts with the figures of `source`, a geometry
    /// of this backend, or with an empty geometry when `source` is `None`.
    ///
    /// Panics if `source` was not created by the Skia render interface.
    pub fn new(source: Option<&dyn IGeometryImpl>) -> Self {
        let (stroke, fill) = match source {
            None => (PathBuilder::new_path(&Self::create_empty_path()), FillBuilder::SameAsStroke),
            Some(source) => {
                let geometry = try_get_geometry_impl(source)
                    .unwrap_or_else(|| panic!("The geometry wasn't created by the Skia render interface."));
                let stroke_path = geometry.stroke_path();
                let fill = match geometry.fill() {
                    FillPath::SameAsStroke => FillBuilder::SameAsStroke,
                    FillPath::Separate(fill_path) => FillBuilder::Separate(PathBuilder::new_path(&fill_path)),
                    // Without a stroke path either, the fill is the (empty) stroke.
                    FillPath::None if stroke_path.is_none() => FillBuilder::SameAsStroke,
                    FillPath::None => FillBuilder::None,
                };
                let stroke = PathBuilder::new_path(&stroke_path.unwrap_or_else(Self::create_empty_path));
                (stroke, fill)
            }
        };

        Self { stroke, fill, is_filled: false, start_point: Point::default(), is_figure_broken: false, built: false }
    }

    /// Creates a new empty path.
    fn create_empty_path() -> Path {
        Path::new_with_fill_type(PathFillType::EvenOdd)
    }

    /// The fill builder, created on first use.
    fn fill(&mut self) -> &mut PathBuilder {
        if matches!(self.fill, FillBuilder::None) {
            self.fill = FillBuilder::Separate(PathBuilder::new());
        }

        match &mut self.fill {
            FillBuilder::SameAsStroke => &mut self.stroke,
            FillBuilder::Separate(fill) => fill,
            FillBuilder::None => unreachable!("the fill builder was just created"),
        }
    }

    /// Whether commands have to be repeated on a separate fill path.
    fn duplicate(&self) -> bool {
        self.is_filled && !matches!(self.fill, FillBuilder::SameAsStroke)
    }

    fn ensure_separate_fill_path(&mut self) {
        if matches!(self.fill, FillBuilder::SameAsStroke) {
            self.fill = FillBuilder::Separate(PathBuilder::new_path(&self.stroke.snapshot()));
        }
    }

    fn break_figure(&mut self) {
        if !self.is_figure_broken {
            self.is_figure_broken = true;
            self.ensure_separate_fill_path();
        }
    }
}

impl IStreamGeometryBuilder for StreamGeometryBuilder {
    fn to_geometry(&mut self) -> Arc<dyn IGeometryImpl> {
        if self.built {
            panic!("The geometry has already been built.");
        }

        self.built = true;

        let stroke = self.stroke.snapshot();
        let fill = match &self.fill {
            FillBuilder::None => FillPath::None,
            FillBuilder::SameAsStroke => FillPath::SameAsStroke,
            FillBuilder::Separate(fill) => FillPath::Separate(fill.snapshot()),
        };

        SimpleGeometryImpl::new(stroke, fill, None)
    }
}

impl IGeometryContext for StreamGeometryBuilder {
    fn dispose(&mut self) {}

    fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
        if !is_filled {
            self.ensure_separate_fill_path();
        }

        self.is_filled = is_filled;
        self.start_point = start_point;
        self.is_figure_broken = false;
        self.stroke.move_to(to_sk_point(start_point));

        if self.duplicate() {
            self.fill().move_to(to_sk_point(start_point));
        }
    }

    fn end_figure(&mut self, is_closed: bool) {
        if is_closed {
            if self.is_figure_broken {
                self.stroke.line_to(to_sk_point(self.start_point));
                self.is_figure_broken = false;
            } else {
                self.stroke.close();
            }

            if self.duplicate() {
                self.fill().close();
            }
        }
    }

    fn set_fill_rule(&mut self, fill_rule: FillRule) {
        let fill_type = if fill_rule == FillRule::EvenOdd { PathFillType::EvenOdd } else { PathFillType::Winding };
        self.fill().set_fill_type(fill_type);
    }

    fn line_to(&mut self, point: Point, is_stroked: bool) {
        let point = to_sk_point(point);

        if is_stroked {
            self.stroke.line_to(point);
        } else {
            self.break_figure();
            self.stroke.move_to(point);
        }

        if self.duplicate() {
            self.fill().line_to(point);
        }
    }

    // A run of stroked lines is one call into Skia for the path of the stroke, and one for the
    // path of the fill where the two are apart, where every point was a call for each.
    fn lines_to(&mut self, points: &[Point], is_stroked: bool) {
        if !is_stroked || points.len() < 2 {
            for point in points {
                self.line_to(*point, is_stroked);
            }
            return;
        }
        let run: Vec<skia_safe::Point> = points.iter().map(|point| to_sk_point(*point)).collect();
        self.stroke.polyline_to(&run);
        if self.duplicate() {
            self.fill().polyline_to(&run);
        }
    }

    fn arc_to(
        &mut self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        is_stroked: bool,
    ) {
        let arc = if is_large_arc { ArcSize::Large } else { ArcSize::Small };
        let sweep =
            if sweep_direction == SweepDirection::Clockwise { PathDirection::CW } else { PathDirection::CCW };
        let radii = (size.width as f32, size.height as f32);
        let end = to_sk_point(point);

        if is_stroked {
            self.stroke.arc_to_radius(radii, rotation_angle as f32, arc, sweep, end);
        } else {
            self.break_figure();
            self.stroke.move_to(end);
        }

        if self.duplicate() {
            self.fill().arc_to_radius(radii, rotation_angle as f32, arc, sweep, end);
        }
    }

    fn cubic_bezier_to(&mut self, point1: Point, point2: Point, point3: Point, is_stroked: bool) {
        let (p1, p2, p3) = (to_sk_point(point1), to_sk_point(point2), to_sk_point(point3));

        if is_stroked {
            self.stroke.cubic_to(p1, p2, p3);
        } else {
            self.break_figure();
            self.stroke.move_to(p3);
        }

        if self.duplicate() {
            self.fill().cubic_to(p1, p2, p3);
        }
    }

    fn quadratic_bezier_to(&mut self, point1: Point, point2: Point, is_stroked: bool) {
        let (p1, p2) = (to_sk_point(point1), to_sk_point(point2));

        if is_stroked {
            self.stroke.quad_to(p1, p2);
        } else {
            self.break_figure();
            self.stroke.move_to(p2);
        }

        if self.duplicate() {
            self.fill().quad_to(p1, p2);
        }
    }
}

#[cfg(test)]
mod run_of_lines_tests {
    // Not from upstream.
    use super::*;
    use ferroui_base::platform::IStreamGeometryBuilder;

    /// Section 9.95: a figure given to the builder as one run of lines is the path of the figure
    /// given point by point, for the stroke and for the fill, filled or not, closed or open.
    #[test]
    fn a_run_of_lines_is_the_path_of_its_lines() {
        let points: Vec<Point> =
            (0..48).map(|index| Point::new(f64::from(index) * 3.0, f64::from((index * 7) % 13))).collect();
        for count in [1, 2, 3, 48] {
            for (is_filled, is_closed) in [(true, true), (false, false), (true, false), (false, true)] {
                let build = |in_one_run: bool| {
                    let mut builder = StreamGeometryBuilder::new(None);
                    builder.begin_figure(points[0], is_filled);
                    if in_one_run {
                        builder.lines_to(&points[1..count], true);
                    } else {
                        for point in &points[1..count] {
                            builder.line_to(*point, true);
                        }
                    }
                    builder.end_figure(is_closed);
                    let geometry = builder.to_geometry();
                    let geometry = try_get_geometry_impl(&*geometry).expect("a geometry of this backend");
                    let fill = match geometry.fill() {
                        FillPath::SameAsStroke => (0, None),
                        FillPath::Separate(path) => (1, Some(path)),
                        FillPath::None => (2, None),
                    };
                    (geometry.stroke_path(), fill)
                };
                assert!(
                    build(true) == build(false),
                    "{count} points, filled {is_filled}, closed {is_closed}"
                );
            }
        }
    }
}
