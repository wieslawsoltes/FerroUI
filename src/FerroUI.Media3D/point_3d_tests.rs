//! Tests of `Point3D`: the operators with vectors and matrices, the conversions and the text form.

use super::*;

#[test]
fn operators_with_vectors() {
    let point = Point3D::new(1.0, 2.0, 3.0);
    let vector = Vector3D::new(10.0, 20.0, 30.0);
    assert_eq!(point + vector, Point3D::new(11.0, 22.0, 33.0));
    assert_eq!(point - vector, Point3D::new(-9.0, -18.0, -27.0));
    // The difference of two points is the vector between them.
    assert_eq!(Point3D::new(4.0, 6.0, 8.0) - point, Vector3D::new(3.0, 4.0, 5.0));

    let mut moved = point;
    moved += vector;
    moved -= Vector3D::new(1.0, 1.0, 1.0);
    assert_eq!(moved, Point3D::new(10.0, 21.0, 32.0));
}

#[test]
fn offset_moves_the_point() {
    let mut point = Point3D::new(1.0, 2.0, 3.0);
    point.offset(0.5, -2.0, 7.0);
    assert_eq!(point, Point3D::new(1.5, 0.0, 10.0));
}

#[test]
fn multiplication_by_a_matrix_transforms_the_point() {
    let mut matrix = Matrix3D::IDENTITY;
    matrix.translate(Vector3D::new(1.0, 2.0, 3.0));
    assert_eq!(Point3D::new(1.0, 1.0, 1.0) * matrix, Point3D::new(2.0, 3.0, 4.0));
}

#[test]
fn conversions() {
    let point = Point3D::new(1.0, -2.0, 3.0);
    assert_eq!(Vector3D::from(point), Vector3D::new(1.0, -2.0, 3.0));
    assert_eq!(Point4D::from(point), Point4D::new(1.0, -2.0, 3.0, 1.0));
}

#[test]
fn parse_and_format_round_trip() {
    for text in ["0,0,5", "0 0 5", "0, 0, 5", " 0,0 5 "] {
        assert_eq!(Point3D::parse(text).unwrap(), Point3D::new(0.0, 0.0, 5.0), "{text:?}");
    }
    let point = Point3D::new(-1.25, 1e-5, 12345.5);
    assert_eq!(point.to_string(), "-1.25,1E-05,12345.5");
    assert_eq!(point.to_string().parse::<Point3D>().unwrap(), point);
    assert_eq!(Point3D::new(f64::INFINITY, f64::NEG_INFINITY, 0.0).to_string(), "Infinity,-Infinity,0");
    assert_eq!(
        Point3D::parse("Infinity,-Infinity,0").unwrap(),
        Point3D::new(f64::INFINITY, f64::NEG_INFINITY, 0.0)
    );

    for text in ["", "1", "1,2", "1,2,3,4", "1;2;3", "x,y,z"] {
        assert_eq!(Point3D::parse(text).unwrap_err().message(), "Invalid Point3D.", "{text:?}");
    }
}
