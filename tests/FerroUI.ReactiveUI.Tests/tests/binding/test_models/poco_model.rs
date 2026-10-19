//! `PocoModel` (`TestModels/PocoModel.cs`).

use std::cell::RefCell;
use std::rc::Rc;

/// An object without change notification (`PocoModel`).
#[derive(Default)]
pub struct PocoModel {
    value: RefCell<String>,
}

#[allow(dead_code)]
impl PocoModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn value(&self) -> String {
        self.value.borrow().clone()
    }

    pub fn set_value(&self, value: String) {
        *self.value.borrow_mut() = value;
    }
}
