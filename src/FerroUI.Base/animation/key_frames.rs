use crate::animation::KeyFrame;
use crate::collections::{FerroList, ResetBehavior};
use crate::Ref;
use std::ops::Deref;

/// A collection of [`KeyFrame`]s. Clearing it notifies a removal of every
/// item.
///
/// The value is a shared handle: clones refer to the same collection, and
/// handles compare by identity.
#[derive(Clone, PartialEq, Debug)]
pub struct KeyFrames(FerroList<Ref<KeyFrame>>);

impl KeyFrames {
    /// Creates an empty collection.
    pub fn new() -> Self {
        let list = FerroList::new();
        list.set_reset_behavior(ResetBehavior::Remove);
        Self(list)
    }

    /// Creates a collection holding `items`.
    pub fn from_items(items: impl IntoIterator<Item = Ref<KeyFrame>>) -> Self {
        let list = FerroList::from_items(items);
        list.set_reset_behavior(ResetBehavior::Remove);
        Self(list)
    }
}

impl Default for KeyFrames {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for KeyFrames {
    type Target = FerroList<Ref<KeyFrame>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
