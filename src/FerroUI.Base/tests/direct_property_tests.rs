//! Port of the upstream direct property tests.

use super::*;
use crate::ferro_property_metadata::PropertyMetadata;
use crate::*;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[repr(C)]
pub struct Class1 {
    base: FerroObject,
    foo: RefCell<Option<String>>,
}

ferro_class!(Class1: FerroObject);
ferro_impl_classes!(Class1: FerroObjectImpl);

impl Class1 {
    ferro_property!(pub fn foo_property() -> DirectProperty<Class1, Option<String>> {
        FerroProperty::register_direct::<Class1, _>("Foo", |o| o.foo(), Some(|o, v| o.set_foo(v)), None)
    });

    pub fn foo(&self) -> Option<String> {
        self.foo.borrow().clone()
    }

    pub fn set_foo(&self, value: Option<String>) {
        self.set_and_raise(Self::foo_property(), &self.foo, value);
    }
}

test_class!(Class2: FerroObject);

fn hash_of(property: &FerroProperty) -> u64 {
    let mut hasher = DefaultHasher::new();
    property.hash(&mut hasher);
    hasher.finish()
}

fn add_owner(p1: &'static DirectProperty<Class1, Option<String>>) -> &'static DirectProperty<Class2, Option<String>> {
    p1.add_owner::<Class2>(|_| None, Some(|_, _| {}), None)
}

#[test]
fn is_direct_property_returns_true() {
    let target =
        DirectProperty::<Class1, Option<String>>::create("test", |_| None, None, DirectPropertyMetadata::new(None));

    assert!(target.is_direct());
}

#[test]
fn add_ownered_property_should_equal_original() {
    let p1 = Class1::foo_property();
    let p2 = add_owner(p1);

    assert!(!std::ptr::eq(p1.as_property(), p2.as_property()));
    assert!(p1.as_property().eq(p2.as_property()));
    assert_eq!(hash_of(p1), hash_of(p2));
    assert!(p1.as_property() == p2.as_property());
}

#[test]
fn add_ownered_property_should_have_owner_type_set() {
    let p1 = Class1::foo_property();
    let p2 = add_owner(p1);

    assert_eq!(Class2::TYPE, p2.owner_type());
}

#[test]
fn add_ownered_properties_should_share_observables() {
    let p1 = Class1::foo_property();
    let p2 = add_owner(p1);

    assert!(std::ptr::eq(p1.changed(), p2.changed()));
}

#[test]
fn default_get_metadata_cannot_be_changed() {
    let p1 = Class1::foo_property();
    let mut metadata = (*p1.get_metadata(Class1::TYPE)).clone();

    assert_panics(|| PropertyMetadata::merge(&mut metadata, &DirectPropertyMetadata::new(None)));
}

#[test]
fn add_ownered_get_metadata_cannot_be_changed() {
    let p1 = Class1::foo_property();
    let p2 = add_owner(p1);
    let mut metadata = (*p2.get_metadata(Class2::TYPE)).clone();

    assert_panics(|| PropertyMetadata::merge(&mut metadata, &DirectPropertyMetadata::new(None)));
}

#[repr(C)]
pub struct Class3 {
    base: Class1,
}

ferro_class!(Class3: Class1);
ferro_impl_classes!(Class3: FerroObjectImpl);

// Not an upstream test. Upstream merges the metadata of an added owner with
// the metadata of the original owner, not with whatever metadata the property
// resolves to for the new owner.
#[test]
fn add_owner_merges_metadata_with_original_owner() {
    use crate::data::BindingMode;

    let p1 = DirectProperty::<Class1, Option<String>>::create(
        "test",
        |_| None,
        None,
        DirectPropertyMetadata::new(None).with_default_binding_mode(BindingMode::TwoWay),
    );
    p1.override_metadata::<Class3>(DirectPropertyMetadata::new(None).with_default_binding_mode(BindingMode::OneTime));

    let p2 = p1.add_owner::<Class3>(|_| None, None, Some(DirectPropertyMetadata::new(Some(Some(s("unset"))))));

    assert_eq!(BindingMode::TwoWay, p2.get_metadata(Class3::TYPE).default_binding_mode());
    assert_eq!(Some(s("unset")), p2.get_unset_value(Class3::TYPE));
}
