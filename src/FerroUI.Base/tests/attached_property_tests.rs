//! Port of the upstream attached property tests.

use super::*;
use crate::*;

test_class!(Class1: FerroObject);

#[test]
fn is_attached_returns_true() {
    let property = AttachedProperty::<String>::create(
        "Foo",
        Class1::TYPE,
        FerroObject::TYPE,
        StyledPropertyMetadata::new(Some(String::new())),
        false,
        None,
        None,
    );

    assert!(property.is_attached());
}
