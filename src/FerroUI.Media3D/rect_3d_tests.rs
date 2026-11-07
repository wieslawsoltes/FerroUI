//! Tests of `Rect3D`: union and intersection (with the empty box), containment, the guarded
//! members and the text form.

use super::*;

fn rect(x: f64, y: f64, z: f64, size_x: f64, size_y: f64, size_z: f64) -> Rect3D {
    Rect3D::new(x, y, z, size_x, size_y, size_z).unwrap()
}

#[test]
fn empty_is_the_box_of_nothing() {
    let empty = Rect3D::EMPTY;
    assert!(empty.is_empty());
    assert_eq!(empty.location(), Point3D::new(f64::INFINITY, f64::INFINITY, f64::INFINITY));
    assert_eq!(empty.size(), Size3D::EMPTY);
    assert!(!Rect3D::default().is_empty());
    assert!(!empty.contains(Point3D::default()));
    assert!(!empty.contains_rect(empty));
    assert!(!empty.intersects_with(rect(-1e9, -1e9, -1e9, 2e9, 2e9, 2e9)));

    // A box made from the empty size is the empty box.
    assert_eq!(Rect3D::from_location_size(Point3D::new(1.0, 2.0, 3.0), Size3D::EMPTY), Rect3D::EMPTY);
}

