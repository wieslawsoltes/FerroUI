//! Port of the upstream `Metadata` object tests.

use super::*;
use crate::*;
use std::cell::RefCell;

#[repr(C)]
pub struct Class1 {
    base: FerroObject,
    direct: RefCell<Option<String>>,
}

ferro_class!(Class1: FerroObject);
ferro_impl_classes!(Class1: FerroObjectImpl);

impl Class1 {
    ferro_property!(pub fn styled_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<Class1, _>("Styled", Some(s("foo")))
    });

    ferro_property!(pub fn direct_property() -> DirectProperty<Class1, Option<String>> {
        FerroProperty::register_direct::<Class1, _>("Styled", |o| o.direct(), None, Some(s("foo")))
    });

    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), direct: RefCell::new(None) }
    }

    pub fn direct(&self) -> Option<String> {
        self.direct.borrow().clone()
    }
}

#[repr(C)]
pub struct Class2 {
    base: Class1,
}

ferro_class!(Class2: Class1);
ferro_impl_classes!(Class2: FerroObjectImpl);

impl Class2 {
    fn class_init() {
        once_per_thread!({
            Class1::styled_property().override_default_value::<Class2>(Some(s("bar")));
            Class1::direct_property().override_metadata::<Class2>(DirectPropertyMetadata::new(Some(Some(s("bar")))));
        });
    }
}

#[repr(C)]
pub struct Class3 {
    base: FerroObject,
    direct: RefCell<Option<String>>,
}

ferro_class!(Class3: FerroObject);
ferro_impl_classes!(Class3: FerroObjectImpl);

impl Class3 {
    ferro_property!(pub fn styled_property() -> StyledProperty<Option<String>> {
        Class1::styled_property().add_owner::<Class3>()
    });

    ferro_property!(pub fn direct_property() -> DirectProperty<Class3, Option<String>> {
        Class1::direct_property().add_owner::<Class3>(
            |o| o.direct(),
            None,
            Some(DirectPropertyMetadata::new(Some(Some(s("baz"))))),
        )
    });

    fn class_init() {
        once_per_thread!({
            Class3::styled_property().override_default_value::<Class3>(Some(s("baz")));
        });
    }

    pub fn direct(&self) -> Option<String> {
        self.direct.borrow().clone()
    }
}

/// Ensures properties are registered (the upstream test fixture constructor).
fn setup() {
    let _ = (Class1::styled_property(), Class1::direct_property());
    Class2::class_init();
    let _ = (Class3::styled_property(), Class3::direct_property());
    Class3::class_init();
}

#[test]
fn styled_default_value_can_be_overridden_in_derived_class() {
    setup();
    let base_value = Class1::styled_property().get_default_value(Class1::TYPE);
    let derived_value = Class1::styled_property().get_default_value(Class2::TYPE);

    assert_eq!(Some(s("foo")), base_value);
    assert_eq!(Some(s("bar")), derived_value);
}

#[test]
fn styled_default_value_can_be_overridden_in_add_ownered_property() {
    setup();
    let base_value = Class1::styled_property().get_default_value(Class1::TYPE);
    let add_ownered_value = Class1::styled_property().get_default_value(Class3::TYPE);

    assert_eq!(Some(s("foo")), base_value);
    assert_eq!(Some(s("baz")), add_ownered_value);
}

#[test]
fn direct_unset_value_can_be_overridden_in_derived_class() {
    setup();
    let base_value = Class1::direct_property().get_unset_value(Class1::TYPE);
    let derived_value = Class1::direct_property().get_unset_value(Class2::TYPE);

    assert_eq!(Some(s("foo")), base_value);
    assert_eq!(Some(s("bar")), derived_value);
}

#[test]
fn direct_unset_value_can_be_overridden_in_add_ownered_property() {
    setup();
    let base_value = Class1::direct_property().get_unset_value(Class1::TYPE);
    let add_ownered_value = Class3::direct_property().get_unset_value(Class3::TYPE);

    assert_eq!(Some(s("foo")), base_value);
    assert_eq!(Some(s("baz")), add_ownered_value);
}
