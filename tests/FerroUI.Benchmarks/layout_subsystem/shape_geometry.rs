//! The geometry of shapes whose geometry properties change: a canvas of shapes, a change of what
//! the geometry of each is made of, a layout pass, and the geometry each draws with (section
//! 9.88 of the performance notes: a shape keeps its geometry and gives it what it is now, where
//! it made another geometry for every change).
//!
//! - `polyline_points_replaced`: polylines given another collection of points, as a sparkline
//!   that is drawn again is;
//! - `polygon_points_replaced`: the same for polygons;
//! - `rectangle_resized`, `ellipse_resized`: shapes given another width;
//! - `line_moved`: lines whose end point moves;
//! - `arc_swept`, `sector_swept`: arcs and sectors given another sweep angle, as a progress ring
//!   that is animated is (a stream geometry, #805);
//! - `border_unequal_resized`: borders with unequal corners and thicknesses given another width
//!   and drawn: the render helper of the border builds two stream geometries for such a border
//!   and builds them again for every size (#805). Unequal corners with one thickness are drawn as
//!   a rounded rectangle by a backend that draws each corner, and make no geometry.

use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::media::{DrawingGroup, Points};
use ferroui_base::{CornerRadius, Point, Ref, Thickness};
use ferroui_controls::shapes::{Arc, Ellipse, Line, Polygon, Polyline, Rectangle, Sector, Shape};
use ferroui_controls::{Border, Canvas, Control};

/// The shapes of the canvas.
const SHAPES: usize = 500;

/// The points of a polyline or polygon.
const POINTS: usize = 24;

pub struct ShapeGeometry {
    fixture: Fixture,
    elements: Vec<Ref<Control>>,
    change: fn(&Ref<Control>, usize),
    /// Whether the elements are drawn after the layout (a border makes its geometries when it is
    /// drawn); a shape is asked for the geometry it draws with instead.
    drawn: bool,
    /// How often the shapes were changed: each change differs from the one before.
    step: usize,
}

fn points(step: usize) -> Points {
    Points::from_items((0..POINTS).map(|index| {
        let height = ((index * 7 + step * 3) % 11) as f64;
        Point::new(index as f64 * 2.0, height)
    }))
}

/// A sweep angle of a step: each differs from the one before, under a full turn.
fn sweep(step: usize) -> f64 {
    30.0 + (step % 11) as f64 * 25.0
}

impl ShapeGeometry {
    fn shown(make: fn() -> Ref<Control>, change: fn(&Ref<Control>, usize), drawn: bool) -> Self {
        let fixture = Fixture::new();
        let canvas = Canvas::new();
        let elements: Vec<Ref<Control>> = (0..SHAPES)
            .map(|index| {
                let element = make();
                if let Some(shape) = element.cast::<Shape>() {
                    shape.set_stroke(Some(fixture.brushes.tile.clone()));
                } else if let Some(border) = element.cast::<Border>() {
                    border.set_border_brush(Some(fixture.brushes.tile.clone()));
                    border.set_background(Some(fixture.brushes.other.clone()));
                }
                Canvas::set_left(&element, (index % 25) as f64 * 50.0);
                Canvas::set_top(&element, (index / 25) as f64 * 14.0);
                canvas.children().add(&element);
                element
            })
            .collect();
        fixture.show(Some(&canvas.upcast::<Control>()));
        let mut scene = Self { fixture, elements, change, drawn, step: 0 };
        scene.change_and_lay_out();
        scene
    }

    fn polyline() -> Self {
        Self::shown(
            || Polyline::new().upcast(),
            |shape, step| shape.cast::<Polyline>().expect("a polyline").set_points(Some(points(step))),
            false,
        )
    }

    fn polygon() -> Self {
        Self::shown(
            || Polygon::new().upcast(),
            |shape, step| shape.cast::<Polygon>().expect("a polygon").set_points(Some(points(step))),
            false,
        )
    }

