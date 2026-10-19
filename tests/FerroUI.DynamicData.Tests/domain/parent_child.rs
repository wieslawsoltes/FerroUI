//! `ParentChild` (`Domain/ParentChild.cs`).

use super::Person;
use std::fmt;
use std::rc::Rc;

/// A child and their parent (`ParentChild`). Held in `Rc`; two are equal only when they are the
/// same one.
pub struct ParentChild {
    child: Rc<Person>,
    parent: Rc<Person>,
}

impl ParentChild {
    /// `new ParentChild(Person child, Person parent)`.
    pub fn new(child: Rc<Person>, parent: Rc<Person>) -> Rc<Self> {
        Rc::new(Self { child, parent })
    }

    /// The child (`Child`).
    pub fn child(&self) -> Rc<Person> {
        self.child.clone()
    }

    /// The parent (`Parent`).
    pub fn parent(&self) -> Rc<Person> {
        self.parent.clone()
    }
}

impl PartialEq for ParentChild {
    /// The same pair (reference equality).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Debug for ParentChild {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParentChild").field("child", &self.child).field("parent", &self.parent).finish()
    }
}
