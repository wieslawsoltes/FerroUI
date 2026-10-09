//! Port of upstream's `tests/*.UnitTests/TestFontManager.cs`, the font
//! manager of upstream's `TestServices` presets.
//!
//! Upstream creates its typefaces as `HeadlessPlatformTypeface` of the
//! headless platform, which the port does not have; the test platform
//! typeface of the base crate is the same thing (the font file in memory,
//! its identity read from its tables).

use ferroui_base::media::fonts::testing::TestPlatformTypeface;
use ferroui_base::media::{FontSimulations, FontStretch, FontStyle, FontWeight, IPlatformTypeface, Typeface};
use ferroui_base::platform::{IAssetLoader, IFontManagerImpl};
use ferroui_base::utilities::{CultureInfo, Uri};
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::cell::Cell;
use std::io::Read;
use std::rc::Rc;

pub struct TestFontManager {
    inter_font_uri: &'static str,
    default_family_name: &'static str,
    try_create_glyph_typeface_count: Cell<i32>,
}

impl TestFontManager {
    pub fn new() -> Self {
        Self {
            inter_font_uri: "ferres://FerroUI.Fonts.Inter/Assets/Inter-Regular.ttf",
            default_family_name: "ferres://FerroUI.Fonts.Inter/Assets#Inter",
            try_create_glyph_typeface_count: Cell::new(0),
        }
    }

    pub fn try_create_glyph_typeface_count(&self) -> i32 {
        self.try_create_glyph_typeface_count.get()
    }
}

impl IFontManagerImpl for TestFontManager {
    fn get_default_font_family_name(&self) -> String {
        self.default_family_name.to_owned()
    }

    fn get_installed_font_family_names(&self, _check_for_updates: bool) -> Vec<String> {
        vec![self.default_family_name.to_owned()]
    }

    fn try_match_character(
        &self,
        _codepoint: i32,
        _font_style: FontStyle,
        _font_weight: FontWeight,
        _font_stretch: FontStretch,
        _family_name: Option<&str>,
        _culture: Option<&CultureInfo>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        None
    }

    fn try_create_glyph_typeface(
        &self,
        family_name: &str,
        _style: FontStyle,
        _weight: FontWeight,
        _stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let mut platform_typeface: Option<Rc<dyn IPlatformTypeface>> = None;

        if family_name == "MyFont" {
            let asset_loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();

            let mut stream = asset_loader
                .open(&Uri::absolute(self.inter_font_uri).unwrap(), None)
                .expect("the Inter font is an asset");

            platform_typeface = TestPlatformTypeface::from_stream(&mut *stream, FontSimulations::None)
                .map(|typeface| typeface as Rc<dyn IPlatformTypeface>);
        }

        self.try_create_glyph_typeface_count.set(self.try_create_glyph_typeface_count.get() + 1);

        platform_typeface
    }

    fn try_create_glyph_typeface_from_stream(
        &self,
        stream: &mut dyn Read,
        _font_simulations: FontSimulations,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let platform_typeface = TestPlatformTypeface::from_stream(stream, FontSimulations::None)
            .map(|typeface| typeface as Rc<dyn IPlatformTypeface>);

        self.try_create_glyph_typeface_count.set(self.try_create_glyph_typeface_count.get() + 1);

        platform_typeface
    }

    fn try_get_family_typefaces(&self, _family_name: &str) -> Option<Vec<Typeface>> {
        None
    }
}