    fn rectangle() -> Self {
        Self::shown(
            || {
                let shape = Rectangle::new();
                shape.set_height(10.0);
                shape.set_radius_x(2.0);
                shape.set_radius_y(2.0);
                shape.upcast()
            },
            |shape, step| shape.set_width(20.0 + (step % 2) as f64 * 16.0),
            false,
        )
    }

    fn ellipse() -> Self {
        Self::shown(
            || {
                let shape = Ellipse::new();
                shape.set_height(10.0);
                shape.upcast()
            },
            |shape, step| shape.set_width(20.0 + (step % 2) as f64 * 16.0),
            false,
        )
    }

    fn line() -> Self {
        Self::shown(
            || Line::new().upcast(),
            |shape, step| {
                shape.cast::<Line>().expect("a line").set_end_point(Point::new(40.0, (step % 2) as f64 * 10.0));
            },
            false,
        )
    }

    fn arc() -> Self {
        Self::shown(
            || {
                let shape = Arc::new();
                shape.set_width(12.0);
                shape.set_height(12.0);
                shape.set_start_angle(-90.0);
                shape.upcast()
            },
            |shape, step| shape.cast::<Arc>().expect("an arc").set_sweep_angle(sweep(step)),
            false,
        )
    }

    fn sector() -> Self {
        Self::shown(
            || {
                let shape = Sector::new();
                shape.set_width(12.0);
                shape.set_height(12.0);
                shape.set_start_angle(-90.0);
                shape.upcast()
            },
            |shape, step| shape.cast::<Sector>().expect("a sector").set_sweep_angle(sweep(step)),
            false,
        )
    }

    fn border() -> Self {
        Self::shown(
            || {
                let border = Border::new();
                border.set_height(12.0);
                border.set_border_thickness(Thickness::new(1.0, 2.0, 1.0, 2.0));
                border.set_corner_radius(CornerRadius::new(2.0, 4.0, 6.0, 1.0));
                border.upcast()
            },
            |border, step| border.set_width(20.0 + (step % 2) as f64 * 16.0),
            true,
        )
    }

    /// Changes every shape, lays the canvas out and asks each shape for the geometry it draws, or
    /// draws each element.
    fn change_and_lay_out(&mut self) -> usize {
        self.step += 1;
        for element in &self.elements {
            (self.change)(element, self.step);
        }
        self.fixture.lay_out();
        if self.drawn {
            let group = DrawingGroup::new();
            let mut context = group.open();
            for element in &self.elements {
                element.render(&mut context);
            }
            drop(context);
            return group.children().len();
        }
        self.elements
            .iter()
            .filter(|element| element.cast::<Shape>().is_some_and(|shape| shape.rendered_geometry().is_some()))
            .count()
    }
}

type Setup = fn() -> ShapeGeometry;

const SCENES: [(&str, Setup); 8] = [
    ("polyline_points_replaced", ShapeGeometry::polyline),
    ("polygon_points_replaced", ShapeGeometry::polygon),
    ("rectangle_resized", ShapeGeometry::rectangle),
    ("ellipse_resized", ShapeGeometry::ellipse),
    ("line_moved", ShapeGeometry::line),
    ("arc_swept", ShapeGeometry::arc),
    ("sector_swept", ShapeGeometry::sector),
    ("border_unequal_resized", ShapeGeometry::border),
];

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout_subsystem", "ShapeGeometry").min_iteration_time(200);
    let parameters = format!("Shapes={SHAPES}");
    for (name, setup) in SCENES {
        class.benchmark(name, parameters.clone(), setup, ShapeGeometry::change_and_lay_out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "ShapeGeometry");
    }

    #[test]
    fn every_shape_has_a_geometry_after_each_change() {
        for (name, setup) in SCENES {
            let mut scene = setup();
            for _ in 0..3 {
                let counted = scene.change_and_lay_out();
                if scene.drawn {
                    // A border draws its background and its border.
                    assert!(counted >= SHAPES, "{name}: {counted}");
                } else {
                    assert_eq!(counted, SHAPES, "{name}");
                }
            }
        }
    }
}
