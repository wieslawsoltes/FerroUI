//! The test types declared by the upstream test file `Templates/DataTemplateTests.cs`.

use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::MarkupTyped;

use crate::support::TypeModule;

/// A class of data.
pub struct Class1;

/// A class of data that derives from [`Class1`].
pub struct Class2 {
    base: Rc<Class1>,
}

crate::test_identity_eq!(Class1, Class2);

impl Class1 {
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
}

impl Class2 {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { base: Class1::new() })
    }

    /// The object as an instance of its base class.
    pub fn base(&self) -> &Rc<Class1> {
        &self.base
    }
}

ferro_markup_type!(class Class1 {
    this: Rc<Class1>,
    handles: [Class1, Rc<Class1>, Option<Rc<Class1>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Templates",
    constructors: [() => Class1::new],
});

ferro_markup_type!(class Class2 {
    this: Rc<Class2>,
    handles: [Class2, Rc<Class2>, Option<Rc<Class2>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Templates",
    base: Rc<Class1>,
    constructors: [() => Class2::new],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[<Class1 as MarkupTyped>::MARKUP, <Class2 as MarkupTyped>::MARKUP],
    value_types: || {
        ValueTypes::register_reference::<Class1>();
        ValueTypes::register_reference::<Class2>();
        ValueTypes::register_cast::<Class2, Rc<Class1>>(|derived| derived.base().clone());
    },
};
