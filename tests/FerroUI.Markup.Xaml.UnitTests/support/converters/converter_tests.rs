//! The test types declared by the upstream test file `Converters/ConverterTests.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::utilities::Uri;

use crate::support::TypeModule;

/// A plain class with a URI property.
pub struct TestClassWithUri {
    uri: RefCell<Option<Uri>>,
}

crate::test_identity_eq!(TestClassWithUri);

impl TestClassWithUri {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { uri: RefCell::new(None) })
    }

    pub fn uri(&self) -> Option<Uri> {
        self.uri.borrow().clone()
    }

    pub fn set_uri(&self, value: Option<Uri>) {
        *self.uri.borrow_mut() = value;
    }
}

ferro_markup_type!(class TestClassWithUri {
    this: Rc<TestClassWithUri>,
    handles: [TestClassWithUri, Rc<TestClassWithUri>, Option<Rc<TestClassWithUri>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Converters",
    constructors: [() => TestClassWithUri::new],
    properties: [
        Uri: Option<Uri> {
            get: |this: &Rc<TestClassWithUri>| this.uri(),
            set: |this: &Rc<TestClassWithUri>, value: Option<Uri>| this.set_uri(value)
        },
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[<TestClassWithUri as MarkupTyped>::MARKUP],
    value_types: || ValueTypes::register_reference::<TestClassWithUri>(),
};
