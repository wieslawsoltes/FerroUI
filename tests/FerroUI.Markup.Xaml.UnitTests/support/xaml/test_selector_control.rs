//! Port of `Xaml/TestSelectorControl.cs`: two types whose names differ by
//! the suffix markup drops from the names of markup extensions.
//!
//! A selector names a class of the object model (a type selector holds a
//! `&'static TypeInfo`), so the two types are classes of the object model
//! here, where the managed original has two plain classes.

use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, Ref};

#[repr(C)]
pub struct TestSelectorControl {
    base: FerroObject,
}

ferro_class!(TestSelectorControl: FerroObject);
ferro_impl_classes!(TestSelectorControl: FerroObjectImpl);
ferro_class_info!(TestSelectorControl {
    new: TestSelectorControl::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml" },
});

impl TestSelectorControl {
    pub fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct() })
    }
}

#[repr(C)]
pub struct TestSelectorControlExtension {
    base: FerroObject,
}

ferro_class!(TestSelectorControlExtension: FerroObject);
ferro_impl_classes!(TestSelectorControlExtension: FerroObjectImpl);
ferro_class_info!(TestSelectorControlExtension {
    new: TestSelectorControlExtension::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml" },
});

impl TestSelectorControlExtension {
    pub fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct() })
    }
}
