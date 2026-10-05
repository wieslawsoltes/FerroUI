use crate::media::media_collection::media_collection_type;
use crate::media::MediaCollection;
use crate::Point;

media_collection_type!(
    /// Represents a collection of [`Point`] values that can be individually
    /// accessed by index.
    Points, Point
);

impl Points {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(MediaCollection::new())
    }

    /// Creates a collection holding `points`.
    pub fn from_items(points: impl IntoIterator<Item = Point>) -> Self {
        Self(MediaCollection::from_items(points))
    }
}
