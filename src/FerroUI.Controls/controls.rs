use crate::Control;
use ferroui_base::collections::{FerroList, ResetBehavior};
use ferroui_base::{IntoRef, Ref};
use std::ops::Deref;

/// A collection of [`Control`]s.
///
/// The value is a shared handle: clones refer to the same collection, and
/// handles compare by identity.
#[derive(Clone, PartialEq, Debug)]
pub struct Controls {
    list: FerroList<Ref<Control>>,
}

impl Default for Controls {
    fn default() -> Self {
        Self::new()
    }
}

impl Controls {
    /// Creates an empty collection.
    pub fn new() -> Self {
        let list = FerroList::new();
        list.set_reset_behavior(ResetBehavior::Remove);
        Self { list }
    }

    /// Creates a collection with the initial items.
    pub fn from_items(items: impl IntoIterator<Item = Ref<Control>>) -> Self {
        let result = Self::new();
        result.list.add_range(items);
        result
    }

    /// Adds a control (of any class) to the collection.
    pub fn add(&self, item: impl IntoRef<Control>) {
        self.list.add(item.into_ref());
    }

    /// Inserts a control (of any class) at the specified index.
    pub fn insert(&self, index: usize, item: impl IntoRef<Control>) {
        self.list.insert(index, item.into_ref());
    }

    /// Removes a control (of any class) from the collection. Returns true if
    /// it was present.
    pub fn remove(&self, item: impl IntoRef<Control>) -> bool {
        self.list.remove(&item.into_ref())
    }

    /// Replaces the control at `index`.
    pub fn set(&self, index: usize, item: impl IntoRef<Control>) {
        self.list.set(index, item.into_ref());
    }
}

impl Deref for Controls {
    type Target = FerroList<Ref<Control>>;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}
