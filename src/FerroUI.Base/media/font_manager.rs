use std::borrow::Cow;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::media::composite_font_family_key::CompositeFontFamilyKey;
use crate::media::font_fallback_mapping::FontFallbackMappingResult;
use crate::media::fonts::font_collection_base::compare_ordinal_ignore_case;
use crate::media::fonts::{
    EmbeddedFontCollection, EmptySystemFontCollection, FontCollectionBase, FontFamilyKey, IFontCollection,
    SystemFontCollection,
};
use crate::media::text_formatting::unicode::Script;
use crate::media::text_formatting::TextLayoutCache;
use crate::media::{
    FontFallback, FontFamily, FontManagerOptions, FontStretch, FontStyle, FontWeight, GlyphTypeface, Typeface,
};
use crate::platform::IFontManagerImpl;
use crate::utilities::{Locale, UriExtensions, UriRef};
use crate::{FerroLocator, LocatorExtensions};

/// The font manager is used to query the system's installed fonts and is
/// responsible for caching loaded fonts. It is also responsible for the font
/// fallback.
pub struct FontManager {
    /// The collections by their key, a URL without its fragment (decision O5 of
    /// docs/idiomatic-rust-signatures.md: `fonts:A#x` is the collection `fonts:A`, as it was
    /// for the `Uri` of .NET).
    font_collections: RefCell<HashMap<url::Url, Rc<dyn IFontCollection>>>,
    /// The key of the system font collection, made once.
    system_fonts_key: url::Url,
    font_fallbacks: Option<Vec<FontFallback>>,
    /// The configured family name mappings, sorted by name ignoring case.
    font_family_mappings: Option<Vec<(String, FontFamily)>>,
    platform_impl: Rc<dyn IFontManagerImpl>,
    default_font_family: OnceCell<FontFamily>,
    /// The glyph typefaces [`try_get_glyph_typeface`](Self::try_get_glyph_typeface) found, by
    /// typeface, valid while the thread's glyph typeface generation is `resolved_generation`
    /// (FerroUI deviation, docs/porting/desktop-performance.md, section 9.4).
    resolved_glyph_typefaces: RefCell<HashMap<Typeface, Rc<GlyphTypeface>>>,
    resolved_generation: Cell<u64>,
    /// The layouts of plain text made with this font manager, by what they were made of
    /// (FerroUI addition, docs/porting/desktop-performance.md, section 9.29).
    text_layouts: TextLayoutCache,
}

crate::ferro_thread_local! {
    static GLYPH_TYPEFACE_GENERATION: Cell<u64> = const { Cell::new(1) };
}

/// Makes every font manager of the thread look its glyph typefaces up again: a font collection's
/// set of glyph typefaces or the set of collections changed.
pub(crate) fn advance_glyph_typeface_generation() {
    GLYPH_TYPEFACE_GENERATION.with(|generation| generation.set(generation.get().wrapping_add(1)));
}

/// The glyph typeface generation of the thread: what was found in a font collection holds while
/// it stays the same.
pub(crate) fn glyph_typeface_generation() -> u64 {
    GLYPH_TYPEFACE_GENERATION.with(Cell::get)
}

/// Font managers compare by reference, so that a handle can be an untyped
/// value (the value of the static member markup names).
impl PartialEq for FontManager {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl FontManager {
    pub const FONT_COLLECTION_SCHEME: &'static str = "fonts";
    pub const SYSTEM_FONT_SCHEME: &'static str = "systemfont";
    pub const COMPOSITE_FONT_SCHEME: &'static str = "compositefont";

    /// The text of the key of the system font collection.
    const SYSTEM_FONTS_KEY: &'static str = "fonts:SystemFonts";

    /// The key of the system font collection (`fonts:SystemFonts`).
    ///
    /// Internal upstream, where the Skia unit tests see it; public here so
    /// that the tests of the Skia crate reach it.
    pub fn system_fonts_key() -> url::Url {
        url::Url::parse(Self::SYSTEM_FONTS_KEY).expect("the system fonts key is a URL")
    }

