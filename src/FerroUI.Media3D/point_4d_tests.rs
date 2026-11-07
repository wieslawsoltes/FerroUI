//! Tests of `Point4D`: the operators, the projective transform and the text form.

use super::*;
use crate::{Point3D, Vector3D};

#[test]
fn operators_and_offset() {
    let a = Point4D::new(1.0, 2.0, 3.0, 4.0);
    let b = Point4D::new(0.5, 0.5, 0.5, 0.5);
    assert_eq!(a + b, Point4D::new(1.5, 2.5, 3.5, 4.5));
    assert_eq!(a - b, Point4D::new(0.5, 1.5, 2.5, 3.5));
    let mut c = a;
    c.offset(1.0, 1.0, 1.0, -4.0);
    assert_eq!(c, Point4D::new(2.0, 3.0, 4.0, 0.0));
    c += b;
    c -= b;
    assert_eq!(c, Point4D::new(2.0, 3.0, 4.0, 0.0));
}

/// A perspective matrix (`m34 = -1/d` puts the eye at distance `d` on the Z axis, looking down
/// the negative Z axis): a `Point4D` keeps its homogeneous `w`, a `Point3D` is divided by it.
#[test]
fn a_projective_matrix_transforms_point4d_without_the_divide() {
    let mut perspective = Matrix3D::IDENTITY;
    perspective.m34 = -0.25;
    perspective.translate(Vector3D::new(1.0, 0.0, 0.0));
    assert!(!perspective.is_affine());

    let point = Point4D::new(2.0, 4.0, -4.0, 1.0);
    // The translation is appended after the projection, so it moves the homogeneous coordinates
    // by w' times the offset: x' = 2 + 1 * 2 = 4, and w' = z * m34 + w * m44 = -4 * -0.25 + 1 = 2.
    assert_eq!(point * perspective, Point4D::new(4.0, 4.0, -4.0, 2.0));
    assert_eq!(perspective.transform_point4d(point), Point4D::new(4.0, 4.0, -4.0, 2.0));
    // The same point as a Point3D is divided by w'.
    assert_eq!(perspective.transform(Point3D::new(2.0, 4.0, -4.0)), Point3D::new(2.0, 2.0, -2.0));

    // A point at infinity (w = 0) is a direction: the translation does not apply to it.
    let direction = Point4D::new(1.0, 0.0, 0.0, 0.0);
    assert_eq!(perspective.transform_point4d(direction), Point4D::new(1.0, 0.0, 0.0, 0.0));

    let mut points = [point, direction];
    perspective.transform_points4d(&mut points);
    assert_eq!(points, [Point4D::new(4.0, 4.0, -4.0, 2.0), Point4D::new(1.0, 0.0, 0.0, 0.0)]);
}

#[test]
fn parse_and_format_round_trip() {
    assert_eq!(Point4D::parse("1,2,3,4").unwrap(), Point4D::new(1.0, 2.0, 3.0, 4.0));
    assert_eq!(Point4D::parse("1 2 3 4").unwrap(), Point4D::new(1.0, 2.0, 3.0, 4.0));
    let point = Point4D::new(0.5, -1.0, 2.0, 1.0);
    assert_eq!(point.to_string(), "0.5,-1,2,1");
    assert_eq!(point.to_string().parse::<Point4D>().unwrap(), point);
    for text in ["1,2,3", "1,2,3,4,5", ""] {
        assert_eq!(Point4D::parse(text).unwrap_err().message(), "Invalid Point4D.", "{text:?}");
    }
}
