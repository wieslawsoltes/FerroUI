use std::any::Any;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::rc::Rc;

use crate::media::fonts::font_family_loader::FontFamilyLoader;
use crate::media::fonts::{FontCollectionKey, FontFallbackScriptHints, IFontCollection};
use crate::media::text_formatting::unicode::{Codepoint, Script};
use crate::media::{FontFamily, FontSimulations, FontStretch, FontStyle, FontWeight, GlyphTypeface, Typeface};
use crate::platform::{IAssetLoader, IFontManagerImpl, ASSET_SCHEME};
use crate::utilities::{CultureInfo, Uri};
use crate::{FerroLocator, LocatorExtensions};

/// The typefaces of one font family: glyph typefaces by style, weight and
/// stretch. A `None` value records that the platform has no typeface for the
/// key, so it is not asked again.
pub type GlyphTypefaceMap = BTreeMap<FontCollectionKey, Option<Rc<GlyphTypeface>>>;

/// Maps a character to the one used for ordinal case-insensitive comparison
/// (the simple upper case mapping).
fn to_upper_ordinal(c: char) -> char {
    if c.is_ascii() {
        return c.to_ascii_uppercase();
    }

    let mut upper = c.to_uppercase();

    match (upper.next(), upper.next()) {
        (Some(single), None) => single,
        _ => c,
    }
}

/// Orders characters as their UTF-16 encodings order.
fn utf16_order_key(c: char) -> (u32, u32) {
    let value = c as u32;

    if value >= 0x10000 {
        (0xD800 + ((value - 0x10000) >> 10), value)
    } else {
        (value, 0)
    }
}

/// Compares two strings ignoring case (C# `StringComparison.OrdinalIgnoreCase`).
pub(crate) fn compare_ordinal_ignore_case(a: &str, b: &str) -> Ordering {
    let mut a = a.chars();
    let mut b = b.chars();

    loop {
        match (a.next(), b.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                if x == y {
                    continue;
                }

                let (x, y) = (to_upper_ordinal(x), to_upper_ordinal(y));

                if x != y {
                    return utf16_order_key(x).cmp(&utf16_order_key(y));
                }
            }
        }
    }
}

/// Whether two strings are equal ignoring case (C# `StringComparison.OrdinalIgnoreCase`).
pub(crate) fn equals_ordinal_ignore_case(a: &str, b: &str) -> bool {
    compare_ordinal_ignore_case(a, b) == Ordering::Equal
}

/// Whether `value` starts with `prefix` ignoring case (C# `StringComparison.OrdinalIgnoreCase`).
pub(crate) fn starts_with_ordinal_ignore_case(value: &str, prefix: &str) -> bool {
    let mut value = value.chars();

    for expected in prefix.chars() {
        match value.next() {
            Some(actual) if actual == expected || to_upper_ordinal(actual) == to_upper_ordinal(expected) => {}
            _ => return false,
        }
    }

    true
}

/// The glyph typefaces of a font collection by family name. Family names are
/// compared ignoring case.
///
/// The entries are kept sorted by name, so lookups neither allocate nor hash.
#[derive(Default)]
pub struct GlyphTypefaceCache {
    entries: RefCell<Vec<(String, Rc<RefCell<GlyphTypefaceMap>>)>>,
}

impl GlyphTypefaceCache {
    fn search(entries: &[(String, Rc<RefCell<GlyphTypefaceMap>>)], family_name: &str) -> Result<usize, usize> {
        entries.binary_search_by(|(name, _)| compare_ordinal_ignore_case(name, family_name))
    }

    /// The typefaces cached for the family.
    pub fn try_get_value(&self, family_name: &str) -> Option<Rc<RefCell<GlyphTypefaceMap>>> {
        let entries = self.entries.borrow();

        Self::search(&entries, family_name).ok().map(|index| entries[index].1.clone())
    }

    /// The typefaces cached for the family, adding an empty entry when the
    /// family is not cached yet. The flag tells whether the entry was added.
    fn get_or_add(&self, family_name: &str) -> (Rc<RefCell<GlyphTypefaceMap>>, bool) {
        let mut entries = self.entries.borrow_mut();

        match Self::search(&entries, family_name) {
            Ok(index) => (entries[index].1.clone(), false),
            Err(index) => {
                let map = Rc::new(RefCell::new(GlyphTypefaceMap::new()));
                entries.insert(index, (family_name.to_owned(), map.clone()));
                (map, true)
            }
        }
    }

    /// Whether typefaces are cached for the family.
    pub fn contains_key(&self, family_name: &str) -> bool {
        Self::search(&self.entries.borrow(), family_name).is_ok()
    }

    /// The number of cached families.
    pub fn count(&self) -> usize {
        self.entries.borrow().len()
    }

    /// The names of the cached families, sorted ignoring case.
    pub fn keys(&self) -> Vec<String> {
        self.entries.borrow().iter().map(|(name, _)| name.clone()).collect()
    }

    fn values(&self) -> Vec<Rc<RefCell<GlyphTypefaceMap>>> {
        self.entries.borrow().iter().map(|(_, map)| map.clone()).collect()
    }
}

/// Identifies a script/culture bucket of the fallback cache.
type ScriptFallbackKey = (Script, Option<CultureInfo>);

/// The state and the shared matching, fallback and caching logic of font
/// collections.
///
/// Upstream this is an abstract class. Here a collection type holds a
/// `FontCollectionBase` and implements [`FontCollectionBaseImpl`] (the
/// abstract and virtual members); [`IFontCollection`] is then implemented for
/// it automatically. Members of the base class that reach overridable members
/// are associated functions taking the collection as `this`
/// (`FontCollectionBase::try_add_glyph_typeface(this, ..)`); the others are
/// methods of the state (`collection.base().count()`).
pub struct FontCollectionBase {
    glyph_typeface_cache: GlyphTypefaceCache,

    // Cache of resolved script/culture fallback family names. A `Some` value is the preferred
    // fallback family for that script bucket: a Tier B hint that is still re-checked for coverage
    // per codepoint, and which does NOT by itself suppress a platform call. A `None` value is a
    // *negative* entry that prevents repeated platform-fallback calls for the same script bucket.
    script_fallback_cache: RefCell<HashMap<ScriptFallbackKey, Option<String>>>,

    font_families: RefCell<Vec<FontFamily>>,
    font_manager_impl: Rc<dyn IFontManagerImpl>,
    asset_loader: Option<Rc<dyn IAssetLoader>>,
}

