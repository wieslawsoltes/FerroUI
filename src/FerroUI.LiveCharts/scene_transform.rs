//! Translation of a measured chart into a host-allocated drawing rectangle.

use crate::{ChartError, Geometry, HitShape, PathCommand, Scene};
use ferroui_base::{Point, Rect, Size};

fn move_point(point: &mut Point, offset: Point) {
    point.x += offset.x;
    point.y += offset.y;
}
fn move_rect(rect: &mut Rect, offset: Point) {
    rect.x += offset.x;
    rect.y += offset.y;
}
fn move_shape(shape: &mut HitShape, offset: Point) {
    match shape {
        HitShape::Rectangle(rect) => move_rect(rect, offset),
        HitShape::Circle { center, .. } | HitShape::AnnularSector { center, .. } => {
            move_point(center, offset)
        }
        HitShape::Polygon(points) => points
            .iter_mut()
            .for_each(|point| move_point(point, offset)),
        HitShape::CompoundPolygon(rings) => rings
            .iter_mut()
            .flatten()
            .for_each(|point| move_point(point, offset)),
        HitShape::WithHoverArea { shape, bounds } => {
            move_shape(shape, offset);
            move_rect(bounds, offset);
        }
        HitShape::Compound(shapes) => shapes
            .iter_mut()
            .for_each(|shape| move_shape(shape, offset)),
    }
}

impl Scene {
    /// Places an inner measurement in control coordinates, preserving geometry,
    /// clipping, exact hit shapes, hover slots and stable entity identities.
    pub fn translate(&mut self, offset: Point, output_size: Size) -> Result<(), ChartError> {
        crate::scene::validate_size(output_size)?;
        if !offset.x.is_finite() || !offset.y.is_finite() {
            return Err(ChartError::InvalidData(
                "scene translation must be finite".into(),
            ));
        }
        for primitive in &mut self.primitives {
            if let Some(clip) = &mut primitive.clip {
                move_rect(clip, offset);
            }
            match &mut primitive.geometry {
                Geometry::Line { start, end } => {
                    move_point(start, offset);
                    move_point(end, offset);
                }
                Geometry::Polyline(points) | Geometry::Polygon(points) => {
                    points
                        .iter_mut()
                        .for_each(|point| move_point(point, offset));
                }
                Geometry::Rectangle { rect, .. } | Geometry::Ellipse(rect) => {
                    move_rect(rect, offset)
                }
                Geometry::Text { origin, .. } => move_point(origin, offset),
                Geometry::Path(commands) => {
                    for command in commands {
                        match command {
                            PathCommand::MoveTo(point) | PathCommand::LineTo(point) => {
                                move_point(point, offset)
                            }
                            PathCommand::CubicTo {
                                control1,
                                control2,
                                end,
                            } => {
                                move_point(control1, offset);
                                move_point(control2, offset);
                                move_point(end, offset);
                            }
                            PathCommand::Close => {}
                        }
                    }
                }
            }
        }
        for hit in &mut self.hits {
            move_point(&mut hit.center, offset);
            move_shape(&mut hit.shape, offset);
        }
        move_rect(&mut self.plot_bounds, offset);
        self.size = output_size;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FindingStrategy, HitRegion, PaintStyle, PointId, Primitive};

    #[test]
    fn translating_a_map_preserves_polygon_holes_hit_testing_and_clip() {
        let ring = |a: f64, b: f64| {
            vec![
                Point::new(a, a),
                Point::new(b, a),
                Point::new(b, b),
                Point::new(a, b),
            ]
        };
        let mut scene = Scene::new(Size::new(100.0, 100.0), Rect::new(0.0, 0.0, 100.0, 100.0));
        scene.hits.push(HitRegion {
            id: PointId {
                series: 0,
                point: 0,
            },
            shape: HitShape::CompoundPolygon(vec![ring(0.0, 100.0), ring(40.0, 60.0)]),
            center: Point::new(20.0, 20.0),
            value: 1.0,
            label: "land".into(),
        });
        scene.primitives.push(
            Primitive::new(
                Geometry::Ellipse(Rect::new(10.0, 10.0, 20.0, 20.0)),
                PaintStyle::default(),
            )
            .clipped(scene.plot_bounds),
        );
        scene
            .translate(Point::new(70.0, 50.0), Size::new(240.0, 180.0))
            .unwrap();
        assert_eq!(scene.plot_bounds, Rect::new(70.0, 50.0, 100.0, 100.0));
        assert_eq!(scene.primitives[0].clip, Some(scene.plot_bounds));
        assert_eq!(
            scene
                .hit_test(Point::new(90.0, 70.0), FindingStrategy::ExactMatch, 0.0)
                .len(),
            1
        );
        assert!(scene
            .hit_test(Point::new(120.0, 100.0), FindingStrategy::ExactMatch, 0.0)
            .is_empty());
        assert!(scene
            .translate(Point::new(f64::NAN, 0.0), Size::new(240.0, 180.0))
            .is_err());
    }
}
