//! `NonObservableTestFixture` (`TestModels/NonObservableTestFixture.cs`).

use std::cell::RefCell;
use std::rc::Rc;

/// An object without change notification (`NonObservableTestFixture`). The ported tests use it
/// only as the type of a property; its members are kept as upstream has them.
#[derive(Default)]
#[allow(dead_code)]
pub struct NonObservableTestFixture {
    not_listening_property: RefCell<String>,
}

#[allow(dead_code)]
impl NonObservableTestFixture {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn not_listening_property(&self) -> String {
        self.not_listening_property.borrow().clone()
    }

    pub fn set_not_listening_property(&self, value: String) {
        *self.not_listening_property.borrow_mut() = value;
    }
}

impl PartialEq for NonObservableTestFixture {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