/// The abstract and virtual members of a font collection deriving from
/// [`FontCollectionBase`].
///
/// `this` is the collection itself. A default implementation is the base
/// class behaviour; an override calls it with
/// `FontCollectionBase::<member>(this, ..)`.
pub trait FontCollectionBaseImpl: Sized + 'static {
    /// The base class state of the collection.
    fn base(this: &Self) -> &FontCollectionBase;

    /// Gets the unique identifier for the font collection.
    fn key(this: &Self) -> Uri;

    /// Tries to match a specified character to a [`Typeface`] that supports
    /// specified font properties.
    fn try_match_character(
        this: &Self,
        codepoint: i32,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Typeface> {
        FontCollectionBase::try_match_character(this, codepoint, style, weight, stretch, family_name, culture)
    }

    /// Hook for platform-backed collections (e.g. the system font collection)
    /// to consult the underlying font manager for a fallback typeface.
    /// Invoked at most once per (script-bucket, culture) pair from
    /// [`FontCollectionBase::try_match_character`].
    fn try_match_character_from_platform(
        _this: &Self,
        _codepoint: i32,
        _key: FontCollectionKey,
        _family_name: Option<&str>,
        _culture: Option<&CultureInfo>,
    ) -> Option<Rc<GlyphTypeface>> {
        None
    }

    /// Try to get a synthetic glyph typeface for given parameters.
    fn try_create_synthetic_glyph_typeface(
        this: &Self,
        glyph_typeface: &Rc<GlyphTypeface>,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        FontCollectionBase::try_create_synthetic_glyph_typeface(this, glyph_typeface, style, weight, stretch)
    }

    /// Try to get a glyph typeface for given parameters.
    fn try_get_glyph_typeface(
        this: &Self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        FontCollectionBase::try_get_glyph_typeface(this, family_name, style, weight, stretch)
    }

    /// Tries to get a list of typefaces for the specified family name.
    fn try_get_family_typefaces(this: &Self, family_name: &str) -> Option<Vec<Typeface>> {
        FontCollectionBase::try_get_family_typefaces(this, family_name)
    }
}

/// A font collection deriving from [`FontCollectionBase`], as a trait object:
/// the `this` of the base class members. Implemented automatically for every
/// [`FontCollectionBaseImpl`].
pub trait IFontCollectionBase: IFontCollection {
    /// The base class state of the collection.
    fn base(&self) -> &FontCollectionBase;

    /// Calls the collection's
    /// [`try_match_character_from_platform`](FontCollectionBaseImpl::try_match_character_from_platform).
    fn try_match_character_from_platform(
        &self,
        codepoint: i32,
        key: FontCollectionKey,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Rc<GlyphTypeface>>;
}

impl<T: FontCollectionBaseImpl> IFontCollectionBase for T {
    fn base(&self) -> &FontCollectionBase {
        T::base(self)
    }

    fn try_match_character_from_platform(
        &self,
        codepoint: i32,
        key: FontCollectionKey,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Rc<GlyphTypeface>> {
        T::try_match_character_from_platform(self, codepoint, key, family_name, culture)
    }
}

impl<T: FontCollectionBaseImpl> IFontCollection for T {
    fn key(&self) -> Uri {
        T::key(self)
    }

    fn count(&self) -> usize {
        T::base(self).count()
    }

    fn get(&self, index: usize) -> FontFamily {
        T::base(self).get(index)
    }

    fn font_families(&self) -> Vec<FontFamily> {
        T::base(self).font_families()
    }

    fn try_get_glyph_typeface(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        T::try_get_glyph_typeface(self, family_name, style, weight, stretch)
    }

