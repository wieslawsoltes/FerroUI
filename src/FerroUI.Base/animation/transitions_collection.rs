use crate::animation::ITransition;
use crate::collections::{FerroList, ResetBehavior};
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

/// A collection of [`ITransition`] objects.
///
/// The handle is shared: clones refer to the same collection, and handles
/// compare by identity. Clearing the collection notifies a removal of every
/// item, and transitions of direct properties are rejected.
#[derive(Clone)]
pub struct Transitions(Rc<FerroList<Rc<dyn ITransition>>>);

impl Transitions {
    /// Creates an empty collection.
    pub fn new() -> Self {
        let list = FerroList::new();
        list.set_reset_behavior(ResetBehavior::Remove);
        list.set_validator(Some(Rc::new(Self::validate)));
        Self(Rc::new(list))
    }

    /// Creates a collection holding `items`.
    pub fn from_items(items: impl IntoIterator<Item = Rc<dyn ITransition>>) -> Self {
        let result = Self::new();
        for item in items {
            result.add(item);
        }
        result
    }

    /// Whether two handles refer to the same collection.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    fn validate(item: &Rc<dyn ITransition>) {
        let property = item.property();
        if property.is_direct() {
            panic!("Cannot animate direct property {} on {}.", property, item.debug_display());
        }
    }
}

impl Default for Transitions {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Transitions {
    type Target = FerroList<Rc<dyn ITransition>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for Transitions {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl fmt::Debug for Transitions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Transitions({})", self.count())
    }
}