    /// Creates a font manager on top of a font backend. The
    /// [`FontManagerOptions`] registered with the locator, if any, are applied.
    ///
    /// Panics when neither the options nor the backend nor the installed
    /// fonts give a default font family name.
    pub fn new(platform_impl: Rc<dyn IFontManagerImpl>) -> Rc<FontManager> {
        let options = FerroLocator::current().get_service::<FontManagerOptions>();

        let font_manager = FontManager {
            font_collections: RefCell::new(HashMap::new()),
            system_fonts_key: Self::system_fonts_key(),
            font_fallbacks: options.as_ref().and_then(|options| options.font_fallbacks.clone()),
            font_family_mappings: Self::create_font_family_mappings(
                options.as_ref().and_then(|options| options.font_family_mappings.as_ref()),
            ),
            platform_impl,
            default_font_family: OnceCell::new(),
            resolved_glyph_typefaces: RefCell::new(HashMap::new()),
            resolved_generation: Cell::new(0),
            text_layouts: TextLayoutCache::new(),
        };

        let default_font_family_name = font_manager.get_default_font_family_name(options.as_deref());

        font_manager
            .default_font_family
            .get_or_init(|| FontFamily::new(&default_font_family_name));

        Rc::new(font_manager)
    }

    /// Copies the configured mappings into a case-insensitive lookup. Family
    /// names are matched case-insensitively everywhere else, so a mapping
    /// must apply whatever casing the requested name arrives in.
    fn create_font_family_mappings(
        font_family_mappings: Option<&HashMap<String, FontFamily>>,
    ) -> Option<Vec<(String, FontFamily)>> {
        let font_family_mappings = font_family_mappings.filter(|mappings| !mappings.is_empty())?;

        // Names that collide only by casing were distinct entries before the copy. One of them
        // is taken rather than failing: a mapping table is configuration, and failing here would
        // bring the application down at startup. The entries are visited in name order so that
        // the choice (the last one) is deterministic.
        let mut entries: Vec<(&String, &FontFamily)> = font_family_mappings.iter().collect();

        entries.sort_by(|a, b| a.0.cmp(b.0));

        let mut mappings: Vec<(String, FontFamily)> = Vec::with_capacity(entries.len());

        for (name, font_family) in entries {
            match mappings.binary_search_by(|(existing, _)| compare_ordinal_ignore_case(existing, name)) {
                Ok(index) => mappings[index].1 = font_family.clone(),
                Err(index) => mappings.insert(index, (name.clone(), font_family.clone())),
            }
        }

        Some(mappings)
    }

    fn try_get_font_family_mapping(&self, family_name: &str) -> Option<&FontFamily> {
        let mappings = self.font_family_mappings.as_ref()?;

        mappings
            .binary_search_by(|(existing, _)| compare_ordinal_ignore_case(existing, family_name))
            .ok()
            .map(|index| &mappings[index].1)
    }

    /// Get the current font manager instance.
    ///
    /// The font manager registered with the locator, or a new one created on
    /// top of the registered font backend (and then registered).
    ///
    /// Panics when no font backend (`IFontManagerImpl`) is registered.
    pub fn current() -> Rc<FontManager> {
        if let Some(current) = FerroLocator::current().get_service::<FontManager>() {
            return current;
        }

        let font_manager_impl = FerroLocator::current().get_required_service::<dyn IFontManagerImpl>();

        let current = FontManager::new(font_manager_impl);

        FerroLocator::current_mutable().bind::<FontManager>().to_constant(current.clone());

        current
    }

    /// The layouts of plain text made with this font manager.
    pub(crate) fn text_layouts(&self) -> &TextLayoutCache {
        &self.text_layouts
    }

    /// Gets the system's default font family.
    pub fn default_font_family(&self) -> &FontFamily {
        self.default_font_family.get().expect("the default font family is set by the constructor")
    }

    /// Get all system fonts.
    pub fn system_fonts(&self) -> Rc<dyn IFontCollection> {
        if let Some(font_collection) = self.try_get_font_collection(&self.system_fonts_key) {
            return font_collection;
        }

        // Fallback to an empty system font collection
        Rc::new(EmptySystemFontCollection::new())
    }

    /// The font backend.
    ///
    /// Internal upstream, where the Skia unit tests see it; public here so
    /// that the tests of the Skia crate reach it.
    pub fn platform_impl(&self) -> &Rc<dyn IFontManagerImpl> {
        &self.platform_impl
    }