    fn try_match_character(
        &self,
        codepoint: i32,
        font_style: FontStyle,
        font_weight: FontWeight,
        font_stretch: FontStretch,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Typeface> {
        T::try_match_character(self, codepoint, font_style, font_weight, font_stretch, family_name, culture)
    }

    fn try_get_family_typefaces(&self, family_name: &str) -> Option<Vec<Typeface>> {
        T::try_get_family_typefaces(self, family_name)
    }

    fn try_create_synthetic_glyph_typeface(
        &self,
        glyph_typeface: &Rc<GlyphTypeface>,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        T::try_create_synthetic_glyph_typeface(self, glyph_typeface, style, weight, stretch)
    }

    fn try_get_nearest_match(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        T::base(self).try_get_nearest_match(family_name, style, weight, stretch)
    }

    fn dispose(&self) {
        T::base(self).dispose();
    }

    fn as_font_collection_base(&self) -> Option<&dyn IFontCollectionBase> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Default for FontCollectionBase {
    fn default() -> Self {
        Self::new()
    }
}

impl FontCollectionBase {
    /// Creates the state of a font collection.
    ///
    /// Panics when no font manager backend (`IFontManagerImpl`) is registered
    /// with the locator.
    pub fn new() -> Self {
        Self {
            glyph_typeface_cache: GlyphTypefaceCache::default(),
            script_fallback_cache: RefCell::new(HashMap::new()),
            font_families: RefCell::new(Vec::new()),
            font_manager_impl: FerroLocator::current().get_required_service::<dyn IFontManagerImpl>(),
            asset_loader: FerroLocator::current().get_service::<dyn IAssetLoader>(),
        }
    }

    /// The cached glyph typefaces by family name.
    pub fn glyph_typeface_cache(&self) -> &GlyphTypefaceCache {
        &self.glyph_typeface_cache
    }

    /// The number of font families in the collection.
    pub fn count(&self) -> usize {
        self.font_families.borrow().len()
    }

    /// The font family at `index`.
    ///
    /// Panics when `index` is out of range.
    pub fn get(&self, index: usize) -> FontFamily {
        self.font_families.borrow()[index].clone()
    }

    /// A snapshot of the font families of the collection, sorted by name.
    pub fn font_families(&self) -> Vec<FontFamily> {
        self.font_families.borrow().clone()
    }

    /// The base implementation of
    /// [`FontCollectionBaseImpl::try_match_character`].
    pub fn try_match_character(
        this: &dyn IFontCollectionBase,
        codepoint: i32,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Typeface> {
        Self::try_match_character_with_script(
            this,
            codepoint,
            style,
            weight,
            stretch,
            family_name,
            culture,
            Script::Unknown,
        )
    }

    /// Character-to-typeface match with an optional shaping-capability
    /// constraint. When `shaping_script` is a complex script, only candidates
    /// that can shape it ([`GlyphTypeface::can_shape_script`]) are considered;
    /// [`Script::Unknown`] imposes no constraint and is identical to the
    /// public overload.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn try_match_character_with_script(
        this: &dyn IFontCollectionBase,
        codepoint: i32,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
        shaping_script: Script,
    ) -> Option<Typeface> {
        let base = this.base();

        let key = FontCollectionKey::new(style, weight, stretch);
        let cp = Codepoint::new(codepoint as u32);
        let script = cp.script();
        let refined_script = FontFallbackScriptHints::refine_with_culture(cp, culture);
        let script_key: ScriptFallbackKey = (refined_script, culture.cloned());

        // --- Tier A: requested family, coverage-checked, culture-compatible ---
        if let Some(family_name) = family_name {
            if let Some(requested_family) = base.glyph_typeface_cache.try_get_value(family_name) {
                if let Some(requested_glyph_typeface) = Self::try_get_covering_match_for_family(
                    this,
                    &requested_family,
                    key,
                    codepoint,
                    culture,
                    shaping_script,
                ) {
                    if Self::is_culture_compatible(&requested_glyph_typeface, culture, script) {
                        return Some(Self::build_typeface_with_synthesis(this, &requested_glyph_typeface, key));
                    }
                }
            }
        }

        // --- Tier B: cached script/culture resolution ---
        if FontFallbackScriptHints::is_locale_sensitive(refined_script) || culture.is_some() {
            let cached_family = base.script_fallback_cache.borrow().get(&script_key).cloned().flatten();

            if let Some(cached_family) = cached_family {
                if !family_name.is_some_and(|family_name| equals_ordinal_ignore_case(&cached_family, family_name)) {
                    if let Some(cached_typefaces) = base.glyph_typeface_cache.try_get_value(&cached_family) {
                        if let Some(cached_glyph_typeface) = Self::try_get_covering_match_for_family(
                            this,
                            &cached_typefaces,
                            key,
                            codepoint,
                            culture,
                            shaping_script,
                        ) {
                            return Some(Self::build_typeface_with_synthesis(this, &cached_glyph_typeface, key));
                        }
                    }
                }
            }
        }

        // --- Tier C: deterministic cache sweep (non last-resort), culture-scored ---
        if let Some((best_gt, best_family_name)) = base.try_match_in_cache(
            codepoint,
            key,
            family_name,
            culture,
            script,
            refined_script,
            shaping_script,
            false,
        ) {
            // The sweep returns the nearest cached key for the winning family. A codepoint the
            // bucket family can't cover (e.g. a Simplified-only ideograph when the bucket is a JP
            // font) lands here, so the exact-key upgrade is needed here too: otherwise a Bold
            // face cached for an earlier run is rendered for a Normal request.
            let best_gt = Self::prefer_exact_key(this, best_gt, key, codepoint, culture, shaping_script);

            if FontFallbackScriptHints::is_locale_sensitive(refined_script) || culture.is_some() {
                base.script_fallback_cache.borrow_mut().entry(script_key).or_insert(Some(best_family_name));
            }

            return Some(Self::build_typeface_with_synthesis(this, &best_gt, key));
        }

        // --- Tier D: platform fallback ---
        // Only a *negative* cache entry (the platform had no font for this script bucket)
        // suppresses a retry. A *positive* entry must not: it records a preferred family for the
        // bucket, but that family may not cover THIS codepoint - e.g. U+4E2D resolves to a CJK
        // font that lacks the Simplified-only U+534E - and the platform resolves per codepoint,
        // so it may still place a codepoint the bucket's font cannot. Reaching Tier D already
        // means no cached family covered this codepoint.
        // Skipped for shaping-constrained queries: the platform match is not capability-checked,
        // and the caller's unconstrained pass already covers the platform path (so the negative
        // cache stays consistent with unconstrained lookups).
        if shaping_script == Script::Unknown {
            let cache_entry = base.script_fallback_cache.borrow().get(&script_key).cloned();
            let has_cache_entry = cache_entry.is_some();
            let platform_already_attempted = matches!(cache_entry, Some(None));

            if !platform_already_attempted {
                if let Some(platform_gt) =
                    this.try_match_character_from_platform(codepoint, key, family_name, culture)
                {
                    // Keep any existing positive hint (the entry is not overwritten); registering
                    // the match lets later lookups for this codepoint be served by Tier C without
                    // the platform.
                    base.script_fallback_cache
                        .borrow_mut()
                        .entry(script_key)
                        .or_insert_with(|| Some(platform_gt.family_name().to_owned()));

                    return Some(Self::build_typeface_with_synthesis(this, &platform_gt, key));
                }
            }

            // Record a negative only when nothing was cached for the bucket yet, so a positive hint
            // for one codepoint isn't downgraded to a negative by another the platform can't place.
            if !has_cache_entry {
                base.script_fallback_cache.borrow_mut().entry(script_key).or_insert(None);
            }
        }

        // --- Tier E: last-resort cache sweep ---
        if let Some((lr_gt, _)) = base.try_match_in_cache(
            codepoint,
            key,
            family_name,
            culture,
            script,
            refined_script,
            shaping_script,
            true,
        ) {
            return Some(Self::build_typeface_with_synthesis(this, &lr_gt, key));
        }

        None
    }

    #[allow(clippy::too_many_arguments)]
    fn try_match_in_cache(
        &self,
        codepoint: i32,
        key: FontCollectionKey,
        skip_family_name: Option<&str>,
        culture: Option<&CultureInfo>,
        script: Script,
        refined_script: Script,
        shaping_script: Script,
        is_last_resort: bool,
    ) -> Option<(Rc<GlyphTypeface>, String)> {
        let mut best: Option<(Rc<GlyphTypeface>, String)> = None;

        // Iterate the sorted family list for deterministic order.
        let snapshot = self.font_families.borrow();
        let mut best_score = i32::MIN;

        for font_family in snapshot.iter() {
            let family_name = font_family.name();

            if skip_family_name.is_some_and(|skip| equals_ordinal_ignore_case(family_name, skip)) {
                continue;
            }

            let Some(glyph_typefaces) = self.glyph_typeface_cache.try_get_value(family_name) else {
                continue;
            };

            let Some(candidate) = Self::try_get_covering_match(
                &glyph_typefaces.borrow(),
                key,
                codepoint,
                is_last_resort,
                shaping_script,
            ) else {
                continue;
            };

            let score = Self::score_candidate(&candidate, key, culture, script, refined_script);

            if score > best_score {
                best_score = score;
                best = Some((candidate, family_name.to_owned()));
            }
        }

        best
    }

    fn score_candidate(
        candidate: &GlyphTypeface,
        requested_key: FontCollectionKey,
        culture: Option<&CultureInfo>,
        script: Script,
        refined_script: Script,
    ) -> i32 {
        let mut score = 0;

        if let Some(culture) = culture {
            // Exact culture match in the font's name table.
            if candidate.family_names().contains_key(culture) {
                score += 8;
            } else {
                let parent = culture.parent();

                if !parent.is_invariant() && candidate.family_names().contains_key(&parent) {
                    score += 4;
                }
            }

            // Self-declared culture coverage via OS/2 codepage bits or meta dlng/slng.
            if FontFallbackScriptHints::is_font_compatible_with_culture(candidate, Some(culture)) {
                score += 4;
            }
        }

        // Font's primary script aligns with the requested script - ask the font directly.
        if FontFallbackScriptHints::is_locale_sensitive(refined_script) {
            if candidate.supports_script(refined_script) {
                score += 2;
            } else if refined_script != script && candidate.supports_script(script) {
                score += 1;
            }
        }

        if FontCollectionKey::from(candidate) == requested_key {
            score += 1;
        }

        score
    }

    fn is_culture_compatible(candidate: &GlyphTypeface, culture: Option<&CultureInfo>, script: Script) -> bool {
        // If no culture or codepoint is locale-insensitive, the candidate is fine.
        let Some(culture) = culture else {
            return true;
        };

        if !FontFallbackScriptHints::is_locale_sensitive(script) {
            return true;
        }

        // Positive signal from the font's own self-declaration (OS/2 codepage bits or meta dlng/slng).
        // When the font declares coverage we accept it as compatible regardless of localized names.
        if FontFallbackScriptHints::is_font_compatible_with_culture(candidate, Some(culture)) {
            return true;
        }

        // If the font has no localized family names at all, treat as compatible (no negative signal).
        if candidate.family_names().is_empty() {
            return true;
        }

        if candidate.family_names().contains_key(culture) {
            return true;
        }

        let parent = culture.parent();

        if !parent.is_invariant() && candidate.family_names().contains_key(&parent) {
            return true;
        }

        // The font advertises localized names but not for this culture - reject so Tier C can score.
        false
    }

    fn build_typeface_with_synthesis(
        this: &dyn IFontCollectionBase,
        glyph_typeface: &Rc<GlyphTypeface>,
        requested_key: FontCollectionKey,
    ) -> Typeface {
        let matched_key = FontCollectionKey::from(&**glyph_typeface);

        // The fallback search already found a font face that maps the codepoint. Synthesis
        // (re-loading the stream with font simulations) is unsafe here: with .ttc collections
        // the platform may resolve a different face from the same stream (e.g. asking for an
        // oblique simulation of one face returns a sibling face). Accept the matched
        // glyph typeface as-is and pre-cache it under the requested key so later
        // glyph typeface lookups via the returned typeface short-circuit through the cache.
        if matched_key != requested_key {
            Self::try_add_glyph_typeface_by_name(
                this,
                glyph_typeface.family_name(),
                requested_key,
                Some(glyph_typeface.clone()),
            );
        }

        Typeface::with_style(
            FontFamily::with_base_uri(
                None,
                &format!("{}#{}", this.key().absolute_uri(), glyph_typeface.family_name()),
            ),
            requested_key.style,
            requested_key.weight,
            requested_key.stretch,
        )
    }

    /// Resolves a covering face for a single family at the requested key.
    /// Takes the cheap cached covering match first (an exact-key hit needs
    /// nothing more) and only escalates to the exact key when that match
    /// differs in any axis, so a Bold (or Italic, or Condensed) face cached
    /// for one run is not reused for a differently-keyed request of the same
    /// family.
    fn try_get_covering_match_for_family(
        this: &dyn IFontCollectionBase,
        glyph_typefaces: &Rc<RefCell<GlyphTypefaceMap>>,
        key: FontCollectionKey,
        codepoint: i32,
        culture: Option<&CultureInfo>,
        shaping_script: Script,
    ) -> Option<Rc<GlyphTypeface>> {
        // The borrow of the family's typefaces ends before the platform is consulted.
        let glyph_typeface =
            Self::try_get_covering_match(&glyph_typefaces.borrow(), key, codepoint, false, shaping_script)?;

        Some(Self::prefer_exact_key(this, glyph_typeface, key, codepoint, culture, shaping_script))
    }

    /// When `glyph_typeface` differs from the requested `key` in any axis
    /// (style, weight or stretch), asks the platform for the exact-key face
    /// of the same family - the only source of a key the cache lacks - via
    /// `try_match_character_from_platform`, and returns it when it is an
    /// exact, shapeable match; otherwise returns the input unchanged. The
    /// platform is consulted only on a mismatch, and a collection without one
    /// keeps the neighbouring match. This guards every key axis, not just
    /// weight.
    fn prefer_exact_key(
        this: &dyn IFontCollectionBase,
        glyph_typeface: Rc<GlyphTypeface>,
        key: FontCollectionKey,
        codepoint: i32,
        culture: Option<&CultureInfo>,
        shaping_script: Script,
    ) -> Rc<GlyphTypeface> {
        // The platform's character match, biased by the family already resolved, yields that family
        // at the requested key when it has that face (the match covers the codepoint, so no
        // extra coverage check is needed). Accept it only when it is the exact key and can shape.
        if FontCollectionKey::from(&*glyph_typeface) != key {
            if let Some(exact) = this.try_match_character_from_platform(
                codepoint,
                key,
                Some(glyph_typeface.family_name()),
                culture,
            ) {
                if FontCollectionKey::from(&*exact) == key && Self::can_shape(&exact, shaping_script) {
                    return exact;
                }
            }
        }

        glyph_typeface
    }

    // A candidate satisfies a shaping-capability constraint when it can shape the requested
    // script; `Script::Unknown` means no constraint (the historical, unconstrained behaviour).
    fn can_shape(glyph_typeface: &GlyphTypeface, shaping_script: Script) -> bool {
        shaping_script == Script::Unknown || glyph_typeface.can_shape_script(shaping_script)
    }

    /// Picks a variant of the family that both is close to the requested key
    /// and actually maps the requested codepoint. Falls back through the
    /// existing weight/stretch search but filters every candidate through the
    /// font's character-to-glyph map.
    fn try_get_covering_match(
        glyph_typefaces: &GlyphTypefaceMap,
        key: FontCollectionKey,
        codepoint: i32,
        is_last_resort: bool,
        shaping_script: Script,
    ) -> Option<Rc<GlyphTypeface>> {
        let covers = |glyph_typeface: &GlyphTypeface| {
            glyph_typeface.is_last_resort() == is_last_resort
                && glyph_typeface.character_to_glyph_map().try_get_glyph(codepoint).is_some()
                && Self::can_shape(glyph_typeface, shaping_script)
        };

        // Exact key first.
        if let Some(Some(glyph_typeface)) = glyph_typefaces.get(&key) {
            if covers(glyph_typeface) {
                return Some(glyph_typeface.clone());
            }
        }

        let mut covering_nearest: Option<&Rc<GlyphTypeface>> = None;
        let mut covering_distance = i32::MAX;

        // The map iterates its keys in sorted order.
        for (candidate_key, candidate) in glyph_typefaces {
            let Some(candidate) = candidate else {
                continue;
            };

            if !covers(candidate) {
                continue;
            }

            let distance = Self::key_distance(*candidate_key, key);

            if distance < covering_distance {
                covering_distance = distance;
                covering_nearest = Some(candidate);
            }
        }

        covering_nearest.cloned()
    }

    fn key_distance(a: FontCollectionKey, b: FontCollectionKey) -> i32 {
        let weight_delta = (a.weight.value() - b.weight.value()).abs();

        let stretch_delta = (a.stretch as i32 - b.stretch as i32).abs();

        let style_delta = if a.style == b.style { 0 } else { 1 };

        weight_delta + stretch_delta * 100 + style_delta * 10_000
    }

    /// The base implementation of
    /// [`FontCollectionBaseImpl::try_create_synthetic_glyph_typeface`].
    pub fn try_create_synthetic_glyph_typeface(
        this: &dyn IFontCollectionBase,
        glyph_typeface: &Rc<GlyphTypeface>,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        let base = this.base();

        // Source family should be present in the cache.
        let glyph_typefaces = base.glyph_typeface_cache.try_get_value(glyph_typeface.family_name())?;

        let key = FontCollectionKey::new(style, weight, stretch);

        let current_key = FontCollectionKey::from(&**glyph_typeface);

        if current_key == key {
            return None;
        }

        let mut font_simulations = FontSimulations::None;

        if style != FontStyle::Normal && glyph_typeface.style() != style {
            font_simulations |= FontSimulations::Oblique;
        }

        if weight.value() >= 600 && glyph_typeface.weight() < weight {
            font_simulations |= FontSimulations::Bold;
        }

        if font_simulations == FontSimulations::None {
            return None;
        }

        // A synthetic for this key may already be cached under the source family, reached
        // through another of its names. Building a second one copies the whole font file
        // through the platform stream, then loses the slot below to the instance already
        // there, so nothing caches it and nothing disposes it.
        if let Some(Some(cached_glyph_typeface)) = glyph_typefaces.borrow().get(&key) {
            if cached_glyph_typeface.font_simulations() == font_simulations {
                return Some(cached_glyph_typeface.clone());
            }
        }

        let mut stream = glyph_typeface.platform_typeface().try_get_stream()?;

        let platform_typeface =
            base.font_manager_impl.try_create_glyph_typeface_from_stream(&mut *stream, font_simulations)?;

        let synthetic_glyph_typeface = GlyphTypeface::try_create(platform_typeface, font_simulations)?;

        // Add the typographic family name to the cache
        if !glyph_typeface.typographic_family_name().is_empty() {
            Self::try_add_glyph_typeface_by_name(
                this,
                glyph_typeface.typographic_family_name(),
                key,
                Some(synthetic_glyph_typeface.clone()),
            );
        }

        for family_name in glyph_typeface.family_names().values() {
            Self::try_add_glyph_typeface_by_name(this, family_name, key, Some(synthetic_glyph_typeface.clone()));
        }

        Some(synthetic_glyph_typeface)
    }

    /// The base implementation of
    /// [`FontCollectionBaseImpl::try_get_glyph_typeface`].
    pub fn try_get_glyph_typeface(
        this: &dyn IFontCollectionBase,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        let (typeface, family_name) =
            Typeface::from_name_with_style(family_name, style, weight, stretch).normalize();

        let key = FontCollectionKey::from(&typeface);

        Self::try_get_glyph_typeface_by_key(this, &family_name, key, true)
    }

    /// The base implementation of
    /// [`FontCollectionBaseImpl::try_get_family_typefaces`].
    pub fn try_get_family_typefaces(this: &dyn IFontCollectionBase, family_name: &str) -> Option<Vec<Typeface>> {
        let glyph_typefaces = this.base().glyph_typeface_cache.try_get_value(family_name)?;

        // Take a snapshot of the keys: creating the font families must not hold the borrow.
        let keys: Vec<FontCollectionKey> = glyph_typefaces.borrow().keys().copied().collect();

        let collection_key = this.key();

        Some(
            keys.into_iter()
                .map(|key| {
                    Typeface::with_style(
                        FontFamily::new(&format!("{collection_key}#{family_name}")),
                        key.style,
                        key.weight,
                        key.stretch,
                    )
                })
                .collect(),
        )
    }

    /// Attempts to retrieve the glyph typeface that most closely matches the
    /// specified font family name, style, weight, and stretch.
    pub fn try_get_nearest_match(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        let glyph_typefaces = self.glyph_typeface_cache.try_get_value(family_name)?;

        let key = FontCollectionKey::new(style, weight, stretch);

        let glyph_typefaces = glyph_typefaces.borrow();

        Self::try_get_nearest_match_in(&glyph_typefaces, key)
    }

    /// Attempts to add the specified [`GlyphTypeface`] to the font collection.
    ///
    /// This method checks the family name and, if applicable, the typographic
    /// family name and other family names provided by the glyph typeface. If
    /// any of these names can be associated with the glyph typeface, the
    /// typeface is added to the collection. The method ensures that duplicate
    /// entries are not added.
    ///
    /// Returns `true` if the glyph typeface was successfully added to the
    /// collection; otherwise, `false`.
    pub fn try_add_glyph_typeface(this: &dyn IFontCollectionBase, glyph_typeface: &Rc<GlyphTypeface>) -> bool {
        let key = FontCollectionKey::from(&**glyph_typeface);

        Self::try_add_glyph_typeface_with_key(this, glyph_typeface, key)
    }

    /// Attempts to add the specified glyph typeface to the collection using
    /// the provided key.
    ///
    /// The method adds the glyph typeface using both its typographic family
    /// name and all available family names. If the glyph typeface's family
    /// name is empty, the method returns `false` and does not add the
    /// typeface.
    pub fn try_add_glyph_typeface_with_key(
        this: &dyn IFontCollectionBase,
        glyph_typeface: &Rc<GlyphTypeface>,
        key: FontCollectionKey,
    ) -> bool {
        if glyph_typeface.family_name().is_empty() {
            return false;
        }

        let mut result = false;

        // Add the typographic family name to the cache
        if !glyph_typeface.typographic_family_name().is_empty()
            && Self::try_add_glyph_typeface_by_name(
                this,
                glyph_typeface.typographic_family_name(),
                key,
                Some(glyph_typeface.clone()),
            )
        {
            result = true;
        }

        for family_name in glyph_typeface.family_names().values() {
            if Self::try_add_glyph_typeface_by_name(this, family_name, key, Some(glyph_typeface.clone())) {
                result = true;
            }
        }

        result
    }

    /// Attempts to add a glyph typeface from the specified font stream.
    ///
    /// The method first attempts to create a glyph typeface from the provided
    /// font stream. If successful, it adds the created glyph typeface to the
    /// collection.
    ///
    /// Returns the created glyph typeface if it was created and added;
    /// otherwise, `None`.
    pub fn try_add_glyph_typeface_from_stream(
        this: &dyn IFontCollectionBase,
        stream: &mut dyn Read,
    ) -> Option<Rc<GlyphTypeface>> {
        let platform_typeface =
            this.base().font_manager_impl.try_create_glyph_typeface_from_stream(stream, FontSimulations::None)?;

        let glyph_typeface = GlyphTypeface::try_create(platform_typeface, FontSimulations::None)?;

        Self::try_add_glyph_typeface(this, &glyph_typeface).then_some(glyph_typeface)
    }

    /// Attempts to add a font source to the font collection.
    ///
    /// This method processes the specified font source and attempts to load
    /// all available fonts from it. Fonts are added to the collection based
    /// on their family name and typographic family name (if available).
    ///
    /// `source` can be a `file:` path (a font file or a directory of font
    /// files) or a resource URI (`ferres:`, `resm:`).
    ///
    /// Returns `true` if at least one font from the specified source was
    /// successfully added to the font collection; otherwise, `false`.
    pub fn try_add_font_source(this: &dyn IFontCollectionBase, source: &Uri) -> bool {
        if !source.is_absolute_uri() {
            return false;
        }

        let base = this.base();

        let mut result = false;

        match source.scheme() {
            ASSET_SCHEME | "resm" => {
                let font_assets = FontFamilyLoader::load_font_assets(source);

                if font_assets.is_empty() {
                    return false;
                }

                let asset_loader = base
                    .asset_loader
                    .clone()
                    .unwrap_or_else(|| FerroLocator::current().get_required_service::<dyn IAssetLoader>());

                for font_asset in &font_assets {
                    let Ok(mut stream) = asset_loader.open(font_asset, None) else {
                        continue;
                    };

                    let Some(glyph_typeface) = base
                        .font_manager_impl
                        .try_create_glyph_typeface_from_stream(&mut *stream, FontSimulations::None)
                        .and_then(|platform_typeface| {
                            GlyphTypeface::try_create(platform_typeface, FontSimulations::None)
                        })
                    else {
                        continue;
                    };

                    let key = FontCollectionKey::from(&*glyph_typeface);

                    // Add the typographic family name to the cache
                    if !glyph_typeface.typographic_family_name().is_empty()
                        && Self::try_add_glyph_typeface_by_name(
                            this,
                            glyph_typeface.typographic_family_name(),
                            key,
                            Some(glyph_typeface.clone()),
                        )
                    {
                        result = true;
                    }

                    if Self::try_add_glyph_typeface_by_name(
                        this,
                        glyph_typeface.family_name(),
                        key,
                        Some(glyph_typeface.clone()),
                    ) {
                        result = true;
                    }
                }
            }
            "file" => {
                result = Self::try_add_file_font_source(this, source);
            }
            _ => {
                // Unsupported scheme
                return false;
            }
        }

        result
    }

    #[cfg(not(target_family = "wasm"))]
    fn try_add_file_font_source(this: &dyn IFontCollectionBase, source: &Uri) -> bool {
        use std::fs::File;
        use std::path::{Path, PathBuf};

        let add_file = |path: &Path| -> bool {
            let Ok(mut stream) = File::open(path) else {
                return false;
            };

            this.base()
                .font_manager_impl
                .try_create_glyph_typeface_from_stream(&mut stream, FontSimulations::None)
                .and_then(|platform_typeface| GlyphTypeface::try_create(platform_typeface, FontSimulations::None))
                .is_some_and(|glyph_typeface| Self::try_add_glyph_typeface(this, &glyph_typeface))
        };

        let local_path = PathBuf::from(local_path(source));

        // If the path is a file, load the font file directly
        if FontFamilyLoader::is_font_source(source) {
            if !local_path.is_file() {
                return false;
            }

            return add_file(&local_path);
        }

        // If the path is a directory, load all font files from that directory
        let Ok(entries) = std::fs::read_dir(&local_path) else {
            return false;
        };

        // The directory is read in name order so that the result does not
        // depend on the file system's enumeration order.
        let mut files: Vec<PathBuf> =
            entries.filter_map(Result::ok).map(|entry| entry.path()).filter(|path| path.is_file()).collect();

        files.sort();

        let mut result = false;

        for file in &files {
            if file.to_str().is_some_and(FontFamilyLoader::is_font_file) && add_file(file) {
                result = true;
            }
        }

        result
    }

    /// There is no file system on this target.
    #[cfg(target_family = "wasm")]
    fn try_add_file_font_source(_this: &dyn IFontCollectionBase, _source: &Uri) -> bool {
        false
    }

    /// Inserts the specified font family into the collection, maintaining the
    /// collection in sorted order by font family name.
    ///
    /// If a font family with the same name (ignoring case) already exists in
    /// the collection, nothing is added.
    pub fn add_font_family(&self, font_family: FontFamily) {
        let mut font_families = self.font_families.borrow_mut();

        let index = font_families
            .binary_search_by(|existing| compare_ordinal_ignore_case(existing.name(), font_family.name()));

        // If an existing family with the same name is present, do nothing
        if let Err(index) = index {
            font_families.insert(index, font_family);
        }
    }

    /// Attempts to retrieve a glyph typeface that matches the specified font
    /// family name and font collection key.
    ///
    /// This method performs a binary search to locate font families with
    /// names that match the specified `family_name` (ignoring case). If
    /// multiple matches are found, the method iterates over them to find the
    /// best match based on the provided `key`.
    ///
    /// `allow_nearest_match` tells whether to allow a nearest match (as
    /// opposed to only an exact match).
    pub fn try_get_glyph_typeface_by_key(
        this: &dyn IFontCollectionBase,
        family_name: &str,
        key: FontCollectionKey,
        allow_nearest_match: bool,
    ) -> Option<Rc<GlyphTypeface>> {
        let base = this.base();

        if let Some(glyph_typefaces) = base.glyph_typeface_cache.try_get_value(family_name) {
            // The borrow of the family's typefaces ends before the synthetic typeface is created.
            let matched = Self::try_get_match(&glyph_typefaces.borrow(), key, allow_nearest_match);

            if let Some((mut glyph_typeface, is_nearest_match)) = matched {
                let matched_key = FontCollectionKey::from(&*glyph_typeface);

                if is_nearest_match && matched_key != key {
                    if let Some(synthetic_glyph_typeface) = this.try_create_synthetic_glyph_typeface(
                        &glyph_typeface,
                        key.style,
                        key.weight,
                        key.stretch,
                    ) {
                        glyph_typeface = synthetic_glyph_typeface;
                    }

                    // The synthetic typeface is registered only under the source font's own
                    // family names, so a request arriving through a different name would
                    // otherwise miss the cache and re-synthesise on every call, copying the
                    // whole font file each time.
                    Self::try_add_glyph_typeface_by_name(this, family_name, key, Some(glyph_typeface.clone()));
                }

                return Some(glyph_typeface);
            }
        }

        // Binary search for the first possible prefix match
        let snapshot = base.font_families.borrow();
        let mut left = 0isize;
        let mut right = snapshot.len() as isize - 1;
        let mut first_match: Option<usize> = None;

        while left <= right {
            let mid = ((left + right) / 2) as usize;
            let name = snapshot[mid].name();

            match compare_ordinal_ignore_case(name, family_name) {
                // If the current name is lexicographically less than the search name, move right
                Ordering::Less => left = mid as isize + 1,
                Ordering::Equal => {
                    // Exact match found. Use the exact family name for lookup.
                    // Exact family present but no matching typeface found gives `None`.
                    let exact_glyph_typefaces = base.glyph_typeface_cache.try_get_value(name)?;
                    let exact_glyph_typefaces = exact_glyph_typefaces.borrow();

                    return Self::try_get_match(&exact_glyph_typefaces, key, allow_nearest_match)
                        .map(|(glyph_typeface, _)| glyph_typeface);
                }
                Ordering::Greater => {
                    // Only check for prefix when the name is greater than the family name. This
                    // avoids the more expensive prefix check for names that are definitely
                    // ordered before the search term.
                    if starts_with_ordinal_ignore_case(name, family_name) {
                        first_match = Some(mid);
                    }

                    // Continue searching to the left for the first match
                    right = mid as isize - 1;
                }
            }
        }

        if let Some(first_match) = first_match {
            // Iterate over all consecutive prefix matches
            for font_family in &snapshot[first_match..] {
                if !starts_with_ordinal_ignore_case(font_family.name(), family_name) {
                    break;
                }

                if let Some(glyph_typefaces) = base.glyph_typeface_cache.try_get_value(font_family.name()) {
                    if let Some((glyph_typeface, _)) =
                        Self::try_get_match(&glyph_typefaces.borrow(), key, allow_nearest_match)
                    {
                        return Some(glyph_typeface);
                    }
                }
            }
        }

        None
    }

    /// The match and whether it is a nearest (not an exact) match.
    fn try_get_match(
        glyph_typefaces: &GlyphTypefaceMap,
        key: FontCollectionKey,
        allow_nearest_match: bool,
    ) -> Option<(Rc<GlyphTypeface>, bool)> {
        if let Some(Some(glyph_typeface)) = glyph_typefaces.get(&key) {
            return Some((glyph_typeface.clone(), false));
        }

        if allow_nearest_match {
            if let Some(glyph_typeface) = Self::try_get_nearest_match_in(glyph_typefaces, key) {
                return Some((glyph_typeface, true));
            }
        }

        None
    }

    /// Attempts to retrieve the nearest matching [`GlyphTypeface`] for the
    /// specified font key from the provided collection of glyph typefaces.
    ///
    /// This method attempts to find the best match for the specified font key
    /// by considering various fallback strategies, such as normalizing the
    /// font style, stretch, and weight. If no suitable match is found, the
    /// method will return the first available glyph typeface from the
    /// collection, if any.
    pub fn try_get_nearest_match_in(
        glyph_typefaces: &GlyphTypefaceMap,
        key: FontCollectionKey,
    ) -> Option<Rc<GlyphTypeface>> {
        Self::try_get_nearest_match_core(glyph_typefaces, key, false)
            .or_else(|| Self::try_get_nearest_match_core(glyph_typefaces, key, true))
    }

    fn try_get_nearest_match_core(
        glyph_typefaces: &GlyphTypefaceMap,
        mut key: FontCollectionKey,
        is_last_resort: bool,
    ) -> Option<Rc<GlyphTypeface>> {
        if let Some(Some(glyph_typeface)) = glyph_typefaces.get(&key) {
            if glyph_typeface.is_last_resort() == is_last_resort {
                return Some(glyph_typeface.clone());
            }
        }

        if key.style != FontStyle::Normal {
            key.style = FontStyle::Normal;
        }

        if key.stretch != FontStretch::Normal {
            if let Some(glyph_typeface) = Self::try_find_stretch_fallback(glyph_typefaces, key, is_last_resort) {
                return Some(glyph_typeface);
            }

            if key.weight != FontWeight::Normal {
                if let Some(glyph_typeface) = Self::try_find_stretch_fallback(
                    glyph_typefaces,
                    FontCollectionKey { weight: FontWeight::Normal, ..key },
                    is_last_resort,
                ) {
                    return Some(glyph_typeface);
                }
            }

            key.stretch = FontStretch::Normal;
        }

        if let Some(glyph_typeface) = Self::try_find_weight_fallback(glyph_typefaces, key, is_last_resort) {
            return Some(glyph_typeface);
        }

        if let Some(glyph_typeface) = Self::try_find_stretch_fallback(glyph_typefaces, key, is_last_resort) {
            return Some(glyph_typeface);
        }

        // Take the first glyph typeface we can find.
        glyph_typefaces
            .values()
            .flatten()
            .find(|typeface| is_last_resort == typeface.is_last_resort())
            .cloned()
    }

    /// Attempts to add a glyph typeface to the cache for the specified font
    /// family and key.
    ///
    /// The font family is added to the collection, in sorted order, with the
    /// first glyph typeface cached for it.
    ///
    /// `glyph_typeface` can be `None` (the platform has no typeface for the
    /// key): the answer is cached so that the platform is not asked again,
    /// and the family is not added to the collection, which lists only
    /// families it has a typeface of.
    ///
    /// Returns `true` if the glyph typeface was successfully added to the
    /// cache or the same typeface is already cached for the key; otherwise,
    /// `false`.
    pub fn try_add_glyph_typeface_by_name(
        this: &dyn IFontCollectionBase,
        family_name: &str,
        key: FontCollectionKey,
        glyph_typeface: Option<Rc<GlyphTypeface>>,
    ) -> bool {
        if family_name.is_empty() {
            return false;
        }

        let base = this.base();

        let (glyph_typefaces, _) = base.glyph_typeface_cache.get_or_add(family_name);

        let mut glyph_typefaces = glyph_typefaces.borrow_mut();

        // Add or compare the glyph typeface.
        if let Some(existing) = glyph_typefaces.get(&key) {
            return match (existing, &glyph_typeface) {
                (Some(existing), Some(glyph_typeface)) => Rc::ptr_eq(existing, glyph_typeface),
                (None, None) => true,
                _ => false,
            };
        }

        // Deviation (DEVIATIONS.md, Fonts): upstream's `TryAddGlyphTypeface` publishes the family
        // when it creates the cache entry, also for a miss, so a family the platform cannot
        // create becomes a family of the system fonts that has no glyph typeface. The family is
        // published once, with its first glyph typeface.
        let publish = glyph_typeface.is_some() && glyph_typefaces.values().all(Option::is_none);

        glyph_typefaces.insert(key, glyph_typeface);

        if publish {
            let font_family = FontFamily::new(&format!("{}#{}", this.key(), family_name));

            // Add the font family to the sorted list
            base.add_font_family(font_family);
        }

        true
    }

    /// Attempts to locate a fallback glyph typeface with a similar font
    /// stretch to the specified key within the provided collection.
    ///
    /// The search prioritizes font stretches closest to the requested value,
    /// expanding outward until a match is found or all options are exhausted.
    fn try_find_stretch_fallback(
        glyph_typefaces: &GlyphTypefaceMap,
        key: FontCollectionKey,
        is_last_resort: bool,
    ) -> Option<Rc<GlyphTypeface>> {
        let stretch = key.stretch as i32;

        let try_get_with_stretch = |effective_stretch: i32| -> Option<Rc<GlyphTypeface>> {
            let effective_stretch = FontStretch::from_i32(effective_stretch)?;

            match glyph_typefaces.get(&FontCollectionKey { stretch: effective_stretch, ..key }) {
                Some(Some(glyph_typeface)) if glyph_typeface.is_last_resort() == is_last_resort => {
                    Some(glyph_typeface.clone())
                }
                _ => None,
            }
        };

        if stretch < 5 {
            let mut i = 0;

            while stretch + i < 9 {
                if let Some(glyph_typeface) = try_get_with_stretch(stretch + i) {
                    return Some(glyph_typeface);
                }

                i += 1;
            }
        } else {
            let mut i = 0;

            while stretch - i > 1 {
                if let Some(glyph_typeface) = try_get_with_stretch(stretch - i) {
                    return Some(glyph_typeface);
                }

                i += 1;
            }
        }

        None
    }

    /// Attempts to locate a fallback glyph typeface in the specified
    /// collection that closely matches the weight of the provided key.
    ///
    /// The method searches for the closest available weight to the requested
    /// value, considering both lighter and heavier alternatives within the
    /// collection. If no exact match is found, it progressively searches for
    /// the nearest available weight in both directions.
    fn try_find_weight_fallback(
        glyph_typefaces: &GlyphTypefaceMap,
        key: FontCollectionKey,
        is_last_resort: bool,
    ) -> Option<Rc<GlyphTypeface>> {
        let weight = key.weight.value();

        let try_get_with_weight = |effective_weight: i32| -> Option<Rc<GlyphTypeface>> {
            match glyph_typefaces.get(&FontCollectionKey { weight: FontWeight(effective_weight), ..key }) {
                Some(Some(glyph_typeface)) if glyph_typeface.is_last_resort() == is_last_resort => {
                    Some(glyph_typeface.clone())
                }
                _ => None,
            }
        };

        // Looks for available weights from the target up to `limit`, in ascending order.
        let ascending = |limit: i32| -> Option<Rc<GlyphTypeface>> {
            let mut i = 0;

            while weight + i <= limit {
                if let Some(glyph_typeface) = try_get_with_weight(weight + i) {
                    return Some(glyph_typeface);
                }

                i += 50;
            }

            None
        };

        // Looks for available weights from the target down to 100, in descending order.
        let descending = || -> Option<Rc<GlyphTypeface>> {
            let mut i = 0;

            while weight - i >= 100 {
                if let Some(glyph_typeface) = try_get_with_weight(weight - i) {
                    return Some(glyph_typeface);
                }

                i += 50;
            }

            None
        };

        // If the target weight given is between 400 and 500 inclusive
        if (400..=500).contains(&weight) {
            // Look for available weights between the target and 500, in ascending order.
            // If no match is found, look for available weights less than the target, in descending order.
            // If no match is found, look for available weights greater than 500, in ascending order.
            if let Some(glyph_typeface) = ascending(500).or_else(descending).or_else(|| ascending(900)) {
                return Some(glyph_typeface);
            }
        }

        // If a weight less than 400 is given, look for available weights less than the target, in descending order.
        if weight < 400 {
            // If no match is found, look for available weights greater than the target, in ascending order.
            if let Some(glyph_typeface) = descending().or_else(|| ascending(900)) {
                return Some(glyph_typeface);
            }
        }

        // If a weight greater than 500 is given, look for available weights greater than the target, in ascending order.
        if weight > 500 {
            // If no match is found, look for available weights less than the target, in descending order.
            if let Some(glyph_typeface) = ascending(900).or_else(descending) {
                return Some(glyph_typeface);
            }
        }

        None
    }

    /// Disposes every cached glyph typeface.
    pub fn dispose(&self) {
        for glyph_typefaces in self.glyph_typeface_cache.values() {
            let glyph_typefaces: Vec<Rc<GlyphTypeface>> =
                glyph_typefaces.borrow().values().flatten().cloned().collect();

            for glyph_typeface in glyph_typefaces {
                glyph_typeface.dispose();
            }
        }
    }
}

/// The local operating-system representation of the path of a `file:` URI
/// (C# `Uri.LocalPath`): the unescaped path, with Windows drive paths in
/// their native form.
fn local_path(uri: &Uri) -> String {
    let path = crate::utilities::UriExtensions::get_unescape_absolute_path(uri);

    if cfg!(windows) && uri.scheme() == "file" {
        let bytes = path.as_bytes();

        // "/C:/dir/file" -> "C:\dir\file"
        if bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' {
            return path[1..].replace('/', "\\");
        }
    }

    path
}
