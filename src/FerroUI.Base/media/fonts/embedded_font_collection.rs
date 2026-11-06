use crate::media::fonts::{FontCollectionBase, FontCollectionBaseImpl};

/// A font collection of the fonts found at a source: application assets
/// (`ferres:`, `resm:`), a font file or a directory of font files (`file:`).
pub struct EmbeddedFontCollection {
    base: FontCollectionBase,
    key: url::Url,
    source: url::Url,
}

impl EmbeddedFontCollection {
    /// Creates the collection identified by `key` and loads the fonts of
    /// `source` into it.
    pub fn new(key: url::Url, source: url::Url) -> Self {
        let collection = Self { base: FontCollectionBase::new(), key, source };

        FontCollectionBase::try_add_font_source(&collection, &collection.source);

        collection
    }
}

impl FontCollectionBaseImpl for EmbeddedFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> url::Url {
        this.key.clone()
    }
}
