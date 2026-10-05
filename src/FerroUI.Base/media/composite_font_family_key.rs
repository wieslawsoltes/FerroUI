use std::rc::Rc;

use crate::media::fonts::FontFamilyKey;
use crate::utilities::Uri;

/// A font family key made of the keys of several families (a family name with
/// fallbacks, e.g. `"Inter, Segoe UI"`).
///
/// Upstream models this as a subclass of `FontFamilyKey`; here a composite key
/// is a [`FontFamilyKey`] that carries its component keys, created with
/// [`CompositeFontFamilyKey::new`] and inspected with
/// [`CompositeFontFamilyKey::keys`].
pub(crate) struct CompositeFontFamilyKey;

impl CompositeFontFamilyKey {
    pub fn new(source: Uri, keys: Vec<FontFamilyKey>) -> FontFamilyKey {
        let mut key = FontFamilyKey::new(source);
        key.keys = Some(Rc::from(keys));
        key
    }

    /// The component keys when `key` is a composite key (C# `key is CompositeFontFamilyKey`).
    pub fn keys(key: &FontFamilyKey) -> Option<&[FontFamilyKey]> {
        key.keys.as_deref()
    }
}