    /// Tries to get a glyph typeface for specified typeface.
    ///
    /// Returns the glyph typeface if the font manager could create it,
    /// `None` otherwise.
    ///
    /// The glyph typeface found for a typeface is kept until a font collection gains a glyph
    /// typeface or a collection is added or removed; a collection that does not derive from
    /// [`FontCollectionBase`](crate::media::fonts::FontCollectionBase) and changes what it returns
    /// calls [`invalidate_glyph_typefaces`](Self::invalidate_glyph_typefaces).
    pub fn try_get_glyph_typeface(&self, typeface: &Typeface) -> Option<Rc<GlyphTypeface>> {
        let generation = GLYPH_TYPEFACE_GENERATION.with(Cell::get);
        if self.resolved_generation.get() == generation {
            if let Some(found) = self.resolved_glyph_typefaces.borrow().get(typeface) {
                return Some(found.clone());
            }
        } else {
            self.resolved_glyph_typefaces.borrow_mut().clear();
            self.resolved_generation.set(generation);
        }
        let found = self.resolve_glyph_typeface(typeface)?;
        // The generation of the lookup: a collection that gained a glyph typeface while it ran
        // made the result stale already.
        if self.resolved_generation.get() == GLYPH_TYPEFACE_GENERATION.with(Cell::get) {
            self.resolved_glyph_typefaces.borrow_mut().insert(typeface.clone(), found.clone());
        }
        Some(found)
    }

    /// Makes this font manager look every glyph typeface up again: for a font collection that
    /// does not derive from `FontCollectionBase` and whose glyph typefaces changed.
    pub fn invalidate_glyph_typefaces(&self) {
        advance_glyph_typeface_generation();
    }

    fn resolve_glyph_typeface(&self, typeface: &Typeface) -> Option<Rc<GlyphTypeface>> {
        let font_family = self
            .try_get_font_family_mapping(typeface.font_family().family_names().primary_family_name())
            .unwrap_or(typeface.font_family());

        if typeface.font_family().name() == FontFamily::DEFAULT_FONT_FAMILY_NAME {
            return self.try_get_glyph_typeface(&self.default_typeface(typeface));
        }

        if let Some(font_family_key) = font_family.key() {
            if let Some(composite_keys) = CompositeFontFamilyKey::keys(font_family_key) {
                for (i, composite_key) in composite_keys.iter().enumerate() {
                    let mut key = composite_key.clone();

                    let mut family_name = &font_family.family_names()[i];

                    if let Some(mapped_font_family) = self.try_get_font_family_mapping(family_name) {
                        key = match mapped_font_family.key() {
                            Some(mapped_key) => mapped_key.clone(),
                            None => FontFamilyKey::new(UriRef::from(self.system_fonts_key.clone())),
                        };

                        family_name = mapped_font_family.family_names().primary_family_name();
                    }

                    if family_name == FontFamily::DEFAULT_FONT_FAMILY_NAME {
                        return self.try_get_glyph_typeface(&self.default_typeface(typeface));
                    }

                    if let Some(glyph_typeface) = self.try_get_glyph_typeface_by_key_and_name(typeface, &key, family_name)
                    {
                        if glyph_typeface.family_name().contains(family_name) {
                            return Some(glyph_typeface);
                        }
                    }
                }
            } else {
                let family_name = font_family.family_names().primary_family_name();

                return self.try_get_glyph_typeface_by_key_and_name(typeface, font_family_key, family_name);
            }
        } else {
            let family_name = font_family.family_names().primary_family_name();

            if let Some(glyph_typeface) = self.system_fonts().try_get_glyph_typeface(
                family_name,
                typeface.style(),
                typeface.weight(),
                typeface.stretch(),
            ) {
                return Some(glyph_typeface);
            }
        }

        if typeface.font_family() == self.default_font_family() {
            return None;
        }

        // Nothing was found so use the default
        self.try_get_glyph_typeface(&self.default_typeface(typeface))
    }

