//! Port of `ViewModels/DataAnnotationsErrorViewModel.cs`.
//!
//! Not ported: the validation attributes of the two properties (`[Phone]` and `[MaxLength(10)]`
//! on `PhoneNumber`, `[Range(0, 9)]` on `LessThan10`). The framework has no validation by data
//! annotations, and the markup metadata of a member has no attribute a validator reads
//! (`GAPS.md`, B001): the two text boxes of the page accept every value.

use ferroui_base::ferro_markup_type;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
pub struct DataAnnotationsErrorViewModel {
    phone_number: RefCell<Option<String>>,
    less_than_10: Cell<i32>,
}

impl PartialEq for DataAnnotationsErrorViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl DataAnnotationsErrorViewModel {
    pub fn new() -> Rc<DataAnnotationsErrorViewModel> {
        Rc::new(Self::default())
    }

    pub fn phone_number(&self) -> Option<String> {
        self.phone_number.borrow().clone()
    }

    pub fn set_phone_number(&self, value: Option<String>) {
        *self.phone_number.borrow_mut() = value;
    }

    pub fn less_than_10(&self) -> i32 {
        self.less_than_10.get()
    }

    pub fn set_less_than_10(&self, value: i32) {
        self.less_than_10.set(value);
    }
}

ferro_markup_type!(class DataAnnotationsErrorViewModel {
    this: Rc<DataAnnotationsErrorViewModel>,
    handles: [DataAnnotationsErrorViewModel, Rc<DataAnnotationsErrorViewModel>, Option<Rc<DataAnnotationsErrorViewModel>>],
    constructors: [() => DataAnnotationsErrorViewModel::new],
    properties: [
        PhoneNumber: Option<String> {
            get: |this: &Rc<DataAnnotationsErrorViewModel>| this.phone_number(),
            set: |this: &Rc<DataAnnotationsErrorViewModel>, value: Option<String>| this.set_phone_number(value)
        },
        LessThan10: i32 {
            get: |this: &Rc<DataAnnotationsErrorViewModel>| this.less_than_10(),
            set: |this: &Rc<DataAnnotationsErrorViewModel>, value: i32| this.set_less_than_10(value)
        },
    ],
});
