use std::cell::RefCell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::str::FromStr;

use rustc_hash::FxHashMap;

use crate::media::composite_font_family_key::CompositeFontFamilyKey;
use crate::media::font_source_identifier::FontSourceIdentifier;
use crate::media::fonts::{FamilyNameCollection, FontFamilyKey};
use crate::media::{FontManager, Typeface};
use crate::utilities::{FormatError, UriRef};

#[derive(Clone)]
struct FontFamilyInner {
    family_names: FamilyNameCollection,
    key: Option<FontFamilyKey>,
    automatic_optical_sizing: bool,
}

/// Represents a family of related fonts.
///
/// A cheap to clone handle; equality compares the names, key and optical sizing.
#[derive(Clone)]
pub struct FontFamily(Rc<FontFamilyInner>);

/// The families a thread made from text, by the text ([`FontFamily::with_base_uri`]).
#[derive(Default)]
struct Made {
    /// For a text, the families made from it: one, unless the text names a source that is
    /// resolved against the base URI, which then tells them apart (`None` for a family that
    /// does not depend on the base URI it was made with).
    families: FxHashMap<Box<str>, Vec<(Option<url::Url>, FontFamily)>>,
    /// How many families the table holds.
    count: usize,
}

impl Made {
    /// The families the table holds at most: an application names a handful. A table that is
    /// full is emptied, so text that is made up at run time costs a bounded memory.
    const CAPACITY: usize = 256;

    /// The longest text the table keeps a family for.
    const MAX_TEXT: usize = 256;

    fn find(&self, base_uri: Option<&url::Url>, name: &str) -> Option<FontFamily> {
        let families = self.families.get(name)?;
        families.iter().find(|(base, _)| base.is_none() || base.as_ref() == base_uri).map(|(_, family)| family.clone())
    }

    fn keep(&mut self, base_uri: Option<url::Url>, name: &str, family: &FontFamily) {
        if name.len() > Self::MAX_TEXT {
            return;
        }
        if self.count >= Self::CAPACITY {
            self.families.clear();
            self.count = 0;
        }
        self.families.entry(name.into()).or_default().push((base_uri, family.clone()));
        self.count += 1;
    }
}

