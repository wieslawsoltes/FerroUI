//! The static initialisation of a class in a build without the tables of types, as an application
//! links this crate: the unit tests of the crate always have the tables (`cfg(test)`).
//!
//! The property registry promises that a type is listed in full whether or not it has an
//! instance. Every test runs on its own thread, so it starts with nothing initialised.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::*;

#[repr(C)]
pub struct NoTablesClass {
    base: FerroObject,
}

ferro_class!(NoTablesClass: FerroObject);
ferro_impl_classes!(NoTablesClass: FerroObjectImpl);

impl NoTablesClass {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

ferro_properties! {
    impl NoTablesClass {
        pub fn styled_value_property() -> StyledProperty<i32> {
            FerroProperty::register::<NoTablesClass, _>("NoTablesStyled", 1)
        }

        pub fn attached_value_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<NoTablesClass, FerroObject, _>("NoTablesAttached", false)
        }
    }
}

fn names(properties: &[&'static FerroProperty]) -> Vec<String> {
    properties.iter().map(|property| property.name().to_string()).collect()
}

#[test]
fn this_target_is_built_without_the_tables_of_types() {
    assert!(!ValueTypes::TYPE_TABLES);
}

#[test]
fn a_class_without_an_instance_is_listed_in_full() {
    let listed = names(&FerroPropertyRegistry::instance().get_registered(NoTablesClass::TYPE));
    assert!(listed.iter().any(|name| name == "NoTablesStyled"), "{listed:?}");
}

#[test]
fn the_attached_properties_of_a_class_without_an_instance_are_listed() {
    let listed = names(&FerroPropertyRegistry::instance().get_registered_attached(NoTablesClass::TYPE));
    assert!(listed.iter().any(|name| name == "NoTablesAttached"), "{listed:?}");
}

#[test]
fn a_class_is_initialised_through_its_runtime_type() {
    NoTablesClass::TYPE.ensure_class_init();
    let listed = names(&FerroPropertyRegistry::instance().get_registered(NoTablesClass::TYPE));
    assert!(listed.iter().any(|name| name == "NoTablesStyled"), "{listed:?}");
}

#[test]
fn a_class_with_an_instance_is_listed() {
    let _instance = NoTablesClass::new();
    let listed = names(&FerroPropertyRegistry::instance().get_registered(NoTablesClass::TYPE));
    assert!(listed.iter().any(|name| name == "NoTablesStyled"), "{listed:?}");
}
