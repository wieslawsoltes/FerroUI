//! `NonObservableTestFixture` (`ReactiveUI.Tests/WhenAny/Mockups/NonObservableTestFixture.cs`).

use super::TestFixture;
use std::cell::RefCell;
use std::rc::Rc;

/// A plain object that does not notify its changes, to check the observation of objects that are
/// not `INotifyPropertyChanged` (`NonObservableTestFixture`).
#[derive(Default)]
pub struct NonObservableTestFixture {
    child: RefCell<Option<Rc<TestFixture>>>,
}

/// Fixtures compare by identity, as the reference types of upstream do.
impl PartialEq for NonObservableTestFixture {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl NonObservableTestFixture {
    /// A fixture without a child, shared as the objects of upstream are.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// `Child`.
    pub fn child(&self) -> Option<Rc<TestFixture>> {
        self.child.borrow().clone()
    }

    /// Sets `Child`, without notification.
    pub fn set_child(&self, value: Option<Rc<TestFixture>>) {
        *self.child.borrow_mut() = value;
    }
}
