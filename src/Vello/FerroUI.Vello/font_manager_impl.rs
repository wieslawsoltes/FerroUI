use crate::helpers::script_tag::script_tag;
use crate::vello_typeface::VelloTypeface;
use ferroui_base::media::text_formatting::unicode::{Codepoint, Script};
use ferroui_base::media::{FontSimulations, FontStretch, FontStyle, FontWeight, IPlatformTypeface, Typeface};
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::utilities::CultureInfo;
use fontique::{
    Collection, CollectionOptions, FallbackKey, FamilyId, FamilyInfo, FontInfo, GenericFamily, Language, SourceCache,
    SourceCacheOptions,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Read;
use std::rc::Rc;
use std::sync::Mutex;

/// The fonts of the system as they were last enumerated, for every font
/// manager of the process: enumerating them reads every font file of the
/// system, and a font manager is created for every application (and for
/// every test). A font manager works on a copy, which shares the families
/// and reads the data of a font when it is first asked for.
static ENUMERATED: Mutex<Option<Collection>> = Mutex::new(None);

/// The enumerated fonts of the system; `refresh` enumerates them again.
fn enumerated_collection(refresh: bool) -> Collection {
    let mut enumerated = ENUMERATED.lock().unwrap_or_else(|e| e.into_inner());

    if refresh {
        *enumerated = None;
    }

    enumerated
        .get_or_insert_with(|| Collection::new(CollectionOptions { shared: false, system_fonts: true }))
        .clone()
}
/// The families a character is looked for in before every installed family
/// is searched: the fonts an interface is set in, then the ones for emoji
/// and for mathematics.
const PREFERRED_FALLBACKS: [GenericFamily; 5] = [
    GenericFamily::SystemUi,
    GenericFamily::SansSerif,
    GenericFamily::Serif,
    GenericFamily::Emoji,
    GenericFamily::Math,
];

/// The fonts of the system and what was found in them.
struct SystemFonts {
    collection: Collection,
    source_cache: SourceCache,
    /// The family that has a character, by the character and the language
    /// it was asked for: the search of every installed family is done once.
    fallbacks: HashMap<(u32, String), Option<FamilyId>>,
}

impl SystemFonts {
    fn new(refresh: bool) -> Self {
        Self {
            collection: enumerated_collection(refresh),
            source_cache: SourceCache::new(SourceCacheOptions::default()),
            fallbacks: HashMap::new(),
        }
    }

    /// The font of a family that matches the attributes best, its data, and
    /// what has to be synthesized for the attributes it does not have.
    fn match_font(
        &mut self,
        family: &FamilyInfo,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<MatchedFont> {
        let (width, slant, boldness) = (to_width(stretch), to_style(style), fontique::FontWeight::new(weight.0 as f32));
        let font = family.match_font(width, slant, boldness, true)?;
        let data = font.load(Some(&mut self.source_cache))?;

        Some(MatchedFont { font: font.clone(), data, synthesis: font.synthesis(width, slant, boldness) })
    }

    /// Whether the font of a family that matches the attributes has a glyph
    /// for a character.
    fn family_has_character(
        &mut self,
        family: FamilyId,
        codepoint: u32,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> bool {
        let Some(family) = self.collection.family(family) else {
            return false;
        };

        self.match_font(&family, style, weight, stretch).is_some_and(|matched| matched.has_character(codepoint))
    }
}

/// A font of the system that was matched.
struct MatchedFont {
    font: FontInfo,
    data: fontique::Blob<u8>,
    synthesis: fontique::Synthesis,
}

impl MatchedFont {
    fn has_character(&self, codepoint: u32) -> bool {
        self.font
            .charmap_index()
            .charmap(self.data.data())
            .and_then(|charmap| charmap.map(codepoint))
            .is_some_and(|glyph| glyph != 0)
    }

    /// The typeface of the font, at the position in the variation space
    /// that the attributes it was matched for ask of a variable font.
    fn create_typeface(&self, font_simulations: FontSimulations) -> Option<Rc<VelloTypeface>> {
        let tags: Vec<[u8; 4]> = self.synthesis.variation_settings().iter().map(|(tag, _)| tag.to_be_bytes()).collect();
        let settings: Vec<(&str, f32)> = tags
            .iter()
            .zip(self.synthesis.variation_settings())
            .filter_map(|(tag, (_, value))| Some((std::str::from_utf8(tag).ok()?, *value)))
            .collect();

        VelloTypeface::new(self.data.clone(), self.font.index(), font_simulations, &settings)
    }
}

fn to_width(stretch: FontStretch) -> fontique::FontWidth {
    const RATIOS: [f32; 9] = [0.5, 0.625, 0.75, 0.875, 1.0, 1.125, 1.25, 1.5, 2.0];

    fontique::FontWidth::from_ratio(RATIOS[(stretch as i32 - 1).clamp(0, 8) as usize])
}

fn to_style(style: FontStyle) -> fontique::FontStyle {
    match style {
        FontStyle::Normal => fontique::FontStyle::Normal,
        FontStyle::Italic => fontique::FontStyle::Italic,
        FontStyle::Oblique => fontique::FontStyle::Oblique(None),
    }
}

fn from_style(style: fontique::FontStyle) -> FontStyle {
    match style {
        fontique::FontStyle::Normal => FontStyle::Normal,
        fontique::FontStyle::Italic => FontStyle::Italic,
        fontique::FontStyle::Oblique(_) => FontStyle::Oblique,
    }
}

fn from_width(width: fontique::FontWidth) -> FontStretch {
    const RATIOS: [f32; 9] = [0.5, 0.625, 0.75, 0.875, 1.0, 1.125, 1.25, 1.5, 2.0];

    let mut nearest = 4;
    for (index, ratio) in RATIOS.iter().enumerate() {
        if (ratio - width.ratio()).abs() < (RATIOS[nearest] - width.ratio()).abs() {
            nearest = index;
        }
    }

    FontStretch::from_i32(nearest as i32 + 1).unwrap_or(FontStretch::Normal)
}

/// The Vello font manager: the installed fonts of the system through
/// `fontique`, which enumerates them with the font interface of the
/// platform (CoreText, DirectWrite, fontconfig) and reads the font files
/// itself.
///
/// The Skia backend asks Skia's font manager; `fontique` gives the
/// families, the match of a family by weight, style and stretch and the
/// data of a font the same way, and a fallback family for a script and a
/// language where Skia gives one for a character: the script of the
/// character is looked up here, and a character without a script of its own
/// (a symbol, an emoji) or one the fallback family lacks is searched in the
/// installed families (design document, section 1.4).
pub struct FontManagerImpl {
    /// The fonts of the system, enumerated when they are first asked for.
    system_fonts: RefCell<Option<SystemFonts>>,
}

impl FontManagerImpl {
    /// Creates a font manager over the system's fonts.
    pub fn new() -> Self {
        Self { system_fonts: RefCell::new(None) }
    }

    fn system_fonts(&self) -> std::cell::RefMut<'_, SystemFonts> {
        std::cell::RefMut::map(self.system_fonts.borrow_mut(), |system_fonts| {
            system_fonts.get_or_insert_with(|| SystemFonts::new(false))
        })
    }

    /// The typeface of the font of an installed family that matches the
    /// attributes best, without simulations: what Skia's font manager
    /// answers to `matchFamilyStyle`.
    ///
    /// Returns `None` when no family of that name is installed.
    pub fn try_match_family_style(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<VelloTypeface>> {
        let mut system_fonts = self.system_fonts();

        let family = system_fonts.collection.family_by_name(family_name)?;

        system_fonts.match_font(&family, style, weight, stretch)?.create_typeface(FontSimulations::None)
    }

    /// The typeface of an installed family, or of the default family of
    /// the system when no family of that name is installed or none is
    /// named: what Skia's font manager answers to `legacyMakeTypeface`.
    pub fn legacy_make_typeface(
        &self,
        family_name: Option<&str>,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<VelloTypeface>> {
        family_name
            .and_then(|family_name| self.try_match_family_style(family_name, style, weight, stretch))
            .or_else(|| self.try_match_family_style(&self.get_default_font_family_name(), style, weight, stretch))
    }

    /// The family of the system that has a glyph for a character.
    fn find_fallback_family(
        system_fonts: &mut SystemFonts,
        codepoint: u32,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        culture: &CultureInfo,
    ) -> Option<FamilyId> {
        let key = (codepoint, culture.name().to_owned());

        if let Some(family) = system_fonts.fallbacks.get(&key) {
            return *family;
        }

        let has_character = |system_fonts: &mut SystemFonts, family: FamilyId| {
            system_fonts.family_has_character(family, codepoint, style, weight, stretch)
        };

        // What the system falls back to for the script of the character in
        // the language of the culture.
        let script = Codepoint::new(codepoint).script();
        let mut candidates: Vec<FamilyId> = Vec::new();

        if !matches!(script, Script::Unknown | Script::Common | Script::Inherited) {
            let language = Language::parse(culture.name()).ok();
            let fallback_key = FallbackKey::new(fontique::Script::from_bytes(script_tag(script)), language.as_ref());

            candidates.extend(system_fonts.collection.fallback_families(fallback_key));
        }

        for generic in PREFERRED_FALLBACKS {
            candidates.extend(system_fonts.collection.generic_families(generic));
        }

        let mut found = candidates.into_iter().find(|family| has_character(system_fonts, *family));

        // Every installed family, in the order of their names.
        if found.is_none() {
            let mut families: Vec<(String, FamilyId)> = system_fonts
                .collection
                .family_names()
                .map(str::to_owned)
                .collect::<Vec<_>>()
                .into_iter()
                .filter_map(|name| Some((name.clone(), system_fonts.collection.family_id(&name)?)))
                .collect();
            families.sort();

            // The last resort font has a glyph for every character and is
            // tried after the fonts that have its own.
            families.sort_by_key(|(name, _)| name.contains("LastResort"));

            found = families.into_iter().map(|(_, family)| family).find(|family| has_character(system_fonts, *family));
        }

        system_fonts.fallbacks.insert(key, found);
        found
    }
}

impl Default for FontManagerImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl IFontManagerImpl for FontManagerImpl {
    fn get_default_font_family_name(&self) -> String {
        let mut system_fonts = self.system_fonts();
        let system_fonts = &mut *system_fonts;

        // The family the system sets plain text in: Skia's default typeface
        // on the platforms the port runs on is its sans-serif family.
        let families: Vec<FamilyId> = [GenericFamily::SansSerif, GenericFamily::SystemUi]
            .into_iter()
            .flat_map(|generic| system_fonts.collection.generic_families(generic).collect::<Vec<_>>())
            .collect();

        families
            .into_iter()
            .find_map(|family| system_fonts.collection.family_name(family).map(str::to_owned))
            .or_else(|| {
                let mut names: Vec<String> = system_fonts.collection.family_names().map(str::to_owned).collect();
                names.sort();
                names.into_iter().find(|name| !name.starts_with('.'))
            })
            .unwrap_or_default()
    }

    fn get_installed_font_family_names(&self, check_for_updates: bool) -> Vec<String> {
        if check_for_updates {
            *self.system_fonts.borrow_mut() = Some(SystemFonts::new(true));
        }

        // A family of which the name begins with a dot is one the system
        // keeps for itself (its interface font): it is matched by name, and
        // not listed, as the system does not list it.
        let mut names: Vec<String> = self
            .system_fonts()
            .collection
            .family_names()
            .filter(|name| !name.starts_with('.'))
            .map(str::to_owned)
            .collect();

        names.sort();
        names.dedup();
        names
    }

    fn try_match_character(
        &self,
        codepoint: i32,
        font_style: FontStyle,
        font_weight: FontWeight,
        font_stretch: FontStretch,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let codepoint = u32::try_from(codepoint).ok()?;

        let current_culture;
        let culture = match culture {
            Some(culture) => culture,
            None => {
                current_culture = CultureInfo::current_ui_culture();
                &current_culture
            }
        };

        let mut system_fonts = self.system_fonts();
        let system_fonts = &mut *system_fonts;

        // The family that was asked for, when it has the character.
        let named = family_name
            .filter(|name| !name.is_empty())
            .and_then(|name| system_fonts.collection.family_id(name))
            .filter(|family| {
                system_fonts.family_has_character(*family, codepoint, font_style, font_weight, font_stretch)
            });

        let family = match named {
            Some(family) => family,
            None => Self::find_fallback_family(
                system_fonts,
                codepoint,
                font_style,
                font_weight,
                font_stretch,
                culture,
            )?,
        };

        let family = system_fonts.collection.family(family)?;
        let matched = system_fonts.match_font(&family, font_style, font_weight, font_stretch)?;

        Some(matched.create_typeface(FontSimulations::None)? as Rc<dyn IPlatformTypeface>)
    }

    fn try_create_glyph_typeface(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let mut system_fonts = self.system_fonts();

        let family = system_fonts.collection.family_by_name(family_name)?;
        let matched = system_fonts.match_font(&family, style, weight, stretch)?;

        // As the Skia backend decides it: a weight from semi-bold on a font
        // that is not that heavy is emboldened, an italic style on an
        // upright font is slanted. A variable font is set to the weight or
        // the slant on its axis instead.
        let varies = |tag: &[u8; 4]| {
            matched.synthesis.variation_settings().iter().any(|(axis, _)| axis.to_be_bytes() == *tag)
        };

        let mut font_simulations = FontSimulations::None;

        if weight.0 >= 600 && matched.font.weight().value() < 600.0 && !varies(b"wght") {
            font_simulations |= FontSimulations::Bold;
        }

        if style == FontStyle::Italic
            && matched.font.style() == fontique::FontStyle::Normal
            && !varies(b"ital")
            && !varies(b"slnt")
        {
            font_simulations |= FontSimulations::Oblique;
        }

        Some(matched.create_typeface(font_simulations)? as Rc<dyn IPlatformTypeface>)
    }

    fn try_create_glyph_typeface_from_stream(
        &self,
        stream: &mut dyn Read,
        font_simulations: FontSimulations,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).ok()?;

        Some(VelloTypeface::from_bytes(bytes, font_simulations)? as Rc<dyn IPlatformTypeface>)
    }

    fn try_get_family_typefaces(&self, family_name: &str) -> Option<Vec<Typeface>> {
        let family = self.system_fonts().collection.family_by_name(family_name)?;

        if family.fonts().is_empty() {
            return None;
        }

        Some(
            family
                .fonts()
                .iter()
                .map(|font| {
                    Typeface::from_name_with_style(
                        family_name,
                        from_style(font.style()),
                        FontWeight(font.weight().value().round() as i32),
                        from_width(font.width()),
                    )
                })
                .collect(),
        )
    }
}
