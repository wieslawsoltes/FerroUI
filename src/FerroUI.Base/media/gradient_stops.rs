use crate::media::immutable::ImmutableGradientStop;
use crate::media::media_collection::media_collection_type;
use crate::media::{GradientStop, MediaCollection, ResetBehavior};
use crate::Ref;

media_collection_type!(
    /// A collection of [`GradientStop`]s.
    GradientStops, Ref<GradientStop>
);

impl GradientStops {
    /// Creates an empty collection.
    pub fn new() -> Self {
        let list = MediaCollection::new();
        list.set_reset_behavior(ResetBehavior::Remove);
        Self(list)
    }

    /// Creates a collection holding `items`.
    pub fn from_items(items: impl IntoIterator<Item = Ref<GradientStop>>) -> Self {
        let list = MediaCollection::from_items(items);
        list.set_reset_behavior(ResetBehavior::Remove);
        Self(list)
    }

    /// Creates an immutable copy of the gradient stops.
    pub fn to_immutable(&self) -> Vec<ImmutableGradientStop> {
        self.iter().map(|stop| ImmutableGradientStop::new(stop.offset(), stop.color())).collect()
    }
}
