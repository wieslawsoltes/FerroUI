//! `ParentAndChildren` (`Domain/ParentAndChildren.cs`).

use super::Person;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// A parent, which may be missing, and their children (`ParentAndChildren`): equal by the
/// identifier of the parent. Held in `Rc`.
pub struct ParentAndChildren {
    parent: Option<Rc<Person>>,
    parent_id: Option<String>,
    children: Vec<Rc<Person>>,
}

impl ParentAndChildren {
    /// `new ParentAndChildren(Person parent, Person[] children)`: no identifier of the parent.
    pub fn new(parent: Rc<Person>, children: Vec<Rc<Person>>) -> Rc<Self> {
        Rc::new(Self { parent: Some(parent), parent_id: None, children })
    }

    /// `new ParentAndChildren(string parentId, Optional<Person> parent, Person[] children)`.
    pub fn with_parent_id(parent_id: impl Into<String>, parent: Option<Rc<Person>>, children: Vec<Rc<Person>>) -> Rc<Self> {
        Rc::new(Self { parent, parent_id: Some(parent_id.into()), children })
    }

    /// The children (`Children`).
    pub fn children(&self) -> Vec<Rc<Person>> {
        self.children.clone()
    }

    /// The number of children (`Count`).
    pub fn count(&self) -> usize {
        self.children.len()
    }

    /// The parent (`Parent`).
    pub fn parent(&self) -> Option<Rc<Person>> {
        self.parent.clone()
    }

    /// The identifier of the parent (`ParentId`).
    pub fn parent_id(&self) -> Option<String> {
        self.parent_id.clone()
    }
}

impl PartialEq for ParentAndChildren {
    /// The same identifier of the parent (`Equals(ParentAndChildren)`).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.parent_id == other.parent_id
    }
}

impl Eq for ParentAndChildren {}

impl Hash for ParentAndChildren {
    /// `GetHashCode`: the hash of the identifier of the parent.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.parent_id.hash(state);
    }
}

impl fmt::Display for ParentAndChildren {
    /// `ToString`: `Parent: {Parent}, ({Count} children)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.parent {
            Some(parent) => write!(f, "Parent: {}, ({} children)", parent, self.count()),
            None => write!(f, "Parent: , ({} children)", self.count()),
        }
    }
}

impl fmt::Debug for ParentAndChildren {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
