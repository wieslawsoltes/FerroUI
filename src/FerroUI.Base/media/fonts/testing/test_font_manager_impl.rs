use std::cell::{Cell, RefCell};
use std::io::Read;
use std::rc::Rc;

use crate::media::fonts::font_collection_base::equals_ordinal_ignore_case;
use crate::media::{FontFamily, FontSimulations, FontStretch, FontStyle, FontWeight, IPlatformTypeface, Typeface};
use crate::platform::IFontManagerImpl;
use crate::utilities::CultureInfo;

use super::{TestFontBuilder, TestPlatformTypeface};

/// A font backend serving a configurable set of in-memory fonts.
///
/// - Installed family names: the distinct family names of the fonts, in the
///   order they were added (or the names given to
///   [`set_installed_font_family_names`](Self::set_installed_font_family_names)).
/// - Creating a typeface by name gives the font of the family with exactly
///   the requested style, weight and stretch, or else the first font of the
///   family. Aliases resolve to their family first.
/// - Matching a character gives a font covering the codepoint: of the hinted
///   family with the requested style if there is one, then of the hinted
///   family, then with the requested style, then the first one.
/// - Creating a typeface from a stream parses the bytes
///   ([`TestPlatformTypeface::from_stream`]).
pub struct TestFontManagerImpl {
    default_family_name: String,
    fonts: RefCell<Vec<Rc<TestPlatformTypeface>>>,
    aliases: RefCell<Vec<(String, String)>>,
    installed_font_family_names: RefCell<Option<Vec<String>>>,
    stream_typeface_creations: Cell<usize>,
    create_typeface_requests: RefCell<Vec<String>>,
    match_character_calls: Cell<usize>,
    is_disposed: Cell<bool>,
}

#[allow(dead_code)] // the full harness API is kept for the text tests built on top
impl TestFontManagerImpl {
    /// A backend without fonts whose default font family is `default_family_name`.
    pub fn new(default_family_name: &str) -> Rc<TestFontManagerImpl> {
        Rc::new(TestFontManagerImpl {
            default_family_name: default_family_name.to_owned(),
            fonts: RefCell::new(Vec::new()),
            aliases: RefCell::new(Vec::new()),
            installed_font_family_names: RefCell::new(None),
            stream_typeface_creations: Cell::new(0),
            create_typeface_requests: RefCell::new(Vec::new()),
            match_character_calls: Cell::new(0),
            is_disposed: Cell::new(false),
        })
    }

    /// The default font set, modelled on the fonts of the upstream headless
    /// test backend (in this order):
    ///
    /// - `Twitter Color Emoji`: U+1F300 to U+1F64F and U+2600 to U+27BF, advance 1000;
    /// - `Noto Sans` (italic): U+0020 to U+007E, U+00A0 to U+024F, U+0370 to U+03FF
    ///   (Greek), U+0400 to U+04FF (Cyrillic), advance 500;
    /// - `Noto Mono` (the default family, fixed pitch): U+0020 to U+007E and
    ///   U+00A0 to U+00FF, advance 600.
    ///
    /// All of them: 1000 units per em, ascender 800, descender -200, line gap 0.
    pub fn headless() -> Rc<TestFontManagerImpl> {
        let font_manager = Self::new("Noto Mono");

        font_manager.add_font(
            TestFontBuilder::new("Twitter Color Emoji")
                .codepoints(&[(0x1F300, 0x1F64F), (0x2600, 0x27BF)])
                .advance(1000)
                .build(),
        );
        font_manager.add_font(
            TestFontBuilder::new("Noto Sans")
                .style(FontStyle::Italic)
                .codepoints(&[(0x20, 0x7E), (0xA0, 0x24F), (0x370, 0x3FF), (0x400, 0x4FF)])
                .advance(500)
                .build(),
        );
        font_manager.add_font(
            TestFontBuilder::new("Noto Mono").codepoints(&[(0x20, 0x7E), (0xA0, 0xFF)]).fixed_pitch(true).build(),
        );

        font_manager
    }

    /// Adds a font to the set.
    pub fn add_font(&self, font: Rc<TestPlatformTypeface>) {
        self.fonts.borrow_mut().push(font);
    }

    /// Makes `alias` resolve to `family_name` when a typeface is created by
    /// name. An alias is not an installed family name.
    pub fn add_alias(&self, alias: &str, family_name: &str) {
        self.aliases.borrow_mut().push((alias.to_owned(), family_name.to_owned()));
    }

    /// Overrides the installed family names reported by the backend
    /// (`None` reports the families of the fonts).
    pub fn set_installed_font_family_names(&self, names: Option<Vec<String>>) {
        *self.installed_font_family_names.borrow_mut() = names;
    }

