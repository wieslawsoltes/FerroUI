//! The test types declared by the upstream test file `Xaml/XamlSourceInfoTests.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::MarkupTyped;

use crate::support::TypeModule;

/// The view model the data template of the test is declared for.
pub struct SourceInfoTestViewModel {
    name: RefCell<Option<String>>,
}

crate::test_identity_eq!(SourceInfoTestViewModel);

impl SourceInfoTestViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(None) })
    }

    pub fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, value: Option<String>) {
        *self.name.borrow_mut() = value;
    }
}

ferro_markup_type!(class SourceInfoTestViewModel {
    this: Rc<SourceInfoTestViewModel>,
    handles: [SourceInfoTestViewModel, Rc<SourceInfoTestViewModel>, Option<Rc<SourceInfoTestViewModel>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    constructors: [() => SourceInfoTestViewModel::new],
    properties: [
        Name: Option<String> {
            get: |this: &Rc<SourceInfoTestViewModel>| this.name(),
            set: |this: &Rc<SourceInfoTestViewModel>, value: Option<String>| this.set_name(value)
        },
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[<SourceInfoTestViewModel as MarkupTyped>::MARKUP],
    value_types: || ValueTypes::register_reference::<SourceInfoTestViewModel>(),
};
