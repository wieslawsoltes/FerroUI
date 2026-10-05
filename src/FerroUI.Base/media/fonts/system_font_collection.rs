use std::rc::Rc;

use crate::media::fonts::font_collection_base::equals_ordinal_ignore_case;
use crate::media::fonts::{FontCollectionBase, FontCollectionBaseImpl, FontCollectionKey};
use crate::media::{
    FontFamily, FontManager, FontSimulations, FontStretch, FontStyle, FontWeight, GlyphTypeface, Typeface,
};
use crate::platform::IFontManagerImpl;
use crate::utilities::{CultureInfo, Uri};

/// The collection of the fonts installed in the system, backed by the font
/// manager backend.
pub struct SystemFontCollection {
    base: FontCollectionBase,
    platform_impl: Rc<dyn IFontManagerImpl>,
}

impl SystemFontCollection {
    pub fn new(platform_impl: Rc<dyn IFontManagerImpl>) -> Self {
        let collection = Self { base: FontCollectionBase::new(), platform_impl };

        for family_name in collection.platform_impl.get_installed_font_family_names(false) {
            if !family_name.is_empty() {
                collection.base.add_font_family(FontFamily::new(&family_name));
            }
        }

        collection
    }
}

impl FontCollectionBaseImpl for SystemFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(_this: &Self) -> Uri {
        FontManager::system_fonts_key()
    }

    fn try_get_glyph_typeface(
        this: &Self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>> {
        let (typeface, family_name) =
            Typeface::from_name_with_style(family_name, style, weight, stretch).normalize();
        let family_name = family_name.as_str();
        let key = FontCollectionKey::from(&typeface);

        // Find an exact match first
        if let Some(glyph_typeface) = FontCollectionBase::try_get_glyph_typeface_by_key(this, family_name, key, false)
        {
            return Some(glyph_typeface);
        }

        // Check cache first to avoid unnecessary calls to the font manager
        if let Some(glyph_typefaces) = this.base.glyph_typeface_cache().try_get_value(family_name) {
            if let Some(glyph_typeface) = glyph_typefaces.borrow().get(&key) {
                return glyph_typeface.clone();
            }
        }

        // Try to create the glyph typeface via system font manager
        let Some(platform_typeface) = this.platform_impl.try_create_glyph_typeface(family_name, style, weight, stretch)
        else {
            // Add `None` to cache to avoid future calls
            FontCollectionBase::try_add_glyph_typeface_by_name(this, family_name, key, None);

            return None;
        };

        // The font manager didn't return a perfect match either. Find the nearest match ourselves.
        if key != FontCollectionKey::from(&*platform_typeface) {
            if let Some(glyph_typeface) =
                FontCollectionBase::try_get_glyph_typeface_by_key(this, family_name, key, true)
            {
                return Some(glyph_typeface);
            }
        }

        let platform_family_name = platform_typeface.family_name();

        let glyph_typeface = GlyphTypeface::try_create(platform_typeface, FontSimulations::None)?;

        // Add to cache with platform typeface family name first
        FontCollectionBase::try_add_glyph_typeface_by_name(
            this,
            &platform_family_name,
            key,
            Some(glyph_typeface.clone()),
        );

        // Then the requested family name
        if !equals_ordinal_ignore_case(family_name, &platform_family_name) {
            FontCollectionBase::try_add_glyph_typeface_by_name(this, family_name, key, Some(glyph_typeface.clone()));
        }

        // Add to cache
        if !FontCollectionBase::try_add_glyph_typeface(this, &glyph_typeface) {
            // An entry for this key may have been added while the glyph typeface was created.
            // Re-check the cache and yield the existing glyph typeface if present.
            return this
                .base
                .glyph_typeface_cache()
                .try_get_value(family_name)
                .and_then(|existing_map| existing_map.borrow().get(&key).cloned().flatten());
        }

        // Requested glyph typeface should be in cache now
        FontCollectionBase::try_get_glyph_typeface_by_key(this, family_name, key, false)
    }

    fn try_get_family_typefaces(this: &Self, family_name: &str) -> Option<Vec<Typeface>> {
        this.platform_impl.try_get_family_typefaces(family_name)
    }

    fn try_match_character(
        this: &Self,
        codepoint: i32,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Typeface> {
        // Delegate to the base algorithm. The platform call is exposed through
        // `try_match_character_from_platform` and invoked at most once per (script-bucket, culture)
        // pair by the base implementation, after every cached candidate has been considered.
        FontCollectionBase::try_match_character(this, codepoint, style, weight, stretch, family_name, culture)
    }

    fn try_match_character_from_platform(
        this: &Self,
        codepoint: i32,
        key: FontCollectionKey,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Rc<GlyphTypeface>> {
        let platform_typeface =
            this.platform_impl.try_match_character(codepoint, key.style, key.weight, key.stretch, family_name, culture)?;

        let platform_key = FontCollectionKey::from(&*platform_typeface);
        let platform_family_name = platform_typeface.family_name();

        // Check cache first to avoid creating a duplicate glyph typeface.
        if let Some(glyph_typefaces) = this.base.glyph_typeface_cache().try_get_value(&platform_family_name) {
            if let Some(Some(existing)) = glyph_typefaces.borrow().get(&platform_key) {
                return Some(existing.clone());
            }
        }

        let glyph_typeface = GlyphTypeface::try_create(platform_typeface, FontSimulations::None)?;

        // Register in the cache so future lookups can short-circuit through the cache sweep of
        // the character match without re-invoking the platform.
        FontCollectionBase::try_add_glyph_typeface_by_name(
            this,
            &platform_family_name,
            platform_key,
            Some(glyph_typeface.clone()),
        );
        FontCollectionBase::try_add_glyph_typeface_with_key(this, &glyph_typeface, platform_key);

        Some(glyph_typeface)
    }
}