crate::ferro_thread_local! {
    static MADE: RefCell<Made> = RefCell::new(Made::default());
}

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

    /// Creates a font family whose relative sources are resolved against
    /// `base_uri` (the URL of the document that names the family).
    ///
    /// A family is what its text and its base URI make it and never changes, so a text is read
    /// once on a thread: the families made from it afterwards are the one made first
    /// ([`ptr_eq`](Self::ptr_eq) holds for them). An application writes the family of a text
    /// where the text is built, and a template names it once for every element made from it
    /// (a FerroUI addition, docs/porting/desktop-performance.md, section 9.99).
    ///
    /// Panics when `name` is empty.
    pub fn with_base_uri(base_uri: Option<&url::Url>, name: &str) -> Self {
        if name.is_empty() {
            panic!("name must not be empty");
        }

        if let Some(made) = MADE.with(|made| made.borrow().find(base_uri, name)) {
            return made;
        }
        let (family, uses_base_uri) = Self::read(base_uri, name);
        let base_uri = if uses_base_uri { base_uri.cloned() } else { None };
        // A family that depends on a base URI it was not given is kept for that: `None` is the
        // mark of the families that do not depend on one.
        if !uses_base_uri || base_uri.is_some() {
            MADE.with(|made| made.borrow_mut().keep(base_uri, name, &family));
        }
        family
    }

    /// Reads the family `name` describes, and whether it depends on `base_uri`.
    fn read(base_uri: Option<&url::Url>, name: &str) -> (Self, bool) {
        let font_sources = Self::get_font_source_identifier(name);

        let family_names = FamilyNameCollection::from_font_sources(&font_sources);

        let mut uses_base_uri = false;
        let key = if font_sources.len() == 1 {
            let single_source = &font_sources[0];

            single_source.source.as_ref().map(|source| {
                if source.is_relative() {
                    uses_base_uri = true;
                    FontFamilyKey::with_base_uri(source.clone(), base_uri.cloned())
                } else {
                    FontFamilyKey::new(source.clone())
                }
            })
        } else {
            let keys = font_sources
                .iter()
                .map(|font_source| match &font_source.source {
                    Some(source) => {
                        uses_base_uri = true;
                        FontFamilyKey::with_base_uri(source.clone(), base_uri.cloned())
                    }
                    None => FontFamilyKey::new(UriRef::from(named_font_url(
                        FontManager::SYSTEM_FONT_SCHEME,
                        &font_source.name,
                    ))),
                })
                .collect();

            let source = named_font_url(FontManager::COMPOSITE_FONT_SCHEME, name);

            Some(CompositeFontFamilyKey::new(source, keys))
        };

        let inner = FontFamilyInner { family_names, key, automatic_optical_sizing: false };
        (FontFamily(Rc::new(inner)), uses_base_uri)
    }

    /// Represents the default font family.
    pub fn default_family() -> FontFamily {
        crate::ferro_thread_local! {
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

    /// Selects the OpenType optical size from the text size when the selected
    /// face and font backend support the `opsz` axis.
    pub fn with_automatic_optical_sizing(mut self) -> Self {
        if !self.0.automatic_optical_sizing {
            Rc::make_mut(&mut self.0).automatic_optical_sizing = true;
        }
        self
    }

    pub(crate) fn automatic_optical_sizing(&self) -> bool {
        self.0.automatic_optical_sizing
    }

    /// Whether both handles refer to the same instance (C# `ReferenceEquals`). Families made
    /// from the same text on a thread are the same instance.
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
                    } else if let Some(source) = Self::get_font_source(path) {
                        identifier = Some(FontSourceIdentifier::new(inner_name.to_owned(), Some(source)));
                    }
                }
            }

            // If we didn't manage to match it to any known format, treat the entire segment as the font name.
            result.push(identifier.unwrap_or_else(|| FontSourceIdentifier::new(segment.to_owned(), None)));
        }

        result
    }

    /// The source of a `source#name` segment: a relative reference when the
    /// path has a `/` (`/Assets/Fonts`, `Fonts/`), or a URL when it has a
    /// scheme and something after it (`fonts:Inter`, `resm:App.Fonts`).
    fn get_font_source(path: &str) -> Option<UriRef> {
        match UriRef::parse(path).ok()? {
            UriRef::Relative(_) if !path.contains('/') => None,
            // A scheme with nothing after it (`fonts:`) is not a source.
            UriRef::Absolute(_) if path.find(':') == Some(path.len() - 1) => None,
            source => Some(source),
        }
    }

    /// Parses a `FontFamily` string.
    pub fn parse(s: &str) -> Result<FontFamily, FormatError> {
        Self::parse_with_base_uri(s, None)
    }

    /// Parses a `FontFamily` string, resolving relative sources against `base_uri`.
    pub fn parse_with_base_uri(s: &str, base_uri: Option<&url::Url>) -> Result<FontFamily, FormatError> {
        if s.is_empty() {
            return Err(FormatError::new("Specified family is not supported."));
        }

        Ok(FontFamily::with_base_uri(base_uri, s))
    }
}

