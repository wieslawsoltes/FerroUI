//! The test types declared by the upstream test file `Xaml/DesignModeTests.cs`:
//! the test class itself, for the static property its documents refer to.

use std::cell::RefCell;

use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_markup_type, BoxedValue};

use crate::support::TypeModule;

thread_local! {
    static SOME_STATIC_PROPERTY: RefCell<Option<BoxedValue>> = const { RefCell::new(None) };
}

/// The test class: the owner of the static property `SomeStaticProperty`.
pub struct DesignModeTests;

impl DesignModeTests {
    pub fn some_static_property() -> Option<BoxedValue> {
        SOME_STATIC_PROPERTY.with(|value| value.borrow().clone())
    }

    pub fn set_some_static_property(value: Option<BoxedValue>) {
        SOME_STATIC_PROPERTY.with(|property| *property.borrow_mut() = value);
    }
}

ferro_markup_type!(static DesignModeTests {
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    fields: [SomeStaticProperty: Option<BoxedValue> => DesignModeTests::some_static_property],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[<DesignModeTests as MarkupTyped>::MARKUP],
    value_types: || {},
};
