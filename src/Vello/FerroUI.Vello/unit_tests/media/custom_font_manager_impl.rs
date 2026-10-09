//! Port of upstream's `Media/CustomFontManagerImpl.cs` of the Skia unit
//! tests: a font manager whose system fonts are the embedded test assets,
//! falling back to the font manager of the backend where upstream's falls
//! back to Skia's default font manager.

use crate::{FontManagerImpl, VelloTypeface};
use ferroui_base::media::fonts::{EmbeddedFontCollection, FontFamilyLoader, IFontCollection};
use ferroui_base::media::{
    FontManager, FontSimulations, FontStretch, FontStyle, FontWeight, IPlatformTypeface, Typeface,
};
use ferroui_base::platform::{IAssetLoader, IFontManagerImpl};
use ferroui_base::utilities::{CultureInfo, Uri, UriKind};
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::cell::RefCell;
use std::collections::HashSet;
use std::io::Read;
use std::rc::Rc;

/// The source of the embedded fonts that serve as the system fonts.
const ASSETS: &str = "resm:FerroUI.Vello.UnitTests.Assets?assembly=ferroui-vello";

pub struct CustomFontManagerImpl {
    default_family_name: String,
    bcp47: [String; 1],
    system_fonts: RefCell<Option<Rc<dyn IFontCollection>>>,
    /// The fonts of the system (upstream: `SKFontManager.Default`).
    font_manager: FontManagerImpl,
}

impl CustomFontManagerImpl {
    pub fn new() -> Self {
        // Seam: upstream passes the culture's `ThreeLetterISOLanguageName`
        // before its `TwoLetterISOLanguageName`; the port's `CultureInfo`
        // has no `ThreeLetterISOLanguageName` yet, so only the two letter
        // name is passed.
        Self {
            default_family_name: format!("{}#Noto Mono", FontManager::system_fonts_key()),
            bcp47: [CultureInfo::current_culture().two_letter_iso_language_name().to_owned()],
            system_fonts: RefCell::new(None),
            font_manager: FontManagerImpl::new(),
        }
    }

    pub fn system_fonts(&self) -> Rc<dyn IFontCollection> {
        if let Some(system_fonts) = &*self.system_fonts.borrow() {
            return system_fonts.clone();
        }

        let source = Uri::try_create(ASSETS, UriKind::Absolute).unwrap();

        let system_fonts: Rc<dyn IFontCollection> =
            Rc::new(EmbeddedFontCollection::new(FontManager::system_fonts_key(), source));

        *self.system_fonts.borrow_mut() = Some(system_fonts.clone());

        system_fonts
    }
}

impl IFontManagerImpl for CustomFontManagerImpl {
    fn get_default_font_family_name(&self) -> String {
        self.default_family_name.clone()
    }

    fn get_installed_font_family_names(&self, _check_for_updates: bool) -> Vec<String> {
        // Directly load from assets to avoid creating the full font collection
        let key = Uri::try_create(ASSETS, UriKind::Absolute).unwrap();

        let asset_loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();

        let font_assets = FontFamilyLoader::load_font_assets(&key);
        let mut names: Vec<String> = Vec::new();
        let mut seen = HashSet::new();

        for font_asset in font_assets {
            let Ok(mut stream) = asset_loader.open(&font_asset, None) else {
                // Ignore faulty assets
                continue;
            };

            let mut bytes = Vec::new();

            if stream.read_to_end(&mut bytes).is_err() {
                continue;
            }

            if let Some(typeface) = VelloTypeface::from_bytes(bytes, FontSimulations::None) {
                let family_name = typeface.family_name();

                if !family_name.is_empty() && seen.insert(family_name.to_lowercase()) {
                    names.push(family_name);
                }
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
        culture: Option<&CultureInfo>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        if let Some(glyph_typeface) = self.system_fonts().try_match_character(
            codepoint,
            font_style,
            font_weight,
            font_stretch,
            family_name,
            culture,
        ) {
            return Some(glyph_typeface.glyph_typeface().platform_typeface().clone());
        }

        // Upstream passes the language of the current culture; a culture
        // that was passed in is not used for the fonts of the system.
        let _ = culture;
        let culture = CultureInfo::get_culture_info(&self.bcp47[0]);

        self.font_manager.try_match_character(
            codepoint,
            font_style,
            font_weight,
            font_stretch,
            family_name,
            Some(&culture),
        )
    }

    fn try_create_glyph_typeface(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        if let Some(glyph_typeface) = self.system_fonts().try_get_glyph_typeface(family_name, style, weight, stretch) {
            return Some(glyph_typeface.platform_typeface().clone());
        }

        let _ = stretch;
        let typeface =
            self.font_manager.legacy_make_typeface(Some(family_name), style, weight, FontStretch::Normal)?;

        Some(typeface as Rc<dyn IPlatformTypeface>)
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
        if let Some(family_typefaces) = self.system_fonts().try_get_family_typefaces(family_name) {
            return Some(family_typefaces);
        }

        self.font_manager.try_get_family_typefaces(family_name)
    }

    fn dispose(&self) {
        // The collection holds this font manager (the font manager of the
        // locator it was created in): releasing it here breaks the cycle
        // that the garbage collector takes care of upstream.
        if let Some(system_fonts) = self.system_fonts.borrow_mut().take() {
            system_fonts.dispose();
        }
    }
}
