//! Tests of `Matrix3D`: determinant and inversion against hand values, the order of append and
//! prepend, rotations about a center, points against vectors, and the text form.

use super::*;

const EPSILON: f64 = 1e-12;

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < EPSILON, "{actual} is not {expected}");
}

fn assert_point_close(actual: Point3D, expected: Point3D) {
    assert!(
        (actual.x - expected.x).abs() < EPSILON
            && (actual.y - expected.y).abs() < EPSILON
            && (actual.z - expected.z).abs() < EPSILON,
        "{actual:?} is not {expected:?}"
    );
}

fn assert_matrix_close(actual: Matrix3D, expected: Matrix3D) {
    let (a, e) = (actual.rows(), expected.rows());
    for row in 0..4 {
        for column in 0..4 {
            assert!(
                (a[row][column] - e[row][column]).abs() < EPSILON,
                "element ({}, {}): {actual:?} is not {expected:?}",
                row + 1,
                column + 1
            );
        }
    }
}

fn from_rows(rows: [[f64; 4]; 4]) -> Matrix3D {
    Matrix3D::from_rows(rows)
}

fn rotation_about_z(angle: f64) -> Quaternion {
    Quaternion::from_axis_angle(Vector3D::new(0.0, 0.0, 1.0), angle).unwrap()
}

