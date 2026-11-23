//! Port of the upstream property registry tests.

use super::*;
use crate::*;

test_class!(Class1: FerroObject);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", String::new())
    });

    ferro_property!(pub fn baz_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Baz", String::new())
    });

    ferro_property!(pub fn qux_property() -> StyledProperty<i32> {
        FerroProperty::register::<Class1, _>("Qux", 0)
    });
}

test_class!(Class2: Class1);

impl Class2 {
    ferro_property!(pub fn bar_property() -> StyledProperty<String> {
        FerroProperty::register::<Class2, _>("Bar", String::new())
    });

    ferro_property!(pub fn flob_property() -> StyledProperty<f64> {
        FerroProperty::register::<Class2, _>("Flob", 0.0)
    });

    ferro_property!(pub fn fred_property() -> StyledProperty<Option<f64>> {
        FerroProperty::register::<Class2, _>("Fred", None)
    });
}

test_class!(Class3: Class1);

impl Class3 {
    ferro_property!(pub fn attached_property() -> AttachedProperty<String> {
        AttachedOwner::attached_property().add_owner::<Class3>()
    });
}

test_class!(AttachedOwner: Class1);

impl AttachedOwner {
    ferro_property!(pub fn attached_property() -> AttachedProperty<String> {
        FerroProperty::register_attached::<AttachedOwner, Class1, _>("Attached", String::new())
    });
}

test_class!(AttachedOwner2: AttachedOwner);
test_class!(Class4: FerroObject);

/// Ensures properties are registered: the equivalent of the static field
/// initialisers of the test classes, in declaration order.
fn setup() {
    let _ = (Class1::foo_property(), Class1::baz_property(), Class1::qux_property());
    let _ = (Class2::bar_property(), Class2::flob_property(), Class2::fred_property());
    let _ = AttachedOwner::attached_property();
    let _ = Class3::attached_property();
}

fn names(properties: &[&'static FerroProperty]) -> Vec<&'static str> {
    properties.iter().map(|p| p.name()).collect()
}

#[test]
fn registered_properties_count_reflects_newly_added_attached_property() {
    setup();
    let registry = FerroPropertyRegistry::default();
    let metadata = StyledPropertyMetadata::new(Some(0));
    let property =
        AttachedProperty::<i32>::create("test", FerroObject::TYPE, FerroObject::TYPE, metadata, true, None, None);
    property.set_inherited_value_changed(Some(crate::property_store::EffectiveValue::<i32>::notify_inherited_value_changed));
    registry.register(FerroObject::TYPE, property);
    registry.register_attached(Class4::TYPE, property);
    property.add_owner::<Class4>();

    assert_eq!(1, registry.properties().len());
}

#[test]
fn get_registered_returns_registered_properties() {
    setup();
    let registered = FerroPropertyRegistry::instance().get_registered(Class1::TYPE);

    assert_eq!(vec!["Foo", "Baz", "Qux"], names(&registered));
}

#[test]
fn get_registered_returns_registered_properties_for_base_types() {
    setup();
    let registered = FerroPropertyRegistry::instance().get_registered(Class2::TYPE);

    assert_eq!(vec!["Bar", "Flob", "Fred", "Foo", "Baz", "Qux"], names(&registered));
}

#[test]
fn get_registered_attached_returns_registered_properties() {
    setup();
    let registered = FerroPropertyRegistry::instance().get_registered_attached(Class1::TYPE);

    assert!(names(&registered).contains(&"Attached"));
}

#[test]
fn get_registered_attached_returns_registered_properties_for_base_types() {
    setup();
    let registered = FerroPropertyRegistry::instance().get_registered_attached(Class2::TYPE);

    assert!(names(&registered).contains(&"Attached"));
}

#[test]
fn find_registered_finds_property() {
    setup();
    let result = FerroPropertyRegistry::instance().find_registered(Class1::TYPE, "Foo");

    assert_eq!(Some(Class1::foo_property().as_property()), result);
}

#[test]
fn find_registered_doesnt_find_nonregistered_property() {
    setup();
    let result = FerroPropertyRegistry::instance().find_registered(Class1::TYPE, "Bar");

    assert!(result.is_none());
}

#[test]
fn find_registered_finds_unqualified_attached_property_on_registering_type() {
    setup();
    let result = FerroPropertyRegistry::instance().find_registered(AttachedOwner::TYPE, "Attached");

    assert!(std::ptr::eq(AttachedOwner::attached_property().as_property(), result.expect("found")));
}

#[test]
fn find_registered_finds_add_ownered_attached_property() {
    setup();
    let result = FerroPropertyRegistry::instance().find_registered(Class3::TYPE, "Attached");

    assert!(std::ptr::eq(AttachedOwner::attached_property().as_property(), result.expect("found")));
}

#[test]
fn find_registered_doesnt_find_non_add_ownered_attached_property() {
    setup();
    let result = FerroPropertyRegistry::instance().find_registered(Class2::TYPE, "Attached");

    assert!(result.is_none());
}

test_class!(UnregisterBase: FerroObject);

impl UnregisterBase {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<UnregisterBase, _>("Foo", String::from("foodefault"))
    });
}