#[test]
fn union_contains_both_and_empty_adds_nothing() {
    let a = rect(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
    let b = rect(2.0, -1.0, 0.5, 1.0, 1.0, 3.0);
    let union = Rect3D::union_rects(a, b);
    assert_eq!(union, rect(0.0, -1.0, 0.0, 3.0, 2.0, 3.5));
    assert!(union.contains_rect(a) && union.contains_rect(b));

    assert_eq!(Rect3D::union_rects(a, Rect3D::EMPTY), a);
    assert_eq!(Rect3D::union_rects(Rect3D::EMPTY, a), a);
    assert_eq!(Rect3D::union_rects(Rect3D::EMPTY, Rect3D::EMPTY), Rect3D::EMPTY);

    // The union with a point grows the box to the point; the empty box becomes the point.
    assert_eq!(Rect3D::union_rect_point(a, Point3D::new(-1.0, 2.0, 0.5)), rect(-1.0, 0.0, 0.0, 2.0, 2.0, 1.0));
    let mut from_points = Rect3D::EMPTY;
    from_points.union_point(Point3D::new(1.0, 2.0, 3.0));
    assert_eq!(from_points, rect(1.0, 2.0, 3.0, 0.0, 0.0, 0.0));
    from_points.union_point(Point3D::new(-1.0, 4.0, 3.0));
    assert_eq!(from_points, rect(-1.0, 2.0, 3.0, 2.0, 2.0, 0.0));
}

#[test]
fn intersect_is_the_common_box_or_empty() {
    let a = rect(0.0, 0.0, 0.0, 2.0, 2.0, 2.0);
    let b = rect(1.0, 1.0, -1.0, 2.0, 2.0, 2.0);
    assert!(a.intersects_with(b));
    assert_eq!(Rect3D::intersect_rects(a, b), rect(1.0, 1.0, 0.0, 1.0, 1.0, 1.0));

    // Touching faces intersect in a flat box.
    let touching = rect(2.0, 0.0, 0.0, 1.0, 1.0, 1.0);
    assert!(a.intersects_with(touching));
    assert_eq!(Rect3D::intersect_rects(a, touching), rect(2.0, 0.0, 0.0, 0.0, 1.0, 1.0));

    // Apart, or with the empty box: the empty box.
    let apart = rect(5.0, 0.0, 0.0, 1.0, 1.0, 1.0);
    assert!(!a.intersects_with(apart));
    assert_eq!(Rect3D::intersect_rects(a, apart), Rect3D::EMPTY);
    assert_eq!(Rect3D::intersect_rects(a, Rect3D::EMPTY), Rect3D::EMPTY);
    assert_eq!(Rect3D::intersect_rects(Rect3D::EMPTY, a), Rect3D::EMPTY);

    let mut c = a;
    c.intersect(b);
    assert_eq!(c, rect(1.0, 1.0, 0.0, 1.0, 1.0, 1.0));
}

#[test]
fn contains_includes_the_faces() {
    let a = rect(0.0, 0.0, 0.0, 2.0, 2.0, 2.0);
    assert!(a.contains(Point3D::new(1.0, 1.0, 1.0)));
    assert!(a.contains(Point3D::new(0.0, 2.0, 2.0)));
    assert!(!a.contains(Point3D::new(2.0, 2.0, 2.000001)));
    assert!(a.contains_xyz(2.0, 0.0, 1.0));
    assert!(a.contains_rect(rect(0.0, 0.0, 0.0, 2.0, 2.0, 2.0)));
    assert!(a.contains_rect(rect(0.5, 0.5, 0.5, 1.0, 1.0, 1.0)));
    assert!(!a.contains_rect(rect(1.5, 0.5, 0.5, 1.0, 1.0, 1.0)));
    assert!(!a.contains_rect(Rect3D::EMPTY));
}

#[test]
fn members_are_guarded_as_in_wpf() {
    assert_eq!(Rect3D::new(0.0, 0.0, 0.0, -1.0, 1.0, 1.0), Err(Size3DError::NegativeDimension));

    let mut a = rect(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
    assert_eq!(a.set_size_z(-1.0), Err(Size3DError::NegativeDimension));
    a.set_x(-1.0).unwrap();
    a.set_size_x(0.0).unwrap();
    assert_eq!(a, rect(-1.0, 2.0, 3.0, 0.0, 5.0, 6.0));
    a.set_location(Point3D::new(7.0, 8.0, 9.0)).unwrap();
    a.set_size(Size3D::new(1.0, 1.0, 1.0).unwrap()).unwrap();
    assert_eq!(a, rect(7.0, 8.0, 9.0, 1.0, 1.0, 1.0));
    a.offset(Vector3D::new(1.0, -1.0, 0.5)).unwrap();
    assert_eq!(a, rect(8.0, 7.0, 9.5, 1.0, 1.0, 1.0));
    assert_eq!(Rect3D::offset_rect(a, Vector3D::new(-8.0, -7.0, -9.5)).unwrap(), rect(0.0, 0.0, 0.0, 1.0, 1.0, 1.0));

    // The empty size makes the box empty; the empty box is not moved or resized.
    a.set_size(Size3D::EMPTY).unwrap();
    assert_eq!(a, Rect3D::EMPTY);
    assert_eq!(a.set_x(0.0), Err(Size3DError::ModifiesEmpty));
    assert_eq!(a.set_size_y(1.0), Err(Size3DError::ModifiesEmpty));
    assert_eq!(a.set_location(Point3D::default()), Err(Size3DError::ModifiesEmpty));
    assert_eq!(a.set_size(Size3D::default()), Err(Size3DError::ModifiesEmpty));
    assert_eq!(a.offset_xyz(1.0, 1.0, 1.0), Err(Size3DError::ModifiesEmpty));
    assert_eq!(a, Rect3D::EMPTY);
}

#[test]
fn parse_and_format_round_trip() {
    let a = rect(1.0, -2.0, 0.5, 3.0, 4.0, 5.25);
    assert_eq!(a.to_string(), "1,-2,0.5,3,4,5.25");
    assert_eq!(Rect3D::parse("1,-2,0.5,3,4,5.25").unwrap(), a);
    assert_eq!(Rect3D::parse("1 -2 0.5 3 4 5.25").unwrap(), a);
    assert_eq!(a.to_string().parse::<Rect3D>().unwrap(), a);

    assert_eq!(Rect3D::EMPTY.to_string(), "Empty");
    assert_eq!(Rect3D::parse("Empty").unwrap(), Rect3D::EMPTY);
    assert_eq!(Rect3D::parse(&Rect3D::EMPTY.to_string()).unwrap(), Rect3D::EMPTY);

    assert_eq!(Rect3D::parse("0,0,0,1,-1,1").unwrap_err().message(), "Dimension cannot be a negative value.");
    for text in ["", "1,2,3,4,5", "1,2,3,4,5,6,7", "Empty 1", "EMPTY"] {
        assert_eq!(Rect3D::parse(text).unwrap_err().message(), "Invalid Rect3D.", "{text:?}");
    }
}