#[test]
fn the_default_is_the_identity() {
    assert_eq!(Matrix3D::default(), Matrix3D::IDENTITY);
    assert!(Matrix3D::IDENTITY.is_identity());
    assert!(Matrix3D::IDENTITY.is_affine());
    assert_eq!(Matrix3D::IDENTITY.determinant(), 1.0);

    let mut matrix = from_rows([[2.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);
    assert!(!matrix.is_identity());
    matrix.set_identity();
    assert!(matrix.is_identity());
}

#[test]
fn affine_means_no_projective_terms() {
    let mut matrix = Matrix3D::IDENTITY;
    matrix.translate(Vector3D::new(1.0, 2.0, 3.0));
    matrix.scale(Vector3D::new(2.0, 3.0, 4.0));
    assert!(matrix.is_affine());
    matrix.m24 = 0.5;
    assert!(!matrix.is_affine());
    let mut matrix = Matrix3D::IDENTITY;
    matrix.m44 = 2.0;
    assert!(!matrix.is_affine());
}

#[test]
fn determinant_against_hand_values() {
    // A general matrix (projective terms in the fourth column).
    let general = from_rows([[1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0], [2.0, 6.0, 4.0, 8.0], [3.0, 1.0, 1.0, 2.0]]);
    assert!(!general.is_affine());
    assert_eq!(general.determinant(), 72.0);
    // An affine matrix: the determinant of its 3x3 block (scale 2, 4, 8 and a translation).
    let affine = from_rows([[2.0, 0.0, 0.0, 0.0], [0.0, 4.0, 0.0, 0.0], [0.0, 0.0, 8.0, 0.0], [1.0, 2.0, 3.0, 1.0]]);
    assert_eq!(affine.determinant(), 64.0);
    // A rotation keeps volumes.
    let mut rotation = Matrix3D::IDENTITY;
    rotation.rotate(Quaternion::from_axis_angle(Vector3D::new(1.0, 2.0, 3.0), 33.0).unwrap());
    assert_close(rotation.determinant(), 1.0);
}

#[test]
fn invert_against_hand_values() {
    // Projective: a 2x2 block in rows and columns 1 and 4.
    let mut projective = from_rows([[2.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);
    assert!(projective.has_inverse());
    projective.invert().unwrap();
    assert_eq!(
        projective,
        from_rows([[0.5, 0.0, 0.0, -0.5], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]])
    );

    // Two shears, one of them projective (m34).
    let mut shears = from_rows([[1.0, 2.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 3.0], [0.0, 0.0, 0.0, 1.0]]);
    shears.invert().unwrap();
    assert_eq!(
        shears,
        from_rows([[1.0, -2.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, -3.0], [0.0, 0.0, 0.0, 1.0]])
    );

    // Affine: the inverse scale and the translation undone.
    let mut affine = from_rows([[2.0, 0.0, 0.0, 0.0], [0.0, 4.0, 0.0, 0.0], [0.0, 0.0, 8.0, 0.0], [1.0, 2.0, 3.0, 1.0]]);
    affine.invert().unwrap();
    assert_eq!(
        affine,
        from_rows([[0.5, 0.0, 0.0, 0.0], [0.0, 0.25, 0.0, 0.0], [0.0, 0.0, 0.125, 0.0], [-0.5, -0.5, -0.375, 1.0]])
    );

    // A general matrix: its inverse times itself is the identity.
    let general = from_rows([[1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0], [2.0, 6.0, 4.0, 8.0], [3.0, 1.0, 1.0, 2.0]]);
    let mut inverse = general;
    inverse.invert().unwrap();
    assert_close(inverse.m11, -1.0 / 6.0);
    assert_close(inverse.m24, -1.0 / 3.0);
    assert_close(inverse.offset_x, 7.0 / 12.0);
    assert_matrix_close(general * inverse, Matrix3D::IDENTITY);
    assert_matrix_close(inverse * general, Matrix3D::IDENTITY);

    // The identity inverts to itself.
    let mut identity = Matrix3D::IDENTITY;
    identity.invert().unwrap();
    assert!(identity.is_identity());
}

#[test]
fn a_singular_matrix_is_not_inverted() {
    // Affine, with two dependent rows.
    let singular = from_rows([[1.0, 2.0, 3.0, 0.0], [2.0, 4.0, 6.0, 0.0], [0.0, 0.0, 1.0, 0.0], [5.0, 5.0, 5.0, 1.0]]);
    let mut matrix = singular;
    assert!(!matrix.has_inverse());
    assert_eq!(matrix.invert(), Err(Matrix3DError::NotInvertible));
    assert_eq!(matrix, singular, "the matrix is unchanged");

    // Projective and all zero.
    let mut zero = from_rows([[0.0; 4]; 4]);
    assert!(!zero.is_affine());
    assert_eq!(zero.invert(), Err(Matrix3DError::NotInvertible));

    // WPF's tolerance: a determinant within ten machine epsilons of zero has no inverse.
    let mut tiny = Matrix3D::IDENTITY;
    tiny.scale(Vector3D::new(1e-6, 1e-6, 1e-6));
    assert!(!tiny.has_inverse());
    assert_eq!(tiny.invert(), Err(Matrix3DError::NotInvertible));
}

#[test]
fn append_applies_after_and_prepend_before() {
    let point = Point3D::new(1.0, 1.0, 1.0);

    // Translate, then scale.
    let mut appended = Matrix3D::IDENTITY;
    appended.translate(Vector3D::new(1.0, 0.0, 0.0));
    appended.scale(Vector3D::new(2.0, 2.0, 2.0));
    assert_eq!(appended.transform(point), Point3D::new(4.0, 2.0, 2.0));

    // Scale (prepended), then translate.
    let mut prepended = Matrix3D::IDENTITY;
    prepended.translate(Vector3D::new(1.0, 0.0, 0.0));
    prepended.scale_prepend(Vector3D::new(2.0, 2.0, 2.0));
    assert_eq!(prepended.transform(point), Point3D::new(3.0, 2.0, 2.0));

    // The same with whole matrices and the product: `a * b` is `a`, then `b`.
    let mut translation = Matrix3D::IDENTITY;
    translation.translate(Vector3D::new(1.0, 0.0, 0.0));
    let mut scale = Matrix3D::IDENTITY;
    scale.scale(Vector3D::new(2.0, 2.0, 2.0));
    let mut a = translation;
    a.append(scale);
    assert_eq!(a, translation * scale);
    assert_eq!(a, appended);
    let mut b = translation;
    b.prepend(scale);
    assert_eq!(b, scale * translation);
    assert_eq!(b, prepended);
    let mut c = translation;
    c *= scale;
    assert_eq!(c, a);

    // A translation prepended is applied first, so it is scaled.
    let mut translate_first = scale;
    translate_first.translate_prepend(Vector3D::new(1.0, 0.0, 0.0));
    assert_eq!(translate_first.transform(Point3D::default()), Point3D::new(2.0, 0.0, 0.0));
}

#[test]
fn rotations_about_the_origin_and_about_a_center() {
    // 90 degrees about Z turns X into Y (right-handed, counterclockwise seen from +Z).
    let mut rotation = Matrix3D::IDENTITY;
    rotation.rotate(rotation_about_z(90.0));
    assert_point_close(rotation.transform(Point3D::new(1.0, 0.0, 0.0)), Point3D::new(0.0, 1.0, 0.0));

    // About the center (1, 0, 0): that point stays, (2, 0, 0) goes to (1, 1, 0).
    let center = Point3D::new(1.0, 0.0, 0.0);
    let mut about_center = Matrix3D::IDENTITY;
    about_center.rotate_at(rotation_about_z(90.0), center);
    assert_point_close(about_center.transform(center), center);
    assert_point_close(about_center.transform(Point3D::new(2.0, 0.0, 0.0)), Point3D::new(1.0, 1.0, 0.0));

    // Rotating at a center is translating the center to the origin, rotating and translating back.
    let mut composed = Matrix3D::IDENTITY;
    composed.translate(Vector3D::new(-1.0, 0.0, 0.0));
    composed.rotate(rotation_about_z(90.0));
    composed.translate(Vector3D::new(1.0, 0.0, 0.0));
    assert_matrix_close(about_center, composed);

    // Appended after a translation the rotation moves the translated point; prepended it rotates
    // first.
    let mut after = Matrix3D::IDENTITY;
    after.translate(Vector3D::new(1.0, 0.0, 0.0));
    after.rotate_at(rotation_about_z(90.0), Point3D::default());
    assert_point_close(after.transform(Point3D::default()), Point3D::new(0.0, 1.0, 0.0));
    let mut before = Matrix3D::IDENTITY;
    before.translate(Vector3D::new(1.0, 0.0, 0.0));
    before.rotate_prepend(rotation_about_z(90.0));
    assert_point_close(before.transform(Point3D::new(1.0, 0.0, 0.0)), Point3D::new(1.0, 1.0, 0.0));
    let mut before_at = Matrix3D::IDENTITY;
    before_at.translate(Vector3D::new(1.0, 0.0, 0.0));
    before_at.rotate_at_prepend(rotation_about_z(90.0), center);
    assert_point_close(before_at.transform(Point3D::new(2.0, 0.0, 0.0)), Point3D::new(2.0, 1.0, 0.0));

    // Two rotations appended are the rotation of the product of the quaternions, the second on
    // the left.
    let first = Quaternion::from_axis_angle(Vector3D::new(1.0, 0.0, 0.0), 30.0).unwrap();
    let second = Quaternion::from_axis_angle(Vector3D::new(0.0, 1.0, 1.0), 50.0).unwrap();
    let mut twice = Matrix3D::IDENTITY;
    twice.rotate(first);
    twice.rotate(second);
    let mut product = Matrix3D::IDENTITY;
    product.rotate(second * first);
    assert_matrix_close(twice, product);
}

#[test]
fn scale_about_a_center() {
    let center = Point3D::new(1.0, 1.0, 1.0);
    let mut matrix = Matrix3D::IDENTITY;
    matrix.scale_at(Vector3D::new(2.0, 3.0, 4.0), center);
    assert_eq!(matrix.transform(center), center);
    assert_eq!(matrix.transform(Point3D::new(2.0, 2.0, 2.0)), Point3D::new(3.0, 4.0, 5.0));

    let mut prepended = Matrix3D::IDENTITY;
    prepended.translate(Vector3D::new(10.0, 0.0, 0.0));
    prepended.scale_at_prepend(Vector3D::new(2.0, 2.0, 2.0), center);
    assert_eq!(prepended.transform(Point3D::new(2.0, 2.0, 2.0)), Point3D::new(13.0, 3.0, 3.0));
}

#[test]
fn points_take_the_translation_and_vectors_do_not() {
    let mut matrix = Matrix3D::IDENTITY;
    matrix.scale(Vector3D::new(2.0, 2.0, 2.0));
    matrix.translate(Vector3D::new(5.0, 6.0, 7.0));

    assert_eq!(matrix.transform(Point3D::new(1.0, 0.0, 0.0)), Point3D::new(7.0, 6.0, 7.0));
    assert_eq!(matrix.transform_vector(Vector3D::new(1.0, 0.0, 0.0)), Vector3D::new(2.0, 0.0, 0.0));
    assert_eq!(Point3D::new(1.0, 0.0, 0.0) * matrix, Point3D::new(7.0, 6.0, 7.0));
    assert_eq!(Vector3D::new(1.0, 0.0, 0.0) * matrix, Vector3D::new(2.0, 0.0, 0.0));

    let mut points = [Point3D::new(0.0, 0.0, 0.0), Point3D::new(1.0, 1.0, 1.0)];
    matrix.transform_points(&mut points);
    assert_eq!(points, [Point3D::new(5.0, 6.0, 7.0), Point3D::new(7.0, 8.0, 9.0)]);
    let mut vectors = [Vector3D::new(0.0, 1.0, 0.0), Vector3D::new(1.0, 1.0, 1.0)];
    matrix.transform_vectors(&mut vectors);
    assert_eq!(vectors, [Vector3D::new(0.0, 2.0, 0.0), Vector3D::new(2.0, 2.0, 2.0)]);

    // A projective matrix divides a point by its w, and leaves a vector to its 3x3 block.
    let mut perspective = Matrix3D::IDENTITY;
    perspective.m34 = -0.5;
    assert_eq!(perspective.transform(Point3D::new(2.0, 2.0, -2.0)), Point3D::new(1.0, 1.0, -1.0));
    assert_eq!(perspective.transform_vector(Vector3D::new(2.0, 2.0, -2.0)), Vector3D::new(2.0, 2.0, -2.0));
}

#[test]
fn parse_and_format_round_trip() {
    assert_eq!(Matrix3D::parse("Identity").unwrap(), Matrix3D::IDENTITY);
    assert_eq!(Matrix3D::parse(" Identity ").unwrap(), Matrix3D::IDENTITY);
    assert_eq!(Matrix3D::IDENTITY.to_string(), "Identity");
    assert_eq!(Matrix3D::parse("1,0,0,0 0,1,0,0 0,0,1,0 0,0,0,1").unwrap(), Matrix3D::IDENTITY);
    // The identity written out is written as "Identity".
    assert_eq!(Matrix3D::parse("1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1").unwrap().to_string(), "Identity");

    let mut matrix = Matrix3D::IDENTITY;
    matrix.scale(Vector3D::new(2.0, 2.0, 2.0));
    matrix.translate(Vector3D::new(5.0, 6.0, 7.0));
    matrix.m34 = -0.25;
    assert_eq!(matrix.to_string(), "2,0,0,0,0,2,0,0,0,0,2,-0.25,5,6,7,1");
    assert_eq!(matrix.to_string().parse::<Matrix3D>().unwrap(), matrix);

    let mut rotation = Matrix3D::IDENTITY;
    rotation.rotate(Quaternion::from_axis_angle(Vector3D::new(1.0, 2.0, 3.0), 40.0).unwrap());
    assert_eq!(Matrix3D::parse(&rotation.to_string()).unwrap(), rotation);

    for text in [
        "",
        "identity",
        "Identity 1",
        "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15",
        "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17",
    ] {
        assert_eq!(Matrix3D::parse(text).unwrap_err().message(), "Invalid Matrix3D.", "{text:?}");
    }
}
