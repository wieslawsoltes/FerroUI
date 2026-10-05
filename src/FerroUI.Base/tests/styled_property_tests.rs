//! Port of the upstream styled property tests.

use super::*;
use crate::ferro_property_metadata::PropertyMetadata;
use crate::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

test_class!(Class1: FerroObject);
test_class!(Class2: FerroObject);

fn create_property() -> &'static StyledProperty<String> {
    StyledProperty::create(
        "p1",
        Class1::TYPE,
        Class1::TYPE,
        StyledPropertyMetadata::new(Some(String::new())),
        false,
        None,
        None,
        false,
    )
}

fn hash_of(property: &FerroProperty) -> u64 {
    let mut hasher = DefaultHasher::new();
    property.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn add_ownered_property_should_equal_original() {
    let p1 = create_property();
    let p2 = p1.add_owner::<Class2>();

    assert_eq!(p1.as_property(), p2.as_property());
    assert_eq!(hash_of(p1), hash_of(p2));
    assert!(p1.as_property() == p2.as_property());
}

#[test]
fn add_ownered_property_should_be_same() {
    let p1 = create_property();
    let p2 = p1.add_owner::<Class2>();

    assert!(std::ptr::eq(p1, p2));
}

#[test]
fn default_get_metadata_cannot_be_changed() {
    let p1 = create_property();
    let mut metadata = (*p1.get_metadata(Class1::TYPE)).clone();

    assert_panics(|| PropertyMetadata::merge(&mut metadata, &StyledPropertyMetadata::new(None)));
}

#[test]
fn add_ownered_get_metadata_cannot_be_changed() {
    let p1 = create_property();
    let p2 = p1.add_owner::<Class2>();
    let mut metadata = (*p2.get_metadata(Class2::TYPE)).clone();

    assert_panics(|| PropertyMetadata::merge(&mut metadata, &StyledPropertyMetadata::new(None)));
}
