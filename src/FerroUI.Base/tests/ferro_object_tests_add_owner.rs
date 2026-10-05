//! Port of the upstream `AddOwner` object tests.

use super::*;
use crate::*;

test_class!(Class1: FerroObject);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", s("foodefault"))
    });
}

test_class!(Class2: FerroObject);

impl Class2 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        Class1::foo_property().add_owner::<Class2>()
    });
}

#[test]
fn add_ownered_property_retains_default_value() {
    let target = Class2::new();

    assert_eq!("foodefault", target.get_value(Class2::foo_property()));
}
