use crate::media::media_collection::media_collection_type;
use crate::media::{Drawing, MediaCollection, ResetBehavior};
use crate::Ref;

media_collection_type!(
    /// A collection of [`Drawing`]s.
    DrawingCollection, Ref<Drawing>
);

impl DrawingCollection {
    /// Creates an empty collection.
    pub fn new() -> Self {
        let list = MediaCollection::new();
        list.set_reset_behavior(ResetBehavior::Remove);
        Self(list)
    }

    /// Creates a collection holding `items`.
    pub fn from_items(items: impl IntoIterator<Item = Ref<Drawing>>) -> Self {
        let list = MediaCollection::from_items(items);
        list.set_reset_behavior(ResetBehavior::Remove);
        Self(list)
    }
}
