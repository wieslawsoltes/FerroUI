use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::str::FromStr;

use crate::media::composite_font_family_key::CompositeFontFamilyKey;
use crate::media::font_source_identifier::FontSourceIdentifier;
use crate::media::fonts::{FamilyNameCollection, FontFamilyKey};
use crate::media::{FontManager, Typeface};
use crate::utilities::{FormatError, Uri, UriKind};

struct FontFamilyInner {
    family_names: FamilyNameCollection,
    key: Option<FontFamilyKey>,
}

/// Represents a family of related fonts.
///
/// A cheap to clone handle; equality compares the family names and the key.
#[derive(Clone)]
pub struct FontFamily(Rc<FontFamilyInner>);

impl FontFamily {
    /// The name of the placeholder family that resolves to the font manager's
    /// default font family.
    pub const DEFAULT_FONT_FAMILY_NAME: &'static str = "$Default";

    /// Creates a font family from a family name, a comma separated list of
    /// family names or `source#name` segments.
    ///
    /// Panics when `name` is empty.
    pub fn new(name: &str) -> Self {
        Self::with_base_uri(None, name)
    }

    /// Creates a font family whose relative sources are resolved against `base_uri`.
    ///
    /// Panics when `name` is empty or `base_uri` is not absolute.
    pub fn with_base_uri(base_uri: Option<&Uri>, name: &str) -> Self {
        if name.is_empty() {
            panic!("name must not be empty");
        }

        if let Some(base_uri) = base_uri {
            if !base_uri.is_absolute_uri() {
                panic!("Base uri must be an absolute uri.");
            }
        }

        let font_sources = Self::get_font_source_identifier(name);

        let family_names = FamilyNameCollection::from_font_sources(&font_sources);

        let key = if font_sources.len() == 1 {
            let single_source = &font_sources[0];

            single_source.source.as_ref().map(|source| {
                if source.is_absolute_uri() {
                    FontFamilyKey::new(source.clone())
                } else {
                    FontFamilyKey::with_base_uri(source.clone(), base_uri.cloned())
                }
            })
        } else {
            let keys = font_sources
                .iter()
                .map(|font_source| match &font_source.source {
                    Some(source) => FontFamilyKey::with_base_uri(source.clone(), base_uri.cloned()),
                    None => FontFamilyKey::new(system_font_uri(&font_source.name)),
                })
                .collect();

            let source = Uri::try_create(&format!("{}:{}", FontManager::COMPOSITE_FONT_SCHEME, name), UriKind::Absolute)
                .expect("a composite font key is an absolute uri");

            Some(CompositeFontFamilyKey::new(source, keys))
        };

        FontFamily(Rc::new(FontFamilyInner { family_names, key }))
    }

    /// Represents the default font family.
    pub fn default_family() -> FontFamily {
        thread_local! {
            static DEFAULT: FontFamily = FontFamily::new(FontFamily::DEFAULT_FONT_FAMILY_NAME);
        }
        DEFAULT.with(Clone::clone)
    }

    /// Gets the primary family name of the font family.
    pub fn name(&self) -> &str {
        self.0.family_names.primary_family_name()
    }

    /// Gets the family names.
    pub fn family_names(&self) -> &FamilyNameCollection {
        &self.0.family_names
    }

    /// Gets the key for associated assets.
    pub fn key(&self) -> Option<&FontFamilyKey> {
        self.0.key.as_ref()
    }

    /// Returns the typefaces for the font family.
    pub fn family_typefaces(&self) -> Vec<Typeface> {
        FontManager::current().get_family_typefaces(self)
    }

    /// Whether both handles refer to the same instance (C# `ReferenceEquals`).
    pub fn ptr_eq(&self, other: &FontFamily) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    fn get_font_source_identifier(name: &str) -> Vec<FontSourceIdentifier> {
        let mut result = Vec::with_capacity(1);

        for raw_segment in name.split(',') {
            let segment = raw_segment.trim();

            let mut identifier = None;

            // Check if there is exactly one '#' (i.e., segment is in the format "path#innerName").
            if let Some(separator_index) = segment.find('#') {
                if !segment[separator_index + 1..].contains('#') {
                    let path = segment[..separator_index].trim();
                    let inner_name = segment[separator_index + 1..].trim();

                    if path.is_empty() {
                        identifier = Some(FontSourceIdentifier::new(inner_name.to_owned(), None));
                    } else {
                        let relative =
                            if path.contains('/') { Uri::try_create(path, UriKind::Relative) } else { None };

                        if let Some(source) = relative.or_else(|| Uri::try_create(path, UriKind::Absolute)) {
                            identifier = Some(FontSourceIdentifier::new(inner_name.to_owned(), Some(source)));
                        }
                    }
                }
            }

            // If we didn't manage to match it to any known format, treat the entire segment as the font name.
            result.push(identifier.unwrap_or_else(|| FontSourceIdentifier::new(segment.to_owned(), None)));
        }

        result
    }

