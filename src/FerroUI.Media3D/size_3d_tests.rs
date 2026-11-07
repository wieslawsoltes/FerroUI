//! Tests of `Size3D`: the empty size, the guarded dimensions, the conversions and the text form.

use super::*;

#[test]
fn empty_has_negative_infinite_dimensions() {
    let empty = Size3D::EMPTY;
    assert!(empty.is_empty());
    assert_eq!((empty.x(), empty.y(), empty.z()), (f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY));
    assert!(!Size3D::default().is_empty());
    assert!(!Size3D::new(0.0, 0.0, 0.0).unwrap().is_empty());
}

#[test]
fn dimensions_are_never_negative() {
    assert_eq!(Size3D::new(-1.0, 0.0, 0.0), Err(Size3DError::NegativeDimension));
    assert_eq!(Size3D::new(0.0, 0.0, -0.5), Err(Size3DError::NegativeDimension));

    let mut size = Size3D::new(1.0, 2.0, 3.0).unwrap();
    assert_eq!(size.set_y(-1.0), Err(Size3DError::NegativeDimension));
    assert_eq!(size.y(), 2.0);
    size.set_x(4.0).unwrap();
    size.set_y(5.0).unwrap();
    size.set_z(6.0).unwrap();
    assert_eq!((size.x(), size.y(), size.z()), (4.0, 5.0, 6.0));

    // The empty size is not changed, even to a valid dimension.
    let mut empty = Size3D::EMPTY;
    assert_eq!(empty.set_x(1.0), Err(Size3DError::ModifiesEmpty));
    assert!(empty.is_empty());
}

#[test]
fn conversions() {
    let size = Size3D::new(1.0, 2.0, 3.0).unwrap();
    assert_eq!(Vector3D::from(size), Vector3D::new(1.0, 2.0, 3.0));
    assert_eq!(Point3D::from(size), Point3D::new(1.0, 2.0, 3.0));
}

#[test]
fn parse_and_format_round_trip() {
    assert_eq!(Size3D::parse("1,2,3").unwrap(), Size3D::new(1.0, 2.0, 3.0).unwrap());
    assert_eq!(Size3D::parse(" 1 2 3 ").unwrap(), Size3D::new(1.0, 2.0, 3.0).unwrap());
    assert_eq!(Size3D::parse("Empty").unwrap(), Size3D::EMPTY);
    assert_eq!(Size3D::parse("  Empty ").unwrap(), Size3D::EMPTY);

    assert_eq!(Size3D::EMPTY.to_string(), "Empty");
    let size = Size3D::new(0.5, 10.0, 1e16).unwrap();
    assert_eq!(size.to_string(), "0.5,10,1E+16");
    assert_eq!(size.to_string().parse::<Size3D>().unwrap(), size);

    assert_eq!(Size3D::parse("1,-2,3").unwrap_err().message(), "Dimension cannot be a negative value.");
    for text in ["", "1,2", "empty", "Empty,1", "1,2,3,4"] {
        assert_eq!(Size3D::parse(text).unwrap_err().message(), "Invalid Size3D.", "{text:?}");
    }
}
