//! Tests of the collections: WPF's text forms and the change notification.

use super::*;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn positions_parse_three_numbers_per_point_and_write_them_back() {
    let positions = Point3DCollection::parse("0,0,0 1,0,0 0,1,0").unwrap();
    assert_eq!(
        positions.to_vec(),
        [Point3D::new(0.0, 0.0, 0.0), Point3D::new(1.0, 0.0, 0.0), Point3D::new(0.0, 1.0, 0.0)]
    );
    assert_eq!(positions.to_string(), "0,0,0 1,0,0 0,1,0");
    // Commas and white space are both separators, as WPF's tokenizer reads them.
    let mixed = Point3DCollection::parse("  -1, 2.5 ,3e1   4 5 6 ").unwrap();
    assert_eq!(mixed.to_vec(), [Point3D::new(-1.0, 2.5, 30.0), Point3D::new(4.0, 5.0, 6.0)]);
    assert!(Point3DCollection::parse("").unwrap().is_empty());
}

#[test]
fn malformed_positions_are_errors() {
    for text in ["0,0", "0,0,0 1", "0,,0,0", "0,0,0,", "a,b,c"] {
        let error = Point3DCollection::parse(text).unwrap_err();
        assert_eq!(error.message(), "Invalid Point3DCollection.", "{text}");
    }
}

#[test]
fn normals_parse_as_vectors() {
    let normals = Vector3DCollection::parse("0,0,1 0 0 1").unwrap();
    assert_eq!(normals.to_vec(), [Vector3D::new(0.0, 0.0, 1.0), Vector3D::new(0.0, 0.0, 1.0)]);
    assert_eq!(normals.to_string(), "0,0,1 0,0,1");
    assert!(Vector3DCollection::parse("1 2").is_err());
}

#[test]
fn triangle_indices_parse_integers_separated_by_spaces_or_commas() {
    assert_eq!(Int32Collection::parse("0 1 2").unwrap().to_vec(), [0, 1, 2]);
    assert_eq!(Int32Collection::parse("0,1,2  2,3,0").unwrap().to_vec(), [0, 1, 2, 2, 3, 0]);
    assert_eq!(Int32Collection::parse("0 1 2").unwrap().to_string(), "0 1 2");
    assert!(Int32Collection::parse("").unwrap().is_empty());
    assert!(Int32Collection::parse("0 1.5 2").is_err());
    assert!(Int32Collection::parse("0 x").is_err());
}

#[test]
fn collections_are_shared_handles_that_notify() {
    let positions = Point3DCollection::new();
    let shared = positions.clone();
    assert!(positions == shared, "a clone is the same collection");
    assert!(positions != Point3DCollection::new(), "equality is identity");
    let changes = Rc::new(Cell::new(0));
    let counted = changes.clone();
    let _subscription = positions.collection_changed(move |_| counted.set(counted.get() + 1));
    shared.add(Point3D::new(1.0, 2.0, 3.0));
    shared.clear();
    assert_eq!(changes.get(), 2);
    assert!(positions.is_empty());
}