/// `<scheme>:<name>` (`systemfont:Arial`, `compositefont:Inter, Arial`): the
/// name is the opaque path of the URL. A name the URL parser would read as an
/// authority (`//host`) has its slashes percent-encoded, so that every name
/// has a URL; an empty name (a trailing comma) is the URL `systemfont:`, which
/// never matches a family.
fn named_font_url(scheme: &str, name: &str) -> url::Url {
    url::Url::parse(&format!("{scheme}:{name}"))
        .or_else(|_| url::Url::parse(&format!("{scheme}:{}", name.replace('/', "%2F"))))
        .expect("a scheme followed by a path without a slash is a URL")
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
        if self.ptr_eq(other) {
            return true;
        }

        if self.0.automatic_optical_sizing != other.0.automatic_optical_sizing || self.0.key != other.0.key {
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
        self.0.automatic_optical_sizing.hash(state);
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

    /// Not from upstream: a text is read once on a thread, and the families made from it later
    /// are the one made first. A source that is resolved against the base URI tells families of
    /// one text apart; a family that does not depend on a base URI is one whatever it was given.
    #[test]
    fn a_text_is_read_once_on_a_thread() {
        for s in NAMES {
            assert!(FontFamily::new(s).ptr_eq(&FontFamily::new(s)), "{s}");
        }
        let first = url::Url::parse("ferres://First/Views/Main.xaml").unwrap();
        let second = url::Url::parse("ferres://Second/Views/Main.xaml").unwrap();

        let plain = FontFamily::with_base_uri(Some(&first), "Font A");
        assert!(plain.ptr_eq(&FontFamily::with_base_uri(Some(&second), "Font A")));
        assert!(plain.ptr_eq(&FontFamily::new("Font A")));

        let relative = "/Assets/Fonts#MyFont";
        let of_first = FontFamily::with_base_uri(Some(&first), relative);
        let of_second = FontFamily::with_base_uri(Some(&second), relative);
        assert_ne!(of_first, of_second);
        assert!(of_first.ptr_eq(&FontFamily::with_base_uri(Some(&first), relative)));
        assert!(of_second.ptr_eq(&FontFamily::with_base_uri(Some(&second), relative)));
        // Without a base URI the family is not the one of either, and is equal to itself.
        let without = FontFamily::new(relative);
        assert!(without != of_first && without != of_second);
        assert_eq!(without, FontFamily::new(relative));
    }

    /// Not from upstream: text made up at run time does not grow the table of a thread without
    /// bound, and a family is the same value whether it comes from the table or not.
    #[test]
    fn the_families_a_thread_keeps_are_bounded() {
        let first = FontFamily::new("Kept 0");
        for index in 0..3 * Made::CAPACITY {
            let name = format!("Kept {index}");
            assert_eq!(FontFamily::new(&name).name(), name);
        }
        assert!(MADE.with(|made| made.borrow().count) <= Made::CAPACITY);
        // Read again after the table was emptied: another instance, the same family.
        let again = FontFamily::new("Kept 0");
        assert!(!again.ptr_eq(&first) && again == first && hash_of(&again) == hash_of(&first));
        // A text beyond what the table keeps is read each time.
        let long = "Long ".repeat(Made::MAX_TEXT);
        assert!(!FontFamily::new(&long).ptr_eq(&FontFamily::new(&long)));
        assert_eq!(FontFamily::new(&long), FontFamily::new(&long));
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

    fn url(text: &str) -> url::Url {
        UriRef::parse_absolute(text).unwrap()
    }

    #[test]
    fn should_parse_font_family_with_resource_folder() {
        let source = "resm:FerroUI.Visuals.UnitTests#MyFont";

        let key = FontFamilyKey::new(UriRef::parse(source).unwrap());

        let font_family = FontFamily::parse(source).unwrap();

        assert_eq!(font_family.name(), "MyFont");
        assert_eq!(font_family.key(), Some(&key));
    }

    #[test]
    fn should_parse_font_family_with_resource_filename() {
        let source = "resm:FerroUI.Visuals.UnitTests.MyFont.ttf#MyFont";

        let key = FontFamilyKey::new(UriRef::parse(source).unwrap());

        let font_family = FontFamily::parse(source).unwrap();

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
            let base_uri = base.map(url);

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
            let b = base_uri.map(url);
            let expected_uri = expected_uri.map(url);

            let font_family = FontFamily::parse_with_base_uri(s, b.as_ref()).unwrap();

            assert_eq!(font_family.name(), expected_name);

            if let Some(expected_uri) = expected_uri {
                let key = font_family.key().expect("the family has a key");

                if key.base_uri().is_none() {
                    assert!(!key.source().is_relative());
                }

                assert_eq!(key.absolute_source(), Some(&expected_uri));
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
            let base_uri = url(base_uri_string);

            let font_family = FontFamily::parse_with_base_uri(path, Some(&base_uri)).unwrap();

            let key = font_family.key().expect("the family has a key");
            let actual = key.source().resolve(key.base_uri().expect("the key has a base uri")).unwrap();

            assert_eq!(actual, url(expected));
            assert_eq!(key.absolute_source(), Some(&actual));
        }
    }

    // The base URI is a `Url`, so a relative base URI cannot be passed (the reference test also
    // rejects one).
    #[test]
    fn parse_rejects_an_empty_name() {
        assert!(FontFamily::parse("").is_err());
        assert!(FontFamily::parse_with_base_uri("", Some(&url("ferres://app/"))).is_err());
    }

    #[test]
    fn sources_are_references_and_names_are_urls() {
        let single = FontFamily::new("/Assets/Fonts#Inter");
        assert_eq!(single.key().map(|key| key.source().to_string()), Some("/Assets/Fonts".to_owned()));

        // A path without a `/` and without a scheme, or a scheme with nothing after it, is not
        // a source: the whole segment is the name.
        assert!(FontFamily::new("Fonts#Inter").key().is_none());
        assert_eq!(FontFamily::new("Fonts#Inter").name(), "Fonts#Inter");
        assert!(FontFamily::new("fonts:#Inter").key().is_none());

        let composite = FontFamily::with_base_uri(Some(&url("ferres://app/Main.xaml")), "Inter, /Assets#Noto");
        let keys = CompositeFontFamilyKey::keys(composite.key().unwrap()).unwrap();
        assert_eq!(keys[0].source().to_string(), "systemfont:Inter");
        assert_eq!(keys[1].absolute_source().map(url::Url::as_str), Some("ferres://app/Assets"));
        assert_eq!(composite.key().unwrap().source().to_string(), "compositefont:Inter, /Assets#Noto");
        assert_eq!(named_font_url(FontManager::SYSTEM_FONT_SCHEME, "//a b").as_str(), "systemfont:%2F%2Fa b");
    }
}
