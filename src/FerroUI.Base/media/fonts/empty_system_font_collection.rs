use crate::media::fonts::{FontCollectionBase, FontCollectionBaseImpl};
use crate::media::FontManager;

/// A system font collection without any font family.
pub(crate) struct EmptySystemFontCollection {
    base: FontCollectionBase,
}

impl EmptySystemFontCollection {
    pub fn new() -> Self {
        Self { base: FontCollectionBase::new() }
    }
}

impl FontCollectionBaseImpl for EmptySystemFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(_this: &Self) -> url::Url {
        FontManager::system_fonts_key()
    }
}