    /// Queries only a provider's supported system-family mapping domain.
    /// Configured fallbacks and keyed/aliased families retain the framework
    /// resolver until a provider explicitly supports those collection rules.
    pub(crate) fn map_characters(
        &self,
        text: &[u16],
        typeface: &Typeface,
        culture: Option<&Locale>,
    ) -> FontFallbackMappingResult {
        if self.font_fallbacks.as_ref().is_some_and(|fallbacks| !fallbacks.is_empty()) {
            return FontFallbackMappingResult::Unavailable;
        }
        let requested = if typeface.font_family().name() == FontFamily::DEFAULT_FONT_FAMILY_NAME {
            self.default_typeface(typeface)
        } else {
            typeface.clone()
        };
        if requested.font_family().key().is_some()
            || self.try_get_font_family_mapping(requested.font_family().name()).is_some()
        {
            return FontFallbackMappingResult::Unavailable;
        }
        let Some(mapper) = self.platform_impl.font_fallback_mapper() else {
            return FontFallbackMappingResult::Unavailable;
        };
        // No collection/cache borrow crosses the provider callback. Its face
        // and scale belong to this mapped range, not the script fallback cache.
        match mapper.try_map_characters(text, &requested, culture) {
            Some(mapping) => FontFallbackMappingResult::Mapped(mapping),
            None => FontFallbackMappingResult::Fallback,
        }
    }

    /// The typeface of the default font family with the style of `typeface`.
    fn default_typeface(&self, typeface: &Typeface) -> Typeface {
        Typeface::with_style(
            self.default_font_family().clone(),
            typeface.style(),
            typeface.weight(),
            typeface.stretch(),
        )
    }

    fn try_get_glyph_typeface_by_key_and_name(
        &self,
        typeface: &Typeface,
        key: &FontFamilyKey,
        family_name: &str,
    ) -> Option<Rc<GlyphTypeface>> {
        let source = Self::absolute_source(key);

        let font_collection = self.try_get_font_collection(source)?;

        if let Some(glyph_typeface) =
            font_collection.try_get_glyph_typeface(family_name, typeface.style(), typeface.weight(), typeface.stretch())
        {
            return Some(glyph_typeface);
        }

        if let Some(logger) = Logger::try_get(LogEventLevel::Debug, LogArea::FONTS) {
            let present: Vec<String> =
                font_collection.font_families().iter().map(|font_family| font_family.to_string()).collect();

            logger.log(
                Some(self),
                &format!(
                    "Font family '{family_name}' could not be found. Present font families: [{}]",
                    present.join(",")
                ),
            );
        }

        None
    }

    /// Add a font collection to the manager.
    ///
    /// If a font collection's key is already present the collection is
    /// replaced (and the replaced collection disposed).
    ///
    /// Panics when the key of the collection does not follow the `fonts:` scheme.
    pub fn add_font_collection(&self, font_collection: Rc<dyn IFontCollection>) {
        let mut key = font_collection.key();

        if !UriExtensions::is_font_collection(&key) {
            panic!("Font collection Key should follow the fonts: scheme.");
        }

        key.set_fragment(None);

        let old_collection = self.font_collections.borrow_mut().insert(key, font_collection);
        advance_glyph_typeface_generation();

        if let Some(old_collection) = old_collection {
            old_collection.dispose();
        }
    }

    /// Removes the font collection that corresponds to specified key.
    pub fn remove_font_collection(&self, key: &url::Url) {
        let font_collection = self.font_collections.borrow_mut().remove(&*collection_key(key));
        advance_glyph_typeface_generation();

        if let Some(font_collection) = font_collection {
            font_collection.dispose();
        }
    }

    /// Tries to match a specified character to a [`Typeface`] that supports
    /// specified font properties.
    ///
    /// `font_family` is optional and used for fallback lookup.
    ///
    /// Returns the matching typeface if the font manager could match the
    /// character to specified parameters, `None` otherwise.
    pub fn try_match_character(
        &self,
        codepoint: i32,
        font_style: FontStyle,
        font_weight: FontWeight,
        font_stretch: FontStretch,
        font_family: Option<&FontFamily>,
        culture: Option<&Locale>,
    ) -> Option<Typeface> {
        self.try_match_character_with_script(
            codepoint,
            font_style,
            font_weight,
            font_stretch,
            font_family,
            culture,
            Script::Unknown,
        )
    }

