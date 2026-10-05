use crate::media::media_collection::media_collection_type;
use crate::media::{MediaCollection, PathFigure, PathGeometry, PathSegment};
use crate::utilities::FormatError;
use crate::Ref;

media_collection_type!(
    /// A collection of [`PathFigure`]s.
    PathFigures, Ref<PathFigure>
);

impl PathFigures {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(MediaCollection::new())
    }

    /// Creates a collection holding `items`.
    pub fn from_items(items: impl IntoIterator<Item = Ref<PathFigure>>) -> Self {
        Self(MediaCollection::from_items(items))
    }

    /// Parses the specified path data to path figures.
    pub fn parse(path_data: &str) -> Result<PathFigures, FormatError> {
        let path_geometry = PathGeometry::parse(path_data)?;
        Ok(path_geometry.figures().expect("a parsed path geometry has figures"))
    }
}

media_collection_type!(
    /// A collection of [`PathSegment`]s.
    PathSegments, Ref<PathSegment>
);

impl PathSegments {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(MediaCollection::new())
    }

    /// Creates a collection holding `items`.
    pub fn from_items(items: impl IntoIterator<Item = Ref<PathSegment>>) -> Self {
        Self(MediaCollection::from_items(items))
    }

    /// Creates an empty collection with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let _ = capacity;
        Self(MediaCollection::new())
    }
}