test_class!(UnregisterDerived: UnregisterBase);
test_class!(UnregisterDerived2: UnregisterBase);

/// Not from upstream, which has no test of `UnregisterByModule`: the
/// metadata a type overrides is gone once the type is unregistered.
#[test]
fn unregister_by_module_removes_metadata_of_types() {
    let property = UnregisterBase::foo_property();
    property.override_default_value_for(UnregisterDerived::TYPE, String::from("foooverride"));

    assert_eq!("foooverride", property.get_default_value(UnregisterDerived::TYPE));
    assert_eq!("foodefault", property.get_default_value(UnregisterBase::TYPE));

    assert!(FerroPropertyRegistry::instance().unregister_by_module([UnregisterDerived::TYPE]));

    assert_eq!("foodefault", property.get_default_value(UnregisterDerived::TYPE));
    assert_eq!("foodefault", property.get_default_value(UnregisterBase::TYPE));
}

/// Not from upstream: `FerroProperty::unregister` removes the metadata of a
/// type through the untyped property.
#[test]
fn unregister_removes_metadata_of_type() {
    let property = UnregisterBase::foo_property();
    property.override_default_value_for(UnregisterDerived2::TYPE, String::from("foooverride"));
    assert_eq!("foooverride", property.get_default_value(UnregisterDerived2::TYPE));

    property.as_property().unregister(UnregisterDerived2::TYPE);

    assert_eq!("foodefault", property.get_default_value(UnregisterDerived2::TYPE));
}

// The direct property a class answers for a property identity with is kept
// by the registry for each (class, identity) it was asked about. These are
// not ported: they hold the kept answer to what the walk of the class chain
// finds, before and after a registration.

test_class!(DirectBase: FerroObject);

impl DirectBase {
    ferro_property!(pub fn value_property() -> DirectProperty<DirectBase, i32> {
        FerroProperty::register_direct::<DirectBase, _>("Value", |_| 1, None, 0)
    });
}

test_class!(DirectMiddle: DirectBase);
test_class!(DirectLeaf: DirectMiddle);
test_class!(DirectLateMiddle: DirectBase);
test_class!(DirectLateLeaf: DirectLateMiddle);
test_class!(DirectStranger: FerroObject);

#[test]
fn direct_property_of_a_base_class_is_found_for_an_object_of_a_derived_class() {
    let property = DirectBase::value_property();
    let registry = FerroPropertyRegistry::instance();
    let leaf = DirectLeaf::new();

    // Asked twice: the second answer is the kept one.
    for _ in 0..2 {
        let found = registry.find_registered_direct(&leaf, property).expect("the property of the base class");
        assert!(std::ptr::eq(found, &**property));
        assert_eq!(1, leaf.get_direct_value(property));
    }
}

#[test]
fn direct_property_added_to_a_nearer_class_replaces_the_answer_kept_for_a_derived_class() {
    let property = DirectBase::value_property();
    let leaf = DirectLateLeaf::new();
    let other_leaf = DirectLeaf::new();
    assert_eq!(1, leaf.get_direct_value(property));
    assert_eq!(1, other_leaf.get_direct_value(property));

    let added = property.add_owner::<DirectLateMiddle>(|_| 2, None, None);

    assert_eq!(2, leaf.get_direct_value(property));
    assert_eq!(2, leaf.get_direct_value(added));
    let found = FerroPropertyRegistry::instance().find_registered_direct(&leaf, property).expect("the added property");
    assert!(std::ptr::eq(found, &**added));
    // A class beside the one the property was added to answers as before.
    assert_eq!(1, other_leaf.get_direct_value(property));
    assert_eq!(1, DirectBase::new().get_direct_value(property));
}

#[test]
fn direct_property_is_not_found_for_a_class_outside_its_owners_until_it_is_added() {
    let property = DirectBase::value_property();
    let registry = FerroPropertyRegistry::instance();
    let stranger = DirectStranger::new();

    for _ in 0..2 {
        assert!(registry.find_registered_direct(&stranger, property).is_none());
    }

    let added = property.add_owner::<DirectStranger>(|_| 3, None, None);

    let found = registry.find_registered_direct(&stranger, property).expect("the added property");
    assert!(std::ptr::eq(found, &**added));
    assert_eq!(3, stranger.get_direct_value(property));
}