    /// Character-to-typeface match with an optional shaping-capability
    /// constraint: when `shaping_script` is a complex script, only fonts that
    /// can shape it are considered. [`Script::Unknown`] imposes no constraint
    /// and is identical to the public overload.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn try_match_character_with_script(
        &self,
        codepoint: i32,
        font_style: FontStyle,
        font_weight: FontWeight,
        font_stretch: FontStretch,
        font_family: Option<&FontFamily>,
        culture: Option<&Locale>,
        shaping_script: Script,
    ) -> Option<Typeface> {
        if let Some(font_fallbacks) = &self.font_fallbacks {
            for fallback in font_fallbacks {
                if fallback.unicode_range.is_in_range(codepoint) {
                    let typeface =
                        Typeface::with_style(fallback.font_family.clone(), font_style, font_weight, font_stretch);

                    if let Some(glyph_typeface) = self.try_get_glyph_typeface(&typeface) {
                        if glyph_typeface.character_to_glyph_map().try_get_glyph(codepoint).is_some()
                            && (shaping_script == Script::Unknown || glyph_typeface.can_shape_script(shaping_script))
                        {
                            return Some(typeface);
                        }
                    }
                }
            }
        }

        // Try to match against fallbacks first
        if let Some(font_family) = font_family {
            if let Some(font_family_key) = font_family.key() {
                if let Some(composite_keys) = CompositeFontFamilyKey::keys(font_family_key) {
                    for (i, key) in composite_keys.iter().enumerate() {
                        let mut family_name = &font_family.family_names()[i];
                        let source = Self::absolute_source(key);

                        if family_name == FontFamily::DEFAULT_FONT_FAMILY_NAME {
                            family_name = self.default_font_family().name();
                        }

                        let Some(font_collection) = self.try_get_font_collection(source) else {
                            continue;
                        };

                        // With composite fonts we need to first check if the font collection contains the family if not we skip it
                        if font_collection
                            .try_get_glyph_typeface(family_name, font_style, font_weight, font_stretch)
                            .is_none()
                        {
                            continue;
                        }

                        if let Some(typeface) = Self::try_match_character_in_collection(
                            &*font_collection,
                            codepoint,
                            font_style,
                            font_weight,
                            font_stretch,
                            Some(family_name),
                            culture,
                            shaping_script,
                        ) {
                            if typeface.font_family().name() == self.default_font_family().name()
                                && i + 1 < composite_keys.len()
                            {
                                continue;
                            }

                            return Some(typeface);
                        }
                    }
                }

                let font_uri = Self::absolute_source(font_family_key);

                if UriExtensions::is_font_collection(font_uri) {
                    if let Some(font_collection) = self.try_get_font_collection(font_uri) {
                        if let Some(typeface) = Self::try_match_character_in_collection(
                            &*font_collection,
                            codepoint,
                            font_style,
                            font_weight,
                            font_stretch,
                            Some(font_family.name()),
                            culture,
                            shaping_script,
                        ) {
                            return Some(typeface);
                        }
                    }
                }
            }
        }

        // Try to find a match with the system font collection
        Self::try_match_character_in_collection(
            &*self.system_fonts(),
            codepoint,
            font_style,
            font_weight,
            font_stretch,
            font_family.map(|font_family| font_family.name()),
            culture,
            shaping_script,
        )
    }

    // Routes through the shaping-aware overload when the collection derives from the font
    // collection base (all built-in collections do); a foreign collection gets the
    // unconstrained match.
    #[allow(clippy::too_many_arguments)]
    fn try_match_character_in_collection(
        font_collection: &dyn IFontCollection,
        codepoint: i32,
        font_style: FontStyle,
        font_weight: FontWeight,
        font_stretch: FontStretch,
        family_name: Option<&str>,
        culture: Option<&Locale>,
        shaping_script: Script,
    ) -> Option<Typeface> {
        if let Some(font_collection_base) = font_collection.as_font_collection_base() {
            return FontCollectionBase::try_match_character_with_script(
                font_collection_base,
                codepoint,
                font_style,
                font_weight,
                font_stretch,
                family_name,
                culture,
                shaping_script,
            );
        }

        font_collection.try_match_character(codepoint, font_style, font_weight, font_stretch, family_name, culture)
    }

    /// The typefaces of a font family; empty when the family is unknown.
    ///
    /// Internal upstream, where the Skia unit tests see it; public here so
    /// that the tests of the Skia crate reach it.
    pub fn get_family_typefaces(&self, font_family: &FontFamily) -> Vec<Typeface> {
        match font_family.key() {
            None => self.system_fonts().try_get_family_typefaces(font_family.name()),
            Some(key) => {
                let source = Self::absolute_source(key);

                self.try_get_font_collection(source)
                    .and_then(|font_collection| font_collection.try_get_family_typefaces(font_family.name()))
            }
        }
        .unwrap_or_default()
    }

