//! Tests of the numerics types. The expected values were produced by the
//! reference runtime's numerics library for the same inputs.

#![allow(clippy::excessive_precision)]

use super::single::ieee_remainder;
use super::*;
use std::f32::consts::PI;

const NAN: f32 = f32::NAN;

#[track_caller]
fn close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        if e.is_nan() {
            assert!(a.is_nan(), "component {i}: expected NaN, got {a} ({actual:?})");
            continue;
        }
        let tolerance = 2e-6 * e.abs().max(1.0);
        assert!(
            (a - e).abs() <= tolerance,
            "component {i}: expected {e}, got {a} ({actual:?} vs {expected:?})"
        );
    }
}

fn v2(v: Vector2) -> [f32; 2] {
    [v.x, v.y]
}

fn v3(v: Vector3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

fn v4(v: Vector4) -> [f32; 4] {
    [v.x, v.y, v.z, v.w]
}

fn q(v: Quaternion) -> [f32; 4] {
    [v.x, v.y, v.z, v.w]
}

fn m3(m: Matrix3x2) -> [f32; 6] {
    [m.m11, m.m12, m.m21, m.m22, m.m31, m.m32]
}

fn m4(m: Matrix4x4) -> [f32; 16] {
    [
        m.m11, m.m12, m.m13, m.m14, m.m21, m.m22, m.m23, m.m24, m.m31, m.m32, m.m33, m.m34, m.m41, m.m42, m.m43,
        m.m44,
    ]
}

const A2: Vector2 = Vector2::new(1.5, -2.25);
const B2: Vector2 = Vector2::new(4.0, 0.5);
const A3: Vector3 = Vector3::new(1.5, -2.25, 3.0);
const B3: Vector3 = Vector3::new(4.0, 0.5, -1.0);
const A4: Vector4 = Vector4::new(1.5, -2.25, 3.0, 0.5);
const B4: Vector4 = Vector4::new(4.0, 0.5, -1.0, 2.0);

fn q1() -> Quaternion {
    Quaternion::create_from_axis_angle(Vector3::normalize(Vector3::new(1.0, 2.0, 3.0)), 0.8)
}

fn q2() -> Quaternion {
    Quaternion::create_from_yaw_pitch_roll(0.3, -0.6, 1.1)
}

fn m32() -> Matrix3x2 {
    Matrix3x2::create_rotation(0.7)
        * Matrix3x2::create_scale(Vector2::new(2.0, 3.0))
        * Matrix3x2::create_translation(Vector2::new(10.0, 20.0))
}

fn m44() -> Matrix4x4 {
    Matrix4x4::create_from_quaternion(q1())
        * Matrix4x4::create_scale(Vector3::new(2.0, 3.0, 4.0))
        * Matrix4x4::create_translation(Vector3::new(10.0, 20.0, 30.0))
}

#[test]
fn single_formatting_matches_invariant_rules() {
    let cases: [(f32, &str); 26] = [
        (1e6, "1000000"),
        (1e7, "10000000"),
        (16777216.0, "16777216"),
        (1e8, "100000000"),
        (123456789.0, "123456790"),
        (999999900.0, "999999900"),
        (1e9, "1E+09"),
        (2147483648.0, "2.1474836E+09"),
        (1.5e9, "1.5E+09"),
        (1.2345678e10, "1.2345678E+10"),
        (-2.5e20, "-2.5E+20"),
        (3.4028235e38, "3.4028235E+38"),
        (1e-4, "0.0001"),
        (0.00012345, "0.00012345"),
        (0.00099999, "0.00099999"),
        (0.000099999, "9.9999E-05"),
        (1e-5, "1E-05"),
        (1e-10, "1E-10"),
        (1.17549435e-38, "1.1754944E-38"),
        (1e-45, "1E-45"),
        (0.1, "0.1"),
        (1.5, "1.5"),
        (1234567.9, "1234567.9"),
        (-0.0, "-0"),
        (f32::INFINITY, "Infinity"),
        (f32::NEG_INFINITY, "-Infinity"),
    ];
    for (value, expected) in cases {
        assert_eq!(InvariantF32(value).to_string(), expected);
    }
    assert_eq!(InvariantF32(NAN).to_string(), "NaN");
    assert_eq!(InvariantF32(0.0).to_string(), "0");
}

#[test]
fn scalar_helpers() {
    close(
        &[
            ieee_remainder(7.0, PI * 2.0),
            ieee_remainder(-10.0, 3.0),
            ieee_remainder(4.5, 3.0),
            ieee_remainder(1.5, 3.0),
        ],
        &[0.716814518, -1.0, -1.5, 1.5],
    );
    assert!(ieee_remainder(1.0, 0.0).is_nan());
    assert!(ieee_remainder(NAN, 1.0).is_nan());
    assert!(ieee_remainder(-6.0, 3.0).is_sign_negative());
}

#[test]
fn vector2_functions() {
    assert_eq!(A2.to_string(), "<1.5, -2.25>");
    close(&v2(Vector2::abs(A2)), &[1.5, 2.25]);
    close(&[A2.length(), A2.length_squared()], &[2.70416355, 7.3125]);
    close(
        &[Vector2::distance(A2, B2), Vector2::distance_squared(A2, B2)],
        &[3.71651721, 13.8125],
    );
    close(&v2(Vector2::normalize(A2)), &[0.554700196, -0.832050264]);
    close(&v2(Vector2::lerp(A2, B2, 0.3)), &[2.25, -1.42499995]);
    close(&v2(Vector2::clamp(Vector2::new(5.0, -5.0), A2, B2)), &[4.0, -2.25]);
    close(
        &v2(Vector2::clamp(
            Vector2::new(2.0, 2.0),
            Vector2::new(3.0, 3.0),
            Vector2::new(1.0, 1.0),
        )),
        &[1.0, 1.0],
    );
    close(&v2(Vector2::min(A2, B2)), &[1.5, -2.25]);
    close(&v2(Vector2::max(A2, B2)), &[4.0, 0.5]);
    close(
        &v2(Vector2::max(Vector2::new(NAN, 1.0), Vector2::new(1.0, NAN))),
        &[NAN, NAN],
    );
    close(
        &v2(Vector2::min(Vector2::new(NAN, 1.0), Vector2::new(1.0, NAN))),
        &[NAN, NAN],
    );
    close(&[Vector2::dot(A2, B2)], &[4.875]);
    close(&v2(A2 * B2), &[6.0, -1.125]);
    close(&v2(A2 / B2), &[0.375, -4.5]);
    close(&v2(A2 / 3.0), &[0.5, -0.75]);
    close(&v2(A2 * 3.0), &[4.5, -6.75]);
    close(&v2(3.0 * A2), &[4.5, -6.75]);
    close(&v2(A2 + B2), &[5.5, -1.75]);
    close(&v2(A2 - B2), &[-2.5, -2.75]);
    close(&v2(-A2), &[-1.5, 2.25]);
    close(&v2(Vector2::normalize(Vector2::ZERO)), &[NAN, NAN]);
    close(&v2(Vector2::transform(A2, m32())), &[15.1935062, 17.7362938]);
    close(&v2(Vector2::transform_quaternion(A2, q1())), &[2.27418852, -0.834826291]);
    assert_eq!(Vector2::default(), Vector2::ZERO);
}

#[test]
fn vector2_converts_to_and_from_the_double_precision_vector() {
    let v: Vector2 = crate::Vector::new(1.5, 0.1).into();
    assert_eq!(v, Vector2::new(1.5, 0.1));
    let d: crate::Vector = Vector2::new(1.5, 0.1).into();
    assert_eq!(d, crate::Vector::new(1.5, 0.1f32 as f64));
}

#[test]
fn vector3_functions() {
    assert_eq!(A3.to_string(), "<1.5, -2.25, 3>");
    close(&v3(Vector3::abs(A3)), &[1.5, 2.25, 3.0]);
    close(&[A3.length(), A3.length_squared()], &[4.03887367, 16.3125]);
    close(
        &[Vector3::distance(A3, B3), Vector3::distance_squared(A3, B3)],
        &[5.46008253, 29.8125],
    );
    close(&v3(Vector3::normalize(A3)), &[0.371390671, -0.557085991, 0.742781341]);
    close(&v3(Vector3::lerp(A3, B3, 0.3)), &[2.25, -1.42499995, 1.79999995]);
    close(
        &v3(Vector3::clamp(Vector3::new(5.0, -5.0, 0.0), A3, B3)),
        &[4.0, -2.25, -1.0],
    );
    close(&v3(Vector3::min(A3, B3)), &[1.5, -2.25, -1.0]);
    close(&v3(Vector3::max(A3, B3)), &[4.0, 0.5, 3.0]);
    close(&[Vector3::dot(A3, B3)], &[1.875]);
    close(&v3(Vector3::cross(A3, B3)), &[0.75, 13.5, 9.75]);
    close(&v3(A3 * B3), &[6.0, -1.125, -3.0]);
    close(&v3(A3 / B3), &[0.375, -4.5, -3.0]);
    close(&v3(A3 / 3.0), &[0.5, -0.75, 1.0]);
    close(&v3(Vector3::transform(A3, m44())), &[17.2389832, 16.9398727, 35.8941231]);
    close(
        &v3(Vector3::transform_quaternion(A3, q1())),
        &[3.61949158, -1.02004242, 1.47353077],
    );
    close(
        &v3(Vector3::transform_normal(A3, m44())),
        &[7.23898315, -3.06012726, 5.89412355],
    );
    assert_eq!(Vector3::from_vector2(A2, 7.0), Vector3::new(1.5, -2.25, 7.0));
    let d: crate::Vector3D = A3.into();
    assert_eq!(d, crate::Vector3D::new(1.5, -2.25, 3.0));
    let s: Vector3 = crate::Vector3D::new(1.5, -2.25, 0.1).into();
    assert_eq!(s, Vector3::new(1.5, -2.25, 0.1));
}

#[test]
fn vector4_functions() {
    assert_eq!(A4.to_string(), "<1.5, -2.25, 3, 0.5>");
    close(&v4(Vector4::abs(A4)), &[1.5, 2.25, 3.0, 0.5]);
    close(&[A4.length(), A4.length_squared()], &[4.06970501, 16.5625]);
    close(
        &[Vector4::distance(A4, B4), Vector4::distance_squared(A4, B4)],
        &[5.66237593, 32.0625],
    );
    close(
        &v4(Vector4::normalize(A4)),
        &[0.368577093, -0.552865624, 0.737154186, 0.122859031],
    );
    close(
        &v4(Vector4::lerp(A4, B4, 0.3)),
        &[2.25, -1.42499995, 1.79999995, 0.950000048],
    );
    close(
        &v4(Vector4::clamp(Vector4::new(5.0, -5.0, 0.0, 1.0), A4, B4)),
        &[4.0, -2.25, -1.0, 1.0],
    );
    close(&v4(Vector4::min(A4, B4)), &[1.5, -2.25, -1.0, 0.5]);
    close(&v4(Vector4::max(A4, B4)), &[4.0, 0.5, 3.0, 2.0]);
    close(&[Vector4::dot(A4, B4)], &[2.875]);
    close(&v4(A4 * B4), &[6.0, -1.125, -3.0, 1.0]);
    close(&v4(A4 / B4), &[0.375, -4.5, -3.0, 0.25]);
    close(&v4(A4 / 3.0), &[0.5, -0.75, 1.0, 0.166666672]);
    close(
        &v4(Vector4::transform(A4, m44())),
        &[12.2389832, 6.93987274, 20.8941231, 0.5],
    );
    close(
        &v4(Vector4::transform_vector3(A3, m44())),
        &[17.2389832, 16.9398727, 35.8941231, 1.0],
    );
    close(
        &v4(Vector4::transform_vector2(A2, m44())),
        &[14.548377, 17.4955215, 25.1939526, 1.0],
    );
    assert_eq!(Vector4::from_vector2(A2, 7.0, 8.0), Vector4::new(1.5, -2.25, 7.0, 8.0));
    assert_eq!(Vector4::from_vector3(A3, 8.0), Vector4::new(1.5, -2.25, 3.0, 8.0));
}

#[test]
fn quaternion_functions() {
    let (q1, q2) = (q1(), q2());
    close(&q(q1), &[0.104076423, 0.208152846, 0.312229276, 0.921060979]);
    close(&q(q2), &[-0.174488455, 0.274439752, 0.531384349, 0.782219529]);
    assert_eq!(Quaternion::new(1.0, 2.0, 3.0, 4.5).to_string(), "{X:1 Y:2 Z:3 W:4.5}");
    close(&q(q1 + q2), &[-0.0704120323, 0.482592583, 0.843613625, 1.70328045]);
    close(&q(q1 - q2), &[0.27856487, -0.0662869066, -0.219155073, 0.13884145]);
    close(&q(-q1), &[-0.104076423, -0.208152846, -0.312229276, -0.921060979]);
    close(&q(q1 * q2), &[-0.0543828532, 0.305811971, 0.798552215, 0.515592813]);
    close(&q(q1 * 2.5), &[0.260191053, 0.520382106, 0.780573189, 2.30265236]);
    close(&q(q1 / q2), &[0.217204064, 0.0198304318, -0.310088515, 0.925350785]);
    close(
        &q(Quaternion::concatenate(q1, q2)),
        &[-0.104224943, 0.525381982, 0.668786287, 0.515592813],
    );
    let v = Quaternion::new(1.0, 2.0, 3.0, 4.0);
    close(&[v.length(), v.length_squared()], &[5.47722578, 30.0]);
    close(
        &q(Quaternion::normalize(v)),
        &[0.182574183, 0.365148365, 0.547722518, 0.730296731],
    );
    let slerp = [0.0202354342, 0.231858417, 0.384542853, 0.893285394];
    let lerp = [0.0208361913, 0.231700048, 0.384044111, 0.893527269];
    close(&q(Quaternion::slerp(q1, q2, 0.3)), &slerp);
    close(&q(Quaternion::lerp(q1, q2, 0.3)), &lerp);
    close(&q(Quaternion::slerp(q1, -q2, 0.3)), &slerp);
    close(&q(Quaternion::lerp(q1, -q2, 0.3)), &lerp);
    close(
        &q(Quaternion::slerp(q1, q1, 0.3)),
        &[0.10407643, 0.20815286, 0.312229276, 0.921060979],
    );
    close(
        &q(Quaternion::conjugate(q1)),
        &[-0.104076423, -0.208152846, -0.312229276, 0.921060979],
    );
    close(
        &q(Quaternion::inverse(v)),
        &[-0.0333333351, -0.0666666701, -0.100000001, 0.13333334],
    );
    close(&[Quaternion::dot(q1, q2)], &[0.925350904]);
    close(
        &q(Quaternion::create_from_rotation_matrix(Matrix4x4::create_from_quaternion(q2))),
        &[-0.17448847, 0.274439752, 0.531384349, 0.78221947],
    );
    assert!(Quaternion::IDENTITY.is_identity());
    assert!(!Quaternion::default().is_identity());
    assert_eq!(Quaternion::default(), Quaternion::ZERO);
}

#[test]
fn quaternion_from_rotation_matrix_covers_every_branch() {
    // Half-turn rotations have a non-positive trace and exercise the three
    // branches that pick the largest diagonal element.
    for axis in [Vector3::UNIT_X, Vector3::UNIT_Y, Vector3::UNIT_Z] {
        let expected = Quaternion::create_from_axis_angle(axis, PI);
        let actual = Quaternion::create_from_rotation_matrix(Matrix4x4::create_from_quaternion(expected));
        let sign = if Quaternion::dot(expected, actual) < 0.0 { -1.0 } else { 1.0 };
        close(&q(actual * sign), &q(expected));
    }
}

#[test]
fn matrix3x2_functions() {
    let m = m32();
    close(&m3(m), &[1.52968442, 1.93265295, -1.28843534, 2.29452658, 10.0, 20.0]);
    assert_eq!(
        Matrix3x2::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.5).to_string(),
        "{ {M11:1 M12:2} {M21:3 M22:4} {M31:5 M32:6.5} }"
    );
    close(
        &m3(Matrix3x2::create_rotation(0.7)),
        &[0.764842212, 0.64421767, -0.64421767, 0.764842212, 0.0, 0.0],
    );
    assert_eq!(m3(Matrix3x2::create_rotation(PI / 2.0)), [0.0, 1.0, -1.0, 0.0, 0.0, 0.0]);
    assert_eq!(m3(Matrix3x2::create_rotation(PI)), [-1.0, 0.0, 0.0, -1.0, 0.0, 0.0]);
    assert_eq!(m3(Matrix3x2::create_rotation(-PI / 2.0)), [0.0, -1.0, 1.0, 0.0, 0.0, 0.0]);
    assert_eq!(m3(Matrix3x2::create_rotation(0.0)), m3(Matrix3x2::IDENTITY));
    close(
        &m3(Matrix3x2::create_rotation(7.0)),
        &[0.753902376, 0.656986475, -0.656986475, 0.753902376, 0.0, 0.0],
    );
    close(
        &m3(Matrix3x2::create_rotation_at(0.7, Vector2::new(3.0, 4.0))),
        &[0.764842212, 0.64421767, -0.64421767, 0.764842212, 3.2823441, -0.992021799],
    );
    close(
        &m3(Matrix3x2::create_skew(0.3, 0.4)),
        &[1.0, 0.422793239, 0.309336275, 1.0, 0.0, 0.0],
    );
    close(
        &m3(Matrix3x2::create_skew_at(0.3, 0.4, Vector2::new(3.0, 4.0))),
        &[1.0, 0.422793239, 0.309336275, 1.0, -1.2373451, -1.26837969],
    );
    close(
        &m3(Matrix3x2::create_scale_at(Vector2::new(2.0, 3.0), Vector2::new(3.0, 4.0))),
        &[2.0, 0.0, 0.0, 3.0, -3.0, -8.0],
    );
    assert_eq!(m3(Matrix3x2::create_scale_uniform(2.0)), [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
    assert_eq!(
        m3(Matrix3x2::create_translation_xy(2.0, 3.0)),
        [1.0, 0.0, 0.0, 1.0, 2.0, 3.0]
    );

    let inverse = Matrix3x2::invert(m).unwrap();
    close(
        &m3(inverse),
        &[0.382421106, -0.322108835, 0.214739233, 0.254947424, -8.11899567, -1.87785983],
    );
    let singular = Matrix3x2::new(1.0, 2.0, 2.0, 4.0, 0.0, 0.0);
    assert_eq!(Matrix3x2::invert(singular), None);
    close(&m3(Matrix3x2::invert_or_nan(singular)), &[NAN; 6]);
    close(&[m.get_determinant()], &[6.0]);
    close(
        &m3(m + inverse),
        &[1.91210556, 1.61054409, -1.07369614, 2.549474, 1.88100433, 18.1221409],
    );
    close(
        &m3(m - inverse),
        &[1.14726329, 2.2547617, -1.50317454, 2.03957915, 18.1189957, 21.8778591],
    );
    close(
        &m3(-m),
        &[-1.52968442, -1.93265295, 1.28843534, -2.29452658, -10.0, -20.0],
    );
    close(
        &m3(m * 2.5),
        &[3.82421112, 4.83163261, -3.22108841, 5.73631668, 25.0, 50.0],
    );
    close(
        &m3(Matrix3x2::lerp(m, inverse, 0.25)),
        &[1.24286854, 1.36896253, -0.912641704, 1.78463173, 5.47025108, 14.5305347],
    );
    close(&m3(m * inverse), &m3(Matrix3x2::IDENTITY));
    assert!(Matrix3x2::IDENTITY.is_identity());
    assert!(!Matrix3x2::default().is_identity());
    assert_eq!(m.translation(), Vector2::new(10.0, 20.0));
}

#[test]
fn matrix4x4_functions() {
    let m = m44();
    close(
        &m4(m),
        &[
            1.43674111, 1.85547602, -1.2738061, 0.0, -1.0636735, 2.35008597, 1.28681719, 0.0, 0.896868706,
            -0.18521592, 3.56672382, 0.0, 10.0, 20.0, 30.0, 1.0,
        ],
    );
    assert_eq!(
        Matrix4x4::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.5)
            .to_string(),
        "{ {M11:1 M12:2 M13:3 M14:4} {M21:5 M22:6 M23:7 M24:8} {M31:9 M32:10 M33:11 M34:12} {M41:13 M42:14 M43:15 M44:16.5} }"
    );
    let rotation = [
        0.718370557, 0.618492007, -0.318451524, 0.0, -0.531836748, 0.783361971, 0.321704298, 0.0, 0.448434353,
        -0.0617386401, 0.891680956, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    close(&m4(Matrix4x4::create_from_quaternion(q1())), &rotation);
    close(
        &m4(Matrix4x4::create_from_axis_angle(
            Vector3::normalize(Vector3::new(1.0, 2.0, 3.0)),
            0.8,
        )),
        &rotation,
    );
    let (c, s) = (0.764842212, 0.64421767);
    let center = Vector3::new(3.0, 4.0, 5.0);
    close(
        &m4(Matrix4x4::create_rotation_z(0.7)),
        &[c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    );
    close(
        &m4(Matrix4x4::create_rotation_z_at(0.7, center)),
        &[
            c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 3.2823441, -0.992021799, 0.0, 1.0,
        ],
    );
    close(
        &m4(Matrix4x4::create_rotation_x(0.7)),
        &[1.0, 0.0, 0.0, 0.0, 0.0, c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0],
    );
    close(
        &m4(Matrix4x4::create_rotation_x_at(0.7, center)),
        &[
            1.0, 0.0, 0.0, 0.0, 0.0, c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 4.16171932, -1.4010818, 1.0,
        ],
    );
    close(
        &m4(Matrix4x4::create_rotation_y(0.7)),
        &[c, 0.0, -s, 0.0, 0.0, 1.0, 0.0, 0.0, s, 0.0, c, 0.0, 0.0, 0.0, 0.0, 1.0],
    );
    close(
        &m4(Matrix4x4::create_rotation_y_at(0.7, center)),
        &[
            c, 0.0, -s, 0.0, 0.0, 1.0, 0.0, 0.0, s, 0.0, c, 0.0, -2.51561499, 0.0, 3.10844183, 1.0,
        ],
    );
    close(
        &m4(Matrix4x4::create_scale_at(Vector3::new(2.0, 3.0, 4.0), center)),
        &[
            2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 4.0, 0.0, -3.0, -8.0, -15.0, 1.0,
        ],
    );
    close(
        &m4(Matrix4x4::create_from_yaw_pitch_roll(0.3, -0.6, 1.1)),
        &[
            0.28462702, 0.735545278, -0.614785135, 0.0, -0.927091599, 0.374368906, 0.0186894238, 0.0, 0.243903399,
            0.564642549, 0.788473248, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    );
    assert_eq!(
        m4(Matrix4x4::from_matrix3x2(Matrix3x2::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0))),
        [1.0, 2.0, 0.0, 0.0, 3.0, 4.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 5.0, 6.0, 0.0, 1.0]
    );

    let inverse = Matrix4x4::invert(m).unwrap();
    close(
        &m4(inverse),
        &[
            0.359185308, -0.265918404, 0.224217176, 0.0, 0.206163988, 0.261120677, -0.0205795579, 0.0,
            -0.0796128809, 0.0804260746, 0.222920239, 0.0, -5.32674551, -4.97601175, -8.51818848, 1.0,
        ],
    );
    assert_eq!(Matrix4x4::invert(Matrix4x4::default()), None);
    close(&m4(Matrix4x4::invert_or_nan(Matrix4x4::default())), &[NAN; 16]);
    close(&[m.get_determinant()], &[24.0]);
    close(
        &m4(m + inverse),
        &[
            1.79592645, 1.58955765, -1.04958892, 0.0, -0.857509494, 2.61120653, 1.26623762, 0.0, 0.817255855,
            -0.104789846, 3.789644, 0.0, 4.67325449, 15.0239887, 21.4818115, 2.0,
        ],
    );
    close(
        &m4(m - inverse),
        &[
            1.07755578, 2.1213944, -1.49802327, 0.0, -1.2698375, 2.08896542, 1.30739677, 0.0, 0.976481557,
            -0.265641987, 3.34380364, 0.0, 15.326746, 24.9760113, 38.5181885, 0.0,
        ],
    );
    close(
        &m4(-m),
        &[
            -1.43674111, -1.85547602, 1.2738061, 0.0, 1.0636735, -2.35008597, -1.28681719, 0.0, -0.896868706,
            0.18521592, -3.56672382, 0.0, -10.0, -20.0, -30.0, -1.0,
        ],
    );
    close(
        &m4(m * 2.5),
        &[
            3.59185266, 4.63868999, -3.18451524, 0.0, -2.65918374, 5.87521505, 3.21704292, 0.0, 2.24217176,
            -0.463039815, 8.91680908, 0.0, 25.0, 50.0, 75.0, 2.5,
        ],
    );
    close(
        &m4(Matrix4x4::transpose(m)),
        &[
            1.43674111, -1.0636735, 0.896868706, 10.0, 1.85547602, 2.35008597, -0.18521592, 20.0, -1.2738061,
            1.28681719, 3.56672382, 30.0, 0.0, 0.0, 0.0, 1.0,
        ],
    );
    close(
        &m4(Matrix4x4::lerp(m, inverse, 0.25)),
        &[
            1.1673522, 1.32512736, -0.899300277, 0.0, -0.746214151, 1.82784462, 0.95996803, 0.0, 0.652748287,
            -0.118805423, 2.73077297, 0.0, 6.1683135, 13.7559967, 20.3704529, 1.0,
        ],
    );
    assert!(Matrix4x4::IDENTITY.is_identity());
    assert!(!Matrix4x4::default().is_identity());
    assert_eq!(m.translation(), Vector3::new(10.0, 20.0, 30.0));
}

#[test]
fn matrix4x4_inverts_a_general_matrix() {
    let m = Matrix4x4::new(
        2.0, 0.5, -1.0, 0.25, 0.0, 3.0, 1.5, -0.5, 1.0, -2.0, 4.0, 0.75, 0.5, 1.0, -1.5, 2.0,
    );
    let inverse = Matrix4x4::invert(m).unwrap();
    let tolerance = 1e-5;
    for (a, e) in m4(m * inverse).iter().zip(m4(Matrix4x4::IDENTITY)) {
        assert!((a - e).abs() < tolerance);
    }
    for (a, e) in m4(inverse * m).iter().zip(m4(Matrix4x4::IDENTITY)) {
        assert!((a - e).abs() < tolerance);
    }
    assert!((m.get_determinant() * inverse.get_determinant() - 1.0).abs() < tolerance);
}

// The expected values of the look-at and perspective tests follow from the
// documented formulas of the reference runtime, computed by hand.

#[test]
fn matrix4x4_look_at_down_the_negative_z_axis_is_a_translation() {
    // zaxis = (0, 0, 1), xaxis = (1, 0, 0), yaxis = (0, 1, 0): no rotation.
    let view = Matrix4x4::create_look_at(Vector3::new(0.0, 0.0, 1.0), Vector3::ZERO, Vector3::UNIT_Y);
    close(&m4(view), &m4(Matrix4x4::create_translation_xyz(0.0, 0.0, -1.0)));
}

#[test]
fn matrix4x4_look_at_builds_the_basis_of_the_camera() {
    // The camera is at (3, 0, 0) and looks at the origin:
    // zaxis = (1, 0, 0), xaxis = up x zaxis = (0, 0, -1), yaxis = zaxis x xaxis = (0, 1, 0).
    let position = Vector3::new(3.0, 0.0, 0.0);
    let view = Matrix4x4::create_look_at(position, Vector3::ZERO, Vector3::UNIT_Y);
    close(
        &m4(view),
        &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -3.0, 1.0],
    );
    // The camera is the origin of the view space, and the target lies on the
    // negative z axis at its distance from the camera.
    close(&v3(Vector3::transform(position, view)), &[0.0, 0.0, 0.0]);
    close(&v3(Vector3::transform(Vector3::ZERO, view)), &[0.0, 0.0, -3.0]);
}

#[test]
fn matrix4x4_look_at_from_a_diagonal() {
    // zaxis = (1, 1, 1) / sqrt(3), xaxis = (1, 0, -1) / sqrt(2), yaxis = (-1, 2, -1) / sqrt(6);
    // the translation is (0, 0, -|position|) with |position| = 25 * sqrt(3).
    let view = Matrix4x4::create_look_at(Vector3::new(25.0, 25.0, 25.0), Vector3::ZERO, Vector3::UNIT_Y);
    let (x, y, z) = (0.70710678, 0.40824829, 0.57735027);
    close(
        &m4(view),
        &[x, -y, z, 0.0, 0.0, 2.0 * y, z, 0.0, -x, -y, z, 0.0, 0.0, 0.0, -43.30127, 1.0],
    );
}

#[test]
fn matrix4x4_look_at_without_a_direction_is_not_a_number() {
    let view = Matrix4x4::create_look_at(Vector3::ONE, Vector3::ONE, Vector3::UNIT_Y);
    assert!(view.m11.is_nan() && view.m22.is_nan() && view.m33.is_nan());
    assert_eq!((view.m14, view.m24, view.m34, view.m44), (0.0, 0.0, 0.0, 1.0));
}

#[test]
fn matrix4x4_perspective_field_of_view() {
    // tan(pi / 4) = 1: height = 1, width = 1 / aspect = 0.5,
    // range = far / (near - far) = 100 / (1 - 100), m43 = near * range.
    let projection = Matrix4x4::create_perspective_field_of_view(PI / 2.0, 2.0, 1.0, 100.0);
    close(
        &m4(projection),
        &[0.5, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -100.0 / 99.0, -1.0, 0.0, 0.0, -100.0 / 99.0, 0.0],
    );

    // height = 1 / tan(pi / 8) = 1 + sqrt(2), width = height / 1.6,
    // range = 1000 / (0.5 - 1000), m43 = 0.5 * range.
    let projection = Matrix4x4::create_perspective_field_of_view(PI / 4.0, 1.6, 0.5, 1000.0);
    close(
        &m4(projection),
        &[
            1.50888348, 0.0, 0.0, 0.0, 0.0, 2.41421356, 0.0, 0.0, 0.0, 0.0, -1.00050025, -1.0, 0.0, 0.0,
            -0.500250125, 0.0,
        ],
    );
}

#[test]
fn matrix4x4_perspective_maps_the_planes_to_the_depth_range() {
    // A point on the near plane has the depth 0 and one on the far plane the depth 1.
    let projection = Matrix4x4::create_perspective_field_of_view(1.0, 1.5, 2.0, 50.0);
    for (z, depth) in [(-2.0, 0.0), (-50.0, 1.0)] {
        let clip_z = z * projection.m33 + projection.m43;
        let clip_w = z * projection.m34 + projection.m44;
        close(&[clip_z / clip_w], &[depth]);
    }
}

#[test]
fn matrix4x4_perspective_accepts_a_far_plane_at_infinity() {
    let projection = Matrix4x4::create_perspective_field_of_view(PI / 2.0, 1.0, 0.25, f32::INFINITY);
    close(
        &m4(projection),
        &[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, -1.0, 0.0, 0.0, -0.25, 0.0],
    );
}

#[test]
#[should_panic(expected = "field_of_view ('0') must be greater than '0'.")]
fn matrix4x4_perspective_rejects_a_field_of_view_of_zero() {
    Matrix4x4::create_perspective_field_of_view(0.0, 1.0, 1.0, 10.0);
}

#[test]
#[should_panic(expected = "field_of_view")]
fn matrix4x4_perspective_rejects_a_field_of_view_of_a_half_turn() {
    Matrix4x4::create_perspective_field_of_view(PI, 1.0, 1.0, 10.0);
}

#[test]
#[should_panic(expected = "near_plane_distance ('0') must be greater than '0'.")]
fn matrix4x4_perspective_rejects_a_near_plane_at_the_camera() {
    Matrix4x4::create_perspective_field_of_view(1.0, 1.0, 0.0, 10.0);
}

#[test]
#[should_panic(expected = "far_plane_distance ('-1') must be greater than '0'.")]
fn matrix4x4_perspective_rejects_a_far_plane_behind_the_camera() {
    Matrix4x4::create_perspective_field_of_view(1.0, 1.0, 1.0, -1.0);
}

#[test]
#[should_panic(expected = "near_plane_distance ('10') must be less than '10'.")]
fn matrix4x4_perspective_rejects_a_near_plane_at_the_far_plane() {
    Matrix4x4::create_perspective_field_of_view(1.0, 1.0, 10.0, 10.0);
}

#[test]
fn matrix4x4_and_vector3_have_the_layout_of_the_reference_runtime() {
    assert_eq!(std::mem::size_of::<Matrix4x4>(), 16 * std::mem::size_of::<f32>());
    assert_eq!(std::mem::size_of::<Vector3>(), 3 * std::mem::size_of::<f32>());
    assert_eq!(std::mem::align_of::<Matrix4x4>(), std::mem::align_of::<f32>());
    assert_eq!(std::mem::align_of::<Vector3>(), std::mem::align_of::<f32>());

    assert_eq!(std::mem::offset_of!(Matrix4x4, m11), 0);
    assert_eq!(std::mem::offset_of!(Matrix4x4, m12), 4);
    assert_eq!(std::mem::offset_of!(Matrix4x4, m21), 16);
    assert_eq!(std::mem::offset_of!(Matrix4x4, m31), 32);
    assert_eq!(std::mem::offset_of!(Matrix4x4, m41), 48);
    assert_eq!(std::mem::offset_of!(Matrix4x4, m44), 60);

    assert_eq!(std::mem::offset_of!(Vector3, x), 0);
    assert_eq!(std::mem::offset_of!(Vector3, y), 4);
    assert_eq!(std::mem::offset_of!(Vector3, z), 8);
}
