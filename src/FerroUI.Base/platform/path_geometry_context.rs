use crate::media::{
    ArcSegment, BezierSegment, FillRule, LineSegment, PathFigure, PathFigures, PathGeometry, PathSegments,
    QuadraticBezierSegment, SweepDirection,
};
use crate::platform::IGeometryContext;
use crate::{Point, Ref, Size};

/// A geometry context that records the drawing commands as the figures and
/// segments of a [`PathGeometry`].
pub struct PathGeometryContext {
    current_figure: Option<Ref<PathFigure>>,
    path_geometry: Option<Ref<PathGeometry>>,
}

impl PathGeometryContext {
    pub fn new(path_geometry: Ref<PathGeometry>) -> Self {
        Self { current_figure: None, path_geometry: Some(path_geometry) }
    }

    fn path_geometry(&self) -> &Ref<PathGeometry> {
        self.path_geometry.as_ref().expect("PathGeometryContext is disposed")
    }

    fn current_figure_segments(&self) -> PathSegments {
        let _ = self.path_geometry();
        let Some(figure) = &self.current_figure else {
            panic!("No figure in progress.");
        };
        figure.segments().expect("Current figure's segments cannot be null.")
    }
}

impl IGeometryContext for PathGeometryContext {
    fn arc_to(
        &mut self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        is_stroked: bool,
    ) {
        let arc_segment = ArcSegment::new();
        arc_segment.set_size(size);
        arc_segment.set_rotation_angle(rotation_angle);
        arc_segment.set_is_large_arc(is_large_arc);
        arc_segment.set_sweep_direction(sweep_direction);
        arc_segment.set_point(point);
        arc_segment.set_is_stroked(is_stroked);
        self.current_figure_segments().add(arc_segment.upcast());
    }

    fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
        let path_geometry = self.path_geometry().clone();

        let figure = PathFigure::new();
        figure.set_start_point(start_point);
        figure.set_is_closed(false);
        figure.set_is_filled(is_filled);

        let figures = match path_geometry.figures() {
            Some(figures) => figures,
            None => {
                let figures = PathFigures::new();
                path_geometry.set_figures(Some(figures.clone()));
                figures
            }
        };
        figures.add(figure.clone());
        self.current_figure = Some(figure);
    }

    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
        let bezier_segment = BezierSegment::new();
        bezier_segment.set_point1(control_point1);
        bezier_segment.set_point2(control_point2);
        bezier_segment.set_point3(end_point);
        bezier_segment.set_is_stroked(is_stroked);
        self.current_figure_segments().add(bezier_segment.upcast());
    }

    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, is_stroked: bool) {
        let quadratic_bezier_segment = QuadraticBezierSegment::new();
        quadratic_bezier_segment.set_point1(control_point);
        quadratic_bezier_segment.set_point2(end_point);
        quadratic_bezier_segment.set_is_stroked(is_stroked);
        self.current_figure_segments().add(quadratic_bezier_segment.upcast());
    }

    fn line_to(&mut self, point: Point, is_stroked: bool) {
        let line_segment = LineSegment::new();
        line_segment.set_point(point);
        line_segment.set_is_stroked(is_stroked);
        self.current_figure_segments().add(line_segment.upcast());
    }

    fn end_figure(&mut self, is_closed: bool) {
        if let Some(figure) = &self.current_figure {
            figure.set_is_closed(is_closed);
        }

        self.current_figure = None;
    }

    fn set_fill_rule(&mut self, fill_rule: FillRule) {
        self.path_geometry().set_fill_rule(fill_rule);
    }

    fn dispose(&mut self) {
        self.path_geometry = None;
    }
}
