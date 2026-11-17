//! The geometry of shapes whose geometry properties change: a canvas of shapes, a change of what
//! the geometry of each is made of, a layout pass, and the geometry each draws with (section
//! 9.88 of the performance notes: a shape keeps its geometry and gives it what it is now, where
//! it made another geometry for every change).
//!
//! - `polyline_points_replaced`: polylines given another collection of points, as a sparkline
//!   that is drawn again is;
//! - `polygon_points_replaced`: the same for polygons;
//! - `rectangle_resized`, `ellipse_resized`: shapes given another width;
//! - `line_moved`: lines whose end point moves.

use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::media::Points;
use ferroui_base::{Point, Ref};
use ferroui_controls::shapes::{Ellipse, Line, Polygon, Polyline, Rectangle, Shape};
use ferroui_controls::{Canvas, Control};

/// The shapes of the canvas.
const SHAPES: usize = 500;

/// The points of a polyline or polygon.
const POINTS: usize = 24;

pub struct ShapeGeometry {
    fixture: Fixture,
    shapes: Vec<Ref<Shape>>,
    change: fn(&Ref<Shape>, usize),
    /// How often the shapes were changed: each change differs from the one before.
    step: usize,
}

fn points(step: usize) -> Points {
    Points::from_items((0..POINTS).map(|index| {
        let height = ((index * 7 + step * 3) % 11) as f64;
        Point::new(index as f64 * 2.0, height)
    }))
}

impl ShapeGeometry {
    fn shown(make: fn() -> Ref<Shape>, change: fn(&Ref<Shape>, usize)) -> Self {
        let fixture = Fixture::new();
        let canvas = Canvas::new();
        let shapes: Vec<Ref<Shape>> = (0..SHAPES)
            .map(|index| {
                let shape = make();
                shape.set_stroke(Some(fixture.brushes.tile.clone()));
                Canvas::set_left(&shape, (index % 25) as f64 * 50.0);
                Canvas::set_top(&shape, (index / 25) as f64 * 14.0);
                canvas.children().add(&shape);
                shape
            })
            .collect();
        fixture.show(Some(&canvas.upcast::<Control>()));
        let mut scene = Self { fixture, shapes, change, step: 0 };
        scene.change_and_lay_out();
        scene
    }

    fn polyline() -> Self {
        Self::shown(|| Polyline::new().upcast(), |shape, step| {
            shape.cast::<Polyline>().expect("a polyline").set_points(Some(points(step)));
        })
    }

    fn polygon() -> Self {
        Self::shown(|| Polygon::new().upcast(), |shape, step| {
            shape.cast::<Polygon>().expect("a polygon").set_points(Some(points(step)));
        })
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
        )
    }

    fn line() -> Self {
        Self::shown(|| Line::new().upcast(), |shape, step| {
            shape.cast::<Line>().expect("a line").set_end_point(Point::new(40.0, (step % 2) as f64 * 10.0));
        })
    }

    /// Changes every shape, lays the canvas out and asks each shape for the geometry it draws.
    fn change_and_lay_out(&mut self) -> usize {
        self.step += 1;
        for shape in &self.shapes {
            (self.change)(shape, self.step);
        }
        self.fixture.lay_out();
        self.shapes.iter().filter(|shape| shape.rendered_geometry().is_some()).count()
    }
}

type Setup = fn() -> ShapeGeometry;

const SCENES: [(&str, Setup); 5] = [
    ("polyline_points_replaced", ShapeGeometry::polyline),
    ("polygon_points_replaced", ShapeGeometry::polygon),
    ("rectangle_resized", ShapeGeometry::rectangle),
    ("ellipse_resized", ShapeGeometry::ellipse),
    ("line_moved", ShapeGeometry::line),
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
                assert_eq!(scene.change_and_lay_out(), SHAPES, "{name}");
            }
        }
    }
}
