//! Tests of `Quaternion`: axis and angle round trips, conjugate, inverse and normalization,
//! spherical interpolation, the product and the text form.

use super::*;

const EPSILON: f64 = 1e-12;

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < EPSILON, "{actual} is not {expected}");
}

fn assert_vector_close(actual: Vector3D, expected: Vector3D) {
    assert!((actual - expected).length() < EPSILON, "{actual:?} is not {expected:?}");
}

fn assert_quaternion_close(actual: Quaternion, expected: Quaternion) {
    let difference = actual - expected;
    assert!(
        difference.norm_squared().sqrt() < EPSILON,
        "{actual:?} is not {expected:?}"
    );
}

fn about(x: f64, y: f64, z: f64, angle: f64) -> Quaternion {
    Quaternion::from_axis_angle(Vector3D::new(x, y, z), angle).unwrap()
}

#[test]
fn the_default_is_the_identity() {
    assert_eq!(Quaternion::default(), Quaternion::IDENTITY);
    assert_eq!(Quaternion::IDENTITY, Quaternion::new(0.0, 0.0, 0.0, 1.0));
    assert!(Quaternion::IDENTITY.is_identity());
    assert!(Quaternion::IDENTITY.is_normalized());
    assert_eq!(Quaternion::IDENTITY.angle(), 0.0);
    // The axis of no rotation is undetermined: WPF gives the Y axis.
    assert_eq!(Quaternion::IDENTITY.axis(), Vector3D::new(0.0, 1.0, 0.0));
    assert_eq!(Quaternion::new(0.0, 0.0, 0.0, -3.0).axis(), Vector3D::new(0.0, 1.0, 0.0));
    assert!(!Quaternion::new(0.0, 0.0, 0.0, 2.0).is_identity());
}

#[test]
fn from_axis_angle_against_hand_values() {
    // 90 degrees about Z: (0, 0, sin 45, cos 45); the length of the axis does not matter.
    let quarter = about(0.0, 0.0, 2.0, 90.0);
    let half = std::f64::consts::FRAC_1_SQRT_2;
    assert_quaternion_close(quarter, Quaternion::new(0.0, 0.0, half, half));
    assert!(quarter.is_normalized());

    // 180 degrees about X: (1, 0, 0, 0).
    assert_quaternion_close(about(1.0, 0.0, 0.0, 180.0), Quaternion::new(1.0, 0.0, 0.0, 0.0));

    // The angle is taken modulo 360 first: 360 is no rotation, 450 is 90.
    assert_quaternion_close(about(0.0, 0.0, 1.0, 360.0), Quaternion::IDENTITY);
    assert_quaternion_close(about(0.0, 0.0, 1.0, 450.0), quarter);

    assert_eq!(
        Quaternion::from_axis_angle(Vector3D::default(), 45.0),
        Err(QuaternionError::ZeroAxis)
    );
}

#[test]
fn axis_and_angle_round_trip() {
    let axis = {
        let mut axis = Vector3D::new(1.0, 2.0, 3.0);
        axis.normalize();
        axis
    };
    for angle in [30.0, 90.0, 135.0, 179.0, 270.0] {
        let quaternion = Quaternion::from_axis_angle(axis, angle).unwrap();
        assert!((quaternion.angle() - angle).abs() < 1e-10, "{angle}: {}", quaternion.angle());
        assert_vector_close(quaternion.axis(), axis);
    }
    // A negative angle is the positive angle about the reversed axis.
    let negative = Quaternion::from_axis_angle(axis, -90.0).unwrap();
    assert_close(negative.angle(), 90.0);
    assert_vector_close(negative.axis(), -axis);
    // The angle does not depend on the length of the quaternion.
    assert_close(Quaternion::new(0.0, 0.0, 2.0, 2.0).angle(), 90.0);
}

#[test]
fn conjugate_invert_and_normalize() {
    let mut quaternion = Quaternion::new(1.0, -2.0, 3.0, 4.0);
    quaternion.conjugate();
    assert_eq!(quaternion, Quaternion::new(-1.0, 2.0, -3.0, 4.0));

    // The inverse of a rotation undoes it.
    let rotation = about(1.0, 1.0, 0.0, 60.0);
    let mut inverse = rotation;
    inverse.invert();
    assert_quaternion_close(rotation * inverse, Quaternion::IDENTITY);
    assert_quaternion_close(inverse * rotation, Quaternion::IDENTITY);
    // Of any quaternion: the conjugate over the squared norm.
    let mut scaled = Quaternion::new(0.0, 0.0, 0.0, 2.0);
    scaled.invert();
    assert_eq!(scaled, Quaternion::new(0.0, 0.0, 0.0, 0.5));

    let mut quaternion = Quaternion::new(0.0, 0.0, 3.0, 4.0);
    assert!(!quaternion.is_normalized());
    quaternion.normalize();
    assert_quaternion_close(quaternion, Quaternion::new(0.0, 0.0, 0.6, 0.8));
    assert!(quaternion.is_normalized());
    // Components whose squares overflow are normalized too.
    let mut huge = Quaternion::new(1e200, 0.0, 0.0, 1e200);
    huge.normalize();
    let half = std::f64::consts::FRAC_1_SQRT_2;
    assert_quaternion_close(huge, Quaternion::new(half, 0.0, 0.0, half));
}

