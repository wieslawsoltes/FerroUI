//! The test types declared by the upstream test file `StyleTests.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::MarkupTyped;

use crate::support::TypeModule;

/// The source of the two-way binding of a setter: a plain object that
/// raises no change notifications.
#[derive(Default)]
pub struct Data {
    foo: RefCell<Option<String>>,
}

crate::test_identity_eq!(Data);

impl Data {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn foo(&self) -> Option<String> {
        self.foo.borrow().clone()
    }

    pub fn set_foo(&self, value: Option<String>) {
        *self.foo.borrow_mut() = value;
    }
}

ferro_markup_type!(class Data {
    this: Rc<Data>,
    handles: [Data, Rc<Data>, Option<Rc<Data>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    properties: [
        Foo: Option<String> {
            get: |this: &Rc<Data>| this.foo(),
            set: |this: &Rc<Data>, value: Option<String>| this.set_foo(value)
        },
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[<Data as MarkupTyped>::MARKUP],
    value_types: || ValueTypes::register_reference::<Data>(),
};