    /// The fonts of the set.
    pub fn fonts(&self) -> Vec<Rc<TestPlatformTypeface>> {
        self.fonts.borrow().clone()
    }

    /// The number of typefaces created from a stream.
    pub fn stream_typeface_creations(&self) -> usize {
        self.stream_typeface_creations.get()
    }

    /// The number of requests to create a typeface by family name.
    pub fn create_typeface_calls(&self) -> usize {
        self.create_typeface_requests.borrow().len()
    }

    /// The number of requests to create a typeface of the family (ignoring case).
    pub fn create_typeface_calls_for(&self, family_name: &str) -> usize {
        self.create_typeface_requests
            .borrow()
            .iter()
            .filter(|requested| equals_ordinal_ignore_case(requested, family_name))
            .count()
    }

    /// The number of character match requests.
    pub fn match_character_calls(&self) -> usize {
        self.match_character_calls.get()
    }

    /// Whether the backend was disposed.
    pub fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }

    fn resolve_alias(&self, family_name: &str) -> String {
        self.aliases
            .borrow()
            .iter()
            .find(|(alias, _)| equals_ordinal_ignore_case(alias, family_name))
            .map_or_else(|| family_name.to_owned(), |(_, family_name)| family_name.clone())
    }
}

impl IFontManagerImpl for TestFontManagerImpl {
    fn get_default_font_family_name(&self) -> String {
        self.default_family_name.clone()
    }

    fn get_installed_font_family_names(&self, _check_for_updates: bool) -> Vec<String> {
        if let Some(names) = self.installed_font_family_names.borrow().as_ref() {
            return names.clone();
        }

        let mut names: Vec<String> = Vec::new();

        for font in self.fonts.borrow().iter() {
            let family_name = font.family_name();

            if !names.contains(&family_name) {
                names.push(family_name);
            }
        }

        names
    }

    fn try_match_character(
        &self,
        codepoint: i32,
        font_style: FontStyle,
        font_weight: FontWeight,
        font_stretch: FontStretch,
        family_name: Option<&str>,
        _culture: Option<&CultureInfo>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        self.match_character_calls.set(self.match_character_calls.get() + 1);

        let fonts = self.fonts.borrow();

        let mut best: Option<(&Rc<TestPlatformTypeface>, i32)> = None;

        for font in fonts.iter().filter(|font| font.covers(codepoint)) {
            let is_family =
                family_name.is_some_and(|family_name| equals_ordinal_ignore_case(&font.family_name(), family_name));
            let is_exact =
                font.style() == font_style && font.weight() == font_weight && font.stretch() == font_stretch;

            let score = i32::from(is_family) * 2 + i32::from(is_exact);

            if best.is_none_or(|(_, best_score)| score > best_score) {
                best = Some((font, score));
            }
        }

        best.map(|(font, _)| font.clone() as Rc<dyn IPlatformTypeface>)
    }

    fn try_create_glyph_typeface(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        self.create_typeface_requests.borrow_mut().push(family_name.to_owned());

        let family_name = self.resolve_alias(family_name);

        let mut family_match: Option<Rc<TestPlatformTypeface>> = None;

        // Search for a matching family name and style
        for font in self.fonts.borrow().iter() {
            if !equals_ordinal_ignore_case(&font.family_name(), &family_name) {
                continue;
            }

            // Exact match - return immediately
            if font.style() == style && font.weight() == weight && font.stretch() == stretch {
                return Some(font.clone());
            }

            // If family matches but style doesn't, keep searching
            // but remember first family match as fallback
            if family_match.is_none() {
                family_match = Some(font.clone());
            }
        }

        family_match.map(|font| font as Rc<dyn IPlatformTypeface>)
    }

    fn try_create_glyph_typeface_from_stream(
        &self,
        stream: &mut dyn Read,
        font_simulations: FontSimulations,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        self.stream_typeface_creations.set(self.stream_typeface_creations.get() + 1);

        TestPlatformTypeface::from_stream(stream, font_simulations)
            .map(|typeface| typeface as Rc<dyn IPlatformTypeface>)
    }

    fn try_get_family_typefaces(&self, family_name: &str) -> Option<Vec<Typeface>> {
        // Find all typefaces that belong to the specified family
        let typefaces: Vec<Typeface> = self
            .fonts
            .borrow()
            .iter()
            .filter(|font| equals_ordinal_ignore_case(&font.family_name(), family_name))
            .map(|font| {
                Typeface::with_style(FontFamily::new(&font.family_name()), font.style(), font.weight(), font.stretch())
            })
            .collect();

        if typefaces.is_empty() {
            None
        } else {
            Some(typefaces)
        }
    }

    fn dispose(&self) {
        self.is_disposed.set(true);
    }
}