    /// Parses a `FontFamily` string.
    pub fn parse(s: &str) -> Result<FontFamily, FormatError> {
        Self::parse_with_base_uri(s, None)
    }

    /// Parses a `FontFamily` string, resolving relative sources against `base_uri`.
    pub fn parse_with_base_uri(s: &str, base_uri: Option<&Uri>) -> Result<FontFamily, FormatError> {
        if s.is_empty() {
            return Err(FormatError::new("Specified family is not supported."));
        }

        if base_uri.is_some_and(|base_uri| !base_uri.is_absolute_uri()) {
            return Err(FormatError::new("Base uri must be an absolute uri."));
        }

        Ok(FontFamily::with_base_uri(base_uri, s))
    }
}

/// `systemfont:<name>`; the name is kept verbatim in the URI text.
fn system_font_uri(name: &str) -> Uri {
    Uri::try_create(&format!("{}:{}", FontManager::SYSTEM_FONT_SCHEME, name), UriKind::Absolute)
        // An empty name (e.g. a trailing comma) has no valid URI form; use a
        // placeholder path so the key still exists and never matches a family.
        .unwrap_or_else(|| {
            Uri::try_create(&format!("{}:/", FontManager::SYSTEM_FONT_SCHEME), UriKind::Absolute)
                .expect("the placeholder is an absolute uri")
        })
}

impl Default for FontFamily {
    fn default() -> Self {
        FontFamily::default_family()
    }
}

impl From<&str> for FontFamily {
    fn from(s: &str) -> Self {
        FontFamily::new(s)
    }
}

impl FromStr for FontFamily {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        FontFamily::parse(s)
    }
}

impl fmt::Display for FontFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(key) = &self.0.key {
            return write!(f, "{}#{}", key, self.0.family_names);
        }

        fmt::Display::fmt(&self.0.family_names, f)
    }
}

impl fmt::Debug for FontFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FontFamily({self})")
    }
}

impl PartialEq for FontFamily {
    fn eq(&self, other: &Self) -> bool {
        if Rc::ptr_eq(&self.0, &other.0) {
            return true;
        }

        if self.0.key != other.0.key {
            return false;
        }

        other.0.family_names == self.0.family_names
    }
}

impl Eq for FontFamily {}

impl Hash for FontFamily {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.family_names.hash(state);
        self.0.key.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;

    use super::*;

    fn hash_of(font_family: &FontFamily) -> u64 {
        let mut hasher = DefaultHasher::new();
        font_family.hash(&mut hasher);
        hasher.finish()
    }

    const NAMES: [&str; 4] = [
        "Font A",
        "Font A, Font B",
        "resm: FerroUI.Visuals.UnitTests#MyFont",
        "ferres://FerroUI.Visuals.UnitTests/Assets/Fonts#MyFont",
    ];

    const DIFFERENT_NAMES: [(&str, &str); 2] =
        [("Font A, Font B", "Font B, Font A"), ("Font A, Font B", "Font A, Font C")];

    #[test]
    fn should_implicitly_convert_string_to_font_family() {
        let font_family: FontFamily = "Arial".into();

        assert_eq!(font_family, FontFamily::new("Arial"));
    }

    #[test]
    fn should_have_equal_hash() {
        for s in NAMES {
            assert_eq!(hash_of(&FontFamily::new(s)), hash_of(&FontFamily::new(s)), "{s}");
        }
    }

    #[test]
    fn should_not_have_equal_hash() {
        for (a, b) in DIFFERENT_NAMES {
            assert_ne!(hash_of(&FontFamily::new(a)), hash_of(&FontFamily::new(b)));
        }
    }

    #[test]
    fn should_be_equal() {
        for s in NAMES {
            assert_eq!(FontFamily::new(s), FontFamily::new(s), "{s}");
        }
    }

    #[test]
    fn should_not_be_equal() {
        for (a, b) in DIFFERENT_NAMES {
            assert_ne!(FontFamily::new(a), FontFamily::new(b));
        }
    }

    #[test]
    fn should_parse_font_family_with_system_font_name() {
        let font_family = FontFamily::parse("Courier New").unwrap();

        assert_eq!(font_family.name(), "Courier New");
    }

    #[test]
    fn should_parse_font_family_with_fallbacks() {
        let font_family = FontFamily::parse("Courier New, Times New Roman").unwrap();

        assert_eq!(font_family.name(), "Courier New");
        assert_eq!(font_family.family_names().count(), 2);
        assert_eq!(font_family.family_names().iter().last(), Some("Times New Roman"));
    }

    #[test]
    fn should_parse_font_family_with_resource_folder() {
        let source = Uri::absolute("resm:FerroUI.Visuals.UnitTests#MyFont").unwrap();

        let key = FontFamilyKey::new(source.clone());

        let font_family = FontFamily::parse(source.original_string()).unwrap();

        assert_eq!(font_family.name(), "MyFont");
        assert_eq!(font_family.key(), Some(&key));
    }

