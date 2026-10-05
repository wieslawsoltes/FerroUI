use crate::media::media_collection::media_collection_type;
use crate::media::{FontFeature, MediaCollection};
use crate::utilities::{FormatError, SpanStringTokenizer};

media_collection_type!(
    /// List of font feature settings.
    ///
    /// A shared handle to an observable list; equality is reference equality.
    FontFeatureCollection,
    FontFeature
);

impl FontFeatureCollection {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(MediaCollection::new())
    }

    /// Creates a collection from features.
    pub fn from_items(font_features: impl IntoIterator<Item = FontFeature>) -> Self {
        Self(MediaCollection::from_items(font_features))
    }

    /// Parses a comma separated list of font features.
    pub fn parse(s: &str) -> Result<FontFeatureCollection, FormatError> {
        let mut features = Vec::new();

        SpanStringTokenizer::with_message(s, "Invalid font feature specification.").scope(|tokenizer| {
            while let Some(token) = tokenizer.try_read_span()? {
                features.push(FontFeature::parse(token));
            }
            Ok(())
        })?;

        Ok(FontFeatureCollection::from_items(features))
    }
}