    /// The font collection of an absolute source URI: the system font
    /// collection (`systemfont:` or the system fonts key), a registered
    /// collection (`fonts:`) or the collection of embedded fonts at the
    /// source (`resm:`, `ferres:`), which is created on first use.
    pub(crate) fn try_get_font_collection(&self, source: &url::Url) -> Option<Rc<dyn IFontCollection>> {
        let source = collection_key(source);

        // Both the systemfont: scheme and the system fonts key (fonts:SystemFonts) map to the
        // system font collection. The key is checked before the generic font collection branch
        // so that the system font collection is created on demand regardless of which URI form
        // is used.
        if source.scheme() == Self::SYSTEM_FONT_SCHEME || source.as_str() == Self::SYSTEM_FONTS_KEY {
            return Some(self.get_or_create_font_collection(&self.system_fonts_key, |_| {
                Rc::new(SystemFontCollection::new(self.platform_impl.clone()))
            }));
        }

        // Other fonts: URIs are only returned when they have been explicitly registered
        // via `add_font_collection` - no implicit creation to avoid caching nothing for unknown keys.
        if UriExtensions::is_font_collection(&source) {
            return self.font_collections.borrow().get(&*source).cloned();
        }

        if UriExtensions::is_resm(&source) || UriExtensions::is_asset(&source) {
            return Some(self.get_or_create_font_collection(&source, |key| {
                Rc::new(EmbeddedFontCollection::new(key.clone(), key.clone()))
            }));
        }

        None
    }

    /// The URL of the source of a key (C# `key.Source.EnsureAbsolute(key.BaseUri)`), resolved
    /// when the key was made.
    ///
    /// Panics for a relative source without a base URI (C# throws `ArgumentException`).
    fn absolute_source(key: &FontFamilyKey) -> &url::Url {
        key.absolute_source().unwrap_or_else(|| panic!("Relative uri {} without base url", key.source()))
    }

    /// Returns the registered collection, or creates and registers one. A
    /// collection registered for the key while the factory ran wins, and the
    /// new one is disposed.
    fn get_or_create_font_collection(
        &self,
        key: &url::Url,
        factory: impl FnOnce(&url::Url) -> Rc<dyn IFontCollection>,
    ) -> Rc<dyn IFontCollection> {
        if let Some(existing) = self.font_collections.borrow().get(key) {
            return existing.clone();
        }

        // The factory runs without a borrow of the collections: it calls into the font backend.
        let candidate = factory(key);

        let winner = self.font_collections.borrow_mut().entry(key.clone()).or_insert_with(|| candidate.clone()).clone();
        advance_glyph_typeface_generation();

        if !Rc::ptr_eq(&winner, &candidate) {
            // Our candidate lost - dispose it to avoid the leak.
            candidate.dispose();
        }

        winner
    }

    fn get_default_font_family_name(&self, options: Option<&FontManagerOptions>) -> String {
        let mut default_font_family_name = options
            .and_then(|options| options.default_family_name.clone())
            .unwrap_or_else(|| self.platform_impl.get_default_font_family_name());

        if default_font_family_name.is_empty() {
            let system_fonts = self.system_fonts();

            if system_fonts.count() > 0 {
                default_font_family_name = system_fonts.get(0).name().to_owned();
            }
        }

        if default_font_family_name.is_empty() {
            panic!("Default font family name can't be null or empty.");
        }

        if default_font_family_name == FontFamily::DEFAULT_FONT_FAMILY_NAME {
            panic!(
                "'{}' is a placeholder and cannot be used as the default font family name. Provide a concrete font family name via FontManagerOptions or the platform implementation.",
                FontFamily::DEFAULT_FONT_FAMILY_NAME
            );
        }

        default_font_family_name
    }

    /// Disposes every font collection and the font backend.
    pub fn dispose(&self) {
        let font_collections: Vec<Rc<dyn IFontCollection>> =
            self.font_collections.borrow_mut().drain().map(|(_, font_collection)| font_collection).collect();

        for font_collection in font_collections {
            font_collection.dispose();
        }

        self.platform_impl.dispose();
    }
}

/// The key of a URL in the font collections: the URL without its fragment, borrowed when it has
/// none (every source a font family makes).
fn collection_key(url: &url::Url) -> Cow<'_, url::Url> {
    if url.fragment().is_none() {
        return Cow::Borrowed(url);
    }

    let mut key = url.clone();
    key.set_fragment(None);
    Cow::Owned(key)
}