    #[test]
    fn should_parse_font_family_with_resource_filename() {
        let source = Uri::absolute("resm:FerroUI.Visuals.UnitTests.MyFont.ttf#MyFont").unwrap();

        let key = FontFamilyKey::new(source.clone());

        let font_family = FontFamily::parse(source.original_string()).unwrap();

        assert_eq!(font_family.name(), "MyFont");
        assert_eq!(font_family.key(), Some(&key));
    }

    #[test]
    fn should_create_font_family_from_uri() {
        for name in [
            "resm:FerroUI.Visuals.UnitTests/Assets/Fonts#MyFont",
            "ferres://FerroUI.Visuals.UnitTests/Assets/Fonts#MyFont",
        ] {
            let font_family = FontFamily::new(name);

            assert_eq!(font_family.name(), "MyFont");
            assert!(font_family.key().is_some());
        }
    }

    #[test]
    fn should_create_font_family_from_uri_with_base_uri() {
        for (base, name) in [
            (None, "resm:FerroUI.Visuals.UnitTests.Assets.Fonts#MyFont"),
            (Some("ferres://FerroUI.Visuals.UnitTests/Assets/Fonts"), "/#MyFont"),
            (Some("ferres://FerroUI.Visuals.UnitTests"), "/Assets/Fonts#MyFont"),
        ] {
            let base_uri = base.map(|base| Uri::absolute(base).unwrap());

            let font_family = FontFamily::with_base_uri(base_uri.as_ref(), name);

            assert_eq!(font_family.name(), "MyFont");
            assert!(font_family.key().is_some());
        }
    }

    #[test]
    fn should_parse_font_family_with_base_uri() {
        for (base_uri, s, expected_name, expected_uri) in [
            (None, "Arial", "Arial", None),
            (
                None,
                "resm:FerroUI.Skia.UnitTests.Fonts?assembly=FerroUI.Skia.UnitTests#Manrope",
                "Manrope",
                Some("resm:FerroUI.Skia.UnitTests.Fonts?assembly=FerroUI.Skia.UnitTests"),
            ),
            (None, "ferres://FerroUI.Fonts.Inter/Assets#Inter", "Inter", None),
            (Some("ferres://FerroUI.Fonts.Inter"), "/Assets#Inter", "Inter", Some("ferres://FerroUI.Fonts.Inter/Assets")),
            (
                Some("ferres://ControlCatalog/MainWindow.xaml"),
                "ferres://FerroUI.Fonts.Inter/Assets#Inter",
                "Inter",
                Some("ferres://FerroUI.Fonts.Inter/Assets"),
            ),
        ] {
            let b = base_uri.map(|base_uri| Uri::absolute(base_uri).unwrap());
            let expected_uri = expected_uri.map(|uri| Uri::absolute(uri).unwrap().absolute_uri().to_owned());

            let font_family = FontFamily::parse_with_base_uri(s, b.as_ref()).unwrap();

            assert_eq!(font_family.name(), expected_name);

            if let Some(expected_uri) = expected_uri {
                let key = font_family.key().expect("the family has a key");

                match key.base_uri() {
                    Some(base_uri) => assert!(base_uri.is_absolute_uri()),
                    None => assert!(key.source().is_absolute_uri()),
                }

                let font_uri = key.source().ensure_absolute(key.base_uri());

                assert_eq!(font_uri.absolute_uri(), expected_uri);
            }
        }
    }

    #[test]
    fn should_parse_relative_path() {
        for (base_uri_string, path, expected) in [
            ("ferres://MyAssembly/", "Some/Path/#FontName", "ferres://MyAssembly/Some/Path/"),
            ("ferres://MyAssembly/", "./Some/Path/#FontName", "ferres://MyAssembly/Some/Path/"),
            ("ferres://MyAssembly/sub/", "../Some/Path/#FontName", "ferres://MyAssembly/Some/Path/"),
        ] {
            let base_uri = Uri::absolute(base_uri_string).unwrap();

            let font_family = FontFamily::parse_with_base_uri(path, Some(&base_uri)).unwrap();

            let key = font_family.key().expect("the family has a key");
            let actual = Uri::combine(key.base_uri().expect("the key has a base uri"), key.source());

            assert_eq!(actual.absolute_uri(), Uri::absolute(expected).unwrap().absolute_uri());
        }
    }

    #[test]
    fn parse_rejects_an_empty_name_and_a_relative_base_uri() {
        assert!(FontFamily::parse("").is_err());

        let relative = Uri::new("/Assets", UriKind::Relative).unwrap();

        assert!(FontFamily::parse_with_base_uri("Arial", Some(&relative)).is_err());
    }
}
