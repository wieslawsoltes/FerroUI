//! The test types declared by the upstream test file
//! `Converters/GeometryTypeConverterTests.cs`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::MarkupTyped;

use crate::support::TypeModule;

// --- StringDataViewModel -----------------------------------------------------

/// A view model whose path data is text.
pub struct StringDataViewModel {
    path_data: RefCell<Option<String>>,
}

crate::test_identity_eq!(StringDataViewModel);

impl StringDataViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { path_data: RefCell::new(None) })
    }

    pub fn path_data(&self) -> Option<String> {
        self.path_data.borrow().clone()
    }

    pub fn set_path_data(&self, value: Option<String>) {
        *self.path_data.borrow_mut() = value;
    }
}

ferro_markup_type!(class StringDataViewModel {
    this: Rc<StringDataViewModel>,
    handles: [StringDataViewModel, Rc<StringDataViewModel>, Option<Rc<StringDataViewModel>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Converters",
    constructors: [() => StringDataViewModel::new],
    properties: [
        PathData: Option<String> {
            get: |this: &Rc<StringDataViewModel>| this.path_data(),
            set: |this: &Rc<StringDataViewModel>, value: Option<String>| this.set_path_data(value)
        },
    ],
});

// --- IntDataViewModel --------------------------------------------------------

/// A view model whose path data is a number.
pub struct IntDataViewModel {
    path_data: Cell<i32>,
}

crate::test_identity_eq!(IntDataViewModel);

impl IntDataViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { path_data: Cell::new(0) })
    }

    pub fn path_data(&self) -> i32 {
        self.path_data.get()
    }

    pub fn set_path_data(&self, value: i32) {
        self.path_data.set(value);
    }
}

ferro_markup_type!(class IntDataViewModel {
    this: Rc<IntDataViewModel>,
    handles: [IntDataViewModel, Rc<IntDataViewModel>, Option<Rc<IntDataViewModel>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Converters",
    constructors: [() => IntDataViewModel::new],
    properties: [
        PathData: i32 {
            get: |this: &Rc<IntDataViewModel>| this.path_data(),
            set: |this: &Rc<IntDataViewModel>, value: i32| this.set_path_data(value)
        },
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[<StringDataViewModel as MarkupTyped>::MARKUP, <IntDataViewModel as MarkupTyped>::MARKUP],
    value_types: || {
        ValueTypes::register_reference::<StringDataViewModel>();
        ValueTypes::register_reference::<IntDataViewModel>();
    },
};
