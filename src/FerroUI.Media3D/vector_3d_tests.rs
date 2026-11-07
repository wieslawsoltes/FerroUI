//! Tests of `Vector3D`: WPF's documented arithmetic, normalization, products and angles, and the
//! text form.

use super::*;

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-12, "{actual} is not {expected}");
}

#[test]
fn length_and_squared_length() {
    let vector = Vector3D::new(2.0, 3.0, 6.0);
    assert_eq!(vector.length_squared(), 49.0);
    assert_eq!(vector.length(), 7.0);
    assert_eq!(Vector3D::default().length(), 0.0);
}

#[test]
fn normalize_makes_a_unit_vector_and_survives_large_components() {
    let mut vector = Vector3D::new(0.0, 0.0, 5.0);
    vector.normalize();
    assert_eq!(vector, Vector3D::new(0.0, 0.0, 1.0));

    let mut vector = Vector3D::new(3.0, 4.0, 0.0);
    vector.normalize();
    assert_close(vector.x, 0.6);
    assert_close(vector.y, 0.8);
    assert_eq!(vector.z, 0.0);

    // The squared length of this vector overflows; the division by the largest component first
    // keeps the result finite.
    let mut huge = Vector3D::new(1e300, 1e300, 0.0);
    huge.normalize();
    assert_close(huge.x, std::f64::consts::FRAC_1_SQRT_2);
    assert_close(huge.y, std::f64::consts::FRAC_1_SQRT_2);

    // The zero vector has no direction.
    let mut zero = Vector3D::default();
    zero.normalize();
    assert!(zero.x.is_nan() && zero.y.is_nan() && zero.z.is_nan());
}

#[test]
fn negate_reverses_the_vector() {
    let mut vector = Vector3D::new(1.0, -2.0, 3.0);
    vector.negate();
    assert_eq!(vector, Vector3D::new(-1.0, 2.0, -3.0));
}

#[test]
fn dot_and_cross_products() {
    let a = Vector3D::new(1.0, 2.0, 3.0);
    let b = Vector3D::new(4.0, 5.0, 6.0);
    assert_eq!(Vector3D::dot_product(a, b), 32.0);
    assert_eq!(Vector3D::cross_product(a, b), Vector3D::new(-3.0, 6.0, -3.0));

    // Right-handed: X cross Y is Z, and the product is perpendicular to both factors.
    let x = Vector3D::new(1.0, 0.0, 0.0);
    let y = Vector3D::new(0.0, 1.0, 0.0);
    assert_eq!(Vector3D::cross_product(x, y), Vector3D::new(0.0, 0.0, 1.0));
    assert_eq!(Vector3D::cross_product(y, x), Vector3D::new(0.0, 0.0, -1.0));
    let cross = Vector3D::cross_product(a, b);
    assert_eq!(Vector3D::dot_product(cross, a), 0.0);
    assert_eq!(Vector3D::dot_product(cross, b), 0.0);
}

#[test]
fn angle_between_is_in_degrees_and_precise_at_the_ends() {
    let x = Vector3D::new(1.0, 0.0, 0.0);
    assert_close(Vector3D::angle_between(x, Vector3D::new(0.0, 3.0, 0.0)), 90.0);
    assert_close(Vector3D::angle_between(x, Vector3D::new(-2.0, 0.0, 0.0)), 180.0);
    assert_close(Vector3D::angle_between(x, Vector3D::new(1.0, 1.0, 0.0)), 45.0);
    assert_close(Vector3D::angle_between(x, Vector3D::new(-1.0, 1.0, 0.0)), 135.0);
    assert_eq!(Vector3D::angle_between(x, Vector3D::new(4.0, 0.0, 0.0)), 0.0);
    // A tiny angle, where the arc cosine of the dot product would round to zero.
    let tiny = Vector3D::angle_between(x, Vector3D::new(1.0, 1e-9, 0.0));
    assert!((tiny - 1e-9f64.to_degrees()).abs() < 1e-18, "{tiny}");
}

#[test]
fn operators_mirror_wpf() {
    let a = Vector3D::new(1.0, 2.0, 3.0);
    let b = Vector3D::new(4.0, 5.0, 6.0);
    assert_eq!(a + b, Vector3D::new(5.0, 7.0, 9.0));
    assert_eq!(b - a, Vector3D::new(3.0, 3.0, 3.0));
    assert_eq!(-a, Vector3D::new(-1.0, -2.0, -3.0));
    assert_eq!(a * 2.0, Vector3D::new(2.0, 4.0, 6.0));
    assert_eq!(2.0 * a, Vector3D::new(2.0, 4.0, 6.0));
    assert_eq!(b / 2.0, Vector3D::new(2.0, 2.5, 3.0));

    // A vector and a point make a point.
    let point = Point3D::new(10.0, 20.0, 30.0);
    assert_eq!(a + point, Point3D::new(11.0, 22.0, 33.0));
    assert_eq!(a - point, Point3D::new(-9.0, -18.0, -27.0));

    let mut c = a;
    c += b;
    c -= a;
    c *= 2.0;
    c /= 4.0;
    assert_eq!(c, Vector3D::new(2.0, 2.5, 3.0));
}

#[test]
fn conversions() {
    let vector = Vector3D::new(-1.0, 2.0, -3.0);
    assert_eq!(Point3D::from(vector), Point3D::new(-1.0, 2.0, -3.0));
    // A size is never negative: the magnitudes of the components.
    let size = Size3D::from(vector);
    assert_eq!((size.x(), size.y(), size.z()), (1.0, 2.0, 3.0));
    // The base library's composition vector.
    let base = ferroui_base::Vector3D::from(vector);
    assert_eq!((base.x, base.y, base.z), (-1.0, 2.0, -3.0));
    assert_eq!(Vector3D::from(base), vector);
}

#[test]
fn parse_and_format_round_trip() {
    for text in ["1,2,3", "1 2 3", "1, 2, 3", "  1 ,2 , 3  "] {
        assert_eq!(Vector3D::parse(text).unwrap(), Vector3D::new(1.0, 2.0, 3.0), "{text:?}");
    }
    assert_eq!("-0.5,1E-07,1E+21".parse::<Vector3D>().unwrap(), Vector3D::new(-0.5, 1e-7, 1e21));

    let vector = Vector3D::new(1.5, -2.25, 1e-7);
    assert_eq!(vector.to_string(), "1.5,-2.25,1E-07");
    assert_eq!(Vector3D::parse(&vector.to_string()).unwrap(), vector);
    let third = Vector3D::new(1.0 / 3.0, 0.1, 0.2);
    assert_eq!(Vector3D::parse(&third.to_string()).unwrap(), third);

    for text in ["", "1,2", "1,2,3,4", "1,,2,3", "1,2,3,", "a,b,c", "Identity"] {
        let error = Vector3D::parse(text).unwrap_err();
        assert_eq!(error.message(), "Invalid Vector3D.", "{text:?}");
    }
}
