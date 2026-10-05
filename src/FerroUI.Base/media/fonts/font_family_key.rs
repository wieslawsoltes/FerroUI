use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::utilities::Uri;

/// Represents an identifier for a `FontFamily`.
#[derive(Clone, Debug)]
pub struct FontFamilyKey {
    source: Uri,
    base_uri: Option<Uri>,
    /// The keys of a composite key (see `CompositeFontFamilyKey`).
    pub(crate) keys: Option<Rc<[FontFamilyKey]>>,
}

impl FontFamilyKey {
    /// Creates a key for a source.
    pub fn new(source: Uri) -> Self {
        Self { source, base_uri: None, keys: None }
    }

    /// Creates a key for a source that is resolved against a base URI.
    pub fn with_base_uri(source: Uri, base_uri: Option<Uri>) -> Self {
        Self { source, base_uri, keys: None }
    }

    /// Gets the source of the font family.
    pub fn source(&self) -> &Uri {
        &self.source
    }

    /// Gets the base URI of the font family.
    pub fn base_uri(&self) -> Option<&Uri> {
        self.base_uri.as_ref()
    }
}

impl PartialEq for FontFamilyKey {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source && self.base_uri == other.base_uri
    }
}

impl Eq for FontFamilyKey {}

impl Hash for FontFamilyKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source.hash(state);
        if let Some(base_uri) = &self.base_uri {
            base_uri.hash(state);
        }
    }
}

impl fmt::Display for FontFamilyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.source.is_absolute_uri() {
            if let Some(base_uri) = &self.base_uri {
                return write!(f, "{}{}", base_uri.absolute_uri(), self.source.original_string());
            }
        }

        fmt::Display::fmt(&self.source, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_initialize_with_location() {
        let source = Uri::absolute("resm:FerroUI.Visuals.UnitTests#MyFont").unwrap();

        let font_family_key = FontFamilyKey::new(source);

        assert_eq!(font_family_key.source(), &Uri::absolute("resm:FerroUI.Visuals.UnitTests").unwrap());
    }

    #[test]
    fn should_initialize_with_location_and_filename() {
        let source = Uri::absolute("resm:FerroUI.Visuals.UnitTests.MyFont.ttf#MyFont").unwrap();

        let font_family_key = FontFamilyKey::new(source);

        assert_eq!(font_family_key.source(), &Uri::absolute("resm:FerroUI.Visuals.UnitTests.MyFont.ttf").unwrap());
    }

    #[test]
    fn keys_compare_and_display_with_their_base_uri() {
        let base_uri = Uri::absolute("ferres://App/").unwrap();
        let source = Uri::new("Assets/Fonts", crate::utilities::UriKind::Relative).unwrap();

        let key = FontFamilyKey::with_base_uri(source.clone(), Some(base_uri.clone()));

        assert_eq!(key, FontFamilyKey::with_base_uri(source.clone(), Some(base_uri)));
        assert_ne!(key, FontFamilyKey::new(source));
        assert_eq!(key.to_string(), "ferres://app/Assets/Fonts");
    }
}
