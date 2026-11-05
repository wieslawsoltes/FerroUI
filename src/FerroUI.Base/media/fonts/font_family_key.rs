use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use url::{Position, Url};

use crate::utilities::UriRef;

/// Represents an identifier for a `FontFamily`.
///
/// The source is a [`UriRef`]: an absolute URL (`ferres://App/Assets#Inter`
/// without the name, `fonts:Inter`), or a reference relative to the base URI
/// of the document that names the font (`/Assets/Fonts`).
///
/// Two keys are equal when their sources and their base URIs are, compared
/// as .NET's `Uri` compares them (decision O5 of
/// docs/idiomatic-rust-signatures.md, the explicit key of the .NET rule): an
/// absolute URL without its fragment, a relative reference by its whole text.
/// The comparison reads the text of the URLs and allocates nothing, since
/// font families compare their keys on the hot paths of text layout.
#[derive(Clone, Debug)]
pub struct FontFamilyKey {
    source: UriRef,
    base_uri: Option<Url>,
    /// The relative source resolved against the base URI, once: what the font
    /// manager looks the collection up by.
    resolved: Option<Url>,
    /// The keys of a composite key (see `CompositeFontFamilyKey`).
    pub(crate) keys: Option<Rc<[FontFamilyKey]>>,
}

impl FontFamilyKey {
    /// Creates a key for a source.
    pub fn new(source: UriRef) -> Self {
        Self::with_base_uri(source, None)
    }

    /// Creates a key for a source that is resolved against a base URI.
    pub fn with_base_uri(source: UriRef, base_uri: Option<Url>) -> Self {
        let resolved = match (&source, &base_uri) {
            (UriRef::Relative(_), Some(base_uri)) => source.resolve(base_uri).ok(),
            _ => None,
        };

        Self { source, base_uri, resolved, keys: None }
    }

    /// Gets the source of the font family.
    pub fn source(&self) -> &UriRef {
        &self.source
    }

    /// Gets the base URI of the font family.
    pub fn base_uri(&self) -> Option<&Url> {
        self.base_uri.as_ref()
    }

    /// The URL of the source (C# `Source.EnsureAbsolute(BaseUri)`): the
    /// absolute source, or the relative one resolved against the base URI;
    /// `None` for a relative source without a base URI.
    pub(crate) fn absolute_source(&self) -> Option<&Url> {
        self.source.as_url().or(self.resolved.as_ref())
    }
}

/// The text a URL compares by in a key: without its fragment, as `Uri`
/// compares absolute URIs.
fn url_identity(url: &Url) -> &str {
    &url[..Position::AfterQuery]
}

/// The text a source compares by in a key, and whether it is relative: an
/// absolute URL without its fragment, a relative reference as written.
fn source_identity(source: &UriRef) -> (bool, &str) {
    match source {
        UriRef::Absolute(url) => (false, url_identity(url)),
        UriRef::Relative(text) => (true, text),
    }
}

impl PartialEq for FontFamilyKey {
    fn eq(&self, other: &Self) -> bool {
        source_identity(&self.source) == source_identity(&other.source)
            && self.base_uri.as_ref().map(url_identity) == other.base_uri.as_ref().map(url_identity)
    }
}

impl Eq for FontFamilyKey {}

impl Hash for FontFamilyKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        source_identity(&self.source).hash(state);
        if let Some(base_uri) = &self.base_uri {
            url_identity(base_uri).hash(state);
        }
    }
}

impl fmt::Display for FontFamilyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let (UriRef::Relative(source), Some(base_uri)) = (&self.source, &self.base_uri) {
            return write!(f, "{}{}", base_uri.as_str(), source);
        }

        fmt::Display::fmt(&self.source, f)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;

    use super::*;

    fn reference(text: &str) -> UriRef {
        UriRef::parse(text).unwrap()
    }

    fn hash_of(key: &FontFamilyKey) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn should_initialize_with_location() {
        let source = reference("resm:FerroUI.Visuals.UnitTests#MyFont");

        let font_family_key = FontFamilyKey::new(source);

        // The source keeps its fragment, and the key compares without it.
        assert_eq!(font_family_key.source(), &reference("resm:FerroUI.Visuals.UnitTests#MyFont"));
        assert_eq!(font_family_key, FontFamilyKey::new(reference("resm:FerroUI.Visuals.UnitTests")));
    }

    #[test]
    fn should_initialize_with_location_and_filename() {
        let source = reference("resm:FerroUI.Visuals.UnitTests.MyFont.ttf#MyFont");

        let font_family_key = FontFamilyKey::new(source);

        assert_eq!(font_family_key, FontFamilyKey::new(reference("resm:FerroUI.Visuals.UnitTests.MyFont.ttf")));
    }

    #[test]
    fn keys_compare_and_display_with_their_base_uri() {
        let base_uri = UriRef::parse_absolute("ferres://App/").unwrap();
        let source = reference("Assets/Fonts");

        let key = FontFamilyKey::with_base_uri(source.clone(), Some(base_uri.clone()));

        assert_eq!(key, FontFamilyKey::with_base_uri(source.clone(), Some(base_uri)));
        assert_ne!(key, FontFamilyKey::new(source));
        assert_eq!(key.to_string(), "ferres://app/Assets/Fonts");
        assert_eq!(key.absolute_source().map(Url::as_str), Some("ferres://app/Assets/Fonts"));
    }

    #[test]
    fn keys_compare_as_uri_without_the_fragment_of_a_url() {
        let with_fragment = FontFamilyKey::new(reference("ferres://app/Assets/Fonts#Inter"));
        let without_fragment = FontFamilyKey::new(reference("ferres://app/Assets/Fonts"));

        assert_eq!(with_fragment, without_fragment);
        assert_eq!(hash_of(&with_fragment), hash_of(&without_fragment));

        // A relative reference compares by its whole text, and never equals a URL of the same text.
        assert_ne!(FontFamilyKey::new(reference("Fonts#A")), FontFamilyKey::new(reference("Fonts#B")));
        assert_ne!(FontFamilyKey::new(reference("fonts:Inter")), FontFamilyKey::new(UriRef::Relative("fonts:Inter".into())));

        // The base URI compares without its fragment too.
        let base = |text: &str| Some(UriRef::parse_absolute(text).unwrap());
        assert_eq!(
            FontFamilyKey::with_base_uri(reference("Fonts"), base("ferres://app/Main.xaml#a")),
            FontFamilyKey::with_base_uri(reference("Fonts"), base("ferres://app/Main.xaml"))
        );
        assert_ne!(
            FontFamilyKey::with_base_uri(reference("Fonts"), base("ferres://app/Main.xaml")),
            FontFamilyKey::with_base_uri(reference("Fonts"), base("ferres://other/Main.xaml"))
        );
    }

    #[test]
    fn a_relative_source_without_a_base_uri_has_no_absolute_source() {
        assert_eq!(FontFamilyKey::new(reference("/Assets/Fonts")).absolute_source(), None);
        assert_eq!(
            FontFamilyKey::new(reference("fonts:Inter")).absolute_source().map(Url::as_str),
            Some("fonts:Inter")
        );
    }
}