#[test]
fn slerp_endpoints_and_midpoint() {
    let from = Quaternion::IDENTITY;
    let to = about(0.0, 0.0, 1.0, 90.0);
    assert_quaternion_close(Quaternion::slerp(from, to, 0.0), from);
    assert_quaternion_close(Quaternion::slerp(from, to, 1.0), to);
    // Halfway is half the angle about the same axis.
    let middle = Quaternion::slerp(from, to, 0.5);
    assert_quaternion_close(middle, about(0.0, 0.0, 1.0, 45.0));
    assert_close(middle.angle(), 45.0);

    // Between two arbitrary rotations the result stays a unit quaternion.
    let a = about(1.0, 2.0, 3.0, 20.0);
    let b = about(-1.0, 0.5, 0.0, 110.0);
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let norm_squared = Quaternion::slerp(a, b, t).norm_squared();
        assert!((norm_squared - 1.0).abs() < EPSILON, "{t}: {norm_squared}");
    }
    assert_quaternion_close(Quaternion::slerp(a, b, 0.0), a);
    assert_quaternion_close(Quaternion::slerp(a, b, 1.0), b);
}

#[test]
fn slerp_with_and_without_the_shortest_path() {
    // 270 degrees about Z is -90 degrees the short way.
    let from = Quaternion::IDENTITY;
    let to = about(0.0, 0.0, 1.0, 270.0);
    let short = Quaternion::slerp_with_path(from, to, 0.5, true);
    assert_quaternion_close(short, about(0.0, 0.0, -1.0, 45.0));
    assert_eq!(Quaternion::slerp(from, to, 0.5), short);
    // The long way: halfway to 270 degrees is 135 degrees.
    let long = Quaternion::slerp_with_path(from, to, 0.5, false);
    assert_quaternion_close(long, about(0.0, 0.0, 1.0, 135.0));
    // Both end at the target rotation: the long way at `to`, the short way at `-to`, the same
    // rotation.
    assert_quaternion_close(Quaternion::slerp_with_path(from, to, 1.0, false), to);
    let mut negated = to;
    negated.x = -negated.x;
    negated.y = -negated.y;
    negated.z = -negated.z;
    negated.w = -negated.w;
    assert_quaternion_close(Quaternion::slerp_with_path(from, to, 1.0, true), negated);

    // Opposite quaternions (the same rotation, the long way) turn about a perpendicular.
    let opposite = Quaternion::new(0.0, 0.0, 0.0, -1.0);
    let turned = Quaternion::slerp_with_path(from, opposite, 0.5, false);
    assert_quaternion_close(turned, Quaternion::new(0.0, 0.0, -1.0, 0.0));
}

#[test]
fn slerp_interpolates_the_length_geometrically() {
    let from = Quaternion::new(0.0, 0.0, 0.0, 2.0);
    let to = Quaternion::new(0.0, 0.0, 0.0, 8.0);
    // Equal directions: the linear branch, at the length 2 ^ (1 - t) * 8 ^ t.
    assert_quaternion_close(Quaternion::slerp(from, to, 0.5), Quaternion::new(0.0, 0.0, 0.0, 4.0));
}

#[test]
fn operators() {
    let a = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    let b = Quaternion::new(0.5, 0.5, 0.5, 0.5);
    assert_eq!(a + b, Quaternion::new(1.5, 2.5, 3.5, 4.5));
    assert_eq!(a - b, Quaternion::new(0.5, 1.5, 2.5, 3.5));
    // The identity is neutral for the product.
    assert_eq!(Quaternion::IDENTITY * a, a);
    assert_eq!(a * Quaternion::IDENTITY, a);
    // Hand values: i * j = k, j * i = -k.
    let i = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    let j = Quaternion::new(0.0, 1.0, 0.0, 0.0);
    assert_eq!(i * j, Quaternion::new(0.0, 0.0, 1.0, 0.0));
    assert_eq!(j * i, Quaternion::new(0.0, 0.0, -1.0, 0.0));
    // Two quarter turns about one axis make a half turn.
    let quarter = about(0.0, 0.0, 1.0, 90.0);
    assert_quaternion_close(quarter * quarter, Quaternion::new(0.0, 0.0, 1.0, 0.0));
}

#[test]
fn parse_and_format_round_trip() {
    assert_eq!(Quaternion::parse("Identity").unwrap(), Quaternion::IDENTITY);
    assert_eq!(Quaternion::parse("0,0,0,1").unwrap(), Quaternion::IDENTITY);
    assert_eq!(Quaternion::IDENTITY.to_string(), "Identity");
    assert_eq!(Quaternion::new(0.0, 0.0, 0.0, 1.0).to_string(), "Identity");

    let quaternion = Quaternion::new(1.0, -2.0, 0.5, 4.0);
    assert_eq!(quaternion.to_string(), "1,-2,0.5,4");
    assert_eq!(Quaternion::parse("1 -2 0.5 4").unwrap(), quaternion);
    assert_eq!(quaternion.to_string().parse::<Quaternion>().unwrap(), quaternion);
    let rotation = about(1.0, 2.0, 3.0, 40.0);
    assert_eq!(Quaternion::parse(&rotation.to_string()).unwrap(), rotation);

    for text in ["", "1,2,3", "1,2,3,4,5", "identity", "Identity,"] {
        assert_eq!(Quaternion::parse(text).unwrap_err().message(), "Invalid Quaternion.", "{text:?}");
    }
}
