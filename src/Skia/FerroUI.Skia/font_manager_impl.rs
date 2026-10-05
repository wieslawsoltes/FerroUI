use crate::skia_sharp_extensions::{font_style_to_slant, slant_to_font_style};
use crate::skia_typeface::SkiaTypeface;
use ferroui_base::media::{
    FontSimulations, FontStretch, FontStyle, FontWeight, IPlatformTypeface, Typeface,
};
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::utilities::CultureInfo;
use skia_safe::font_style::{Weight, Width};
use skia_safe::{Data, FontMgr};
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;

/// The Skia font manager: system fonts through Skia's default font manager.
pub struct FontManagerImpl {
    sk_font_manager: RefCell<FontMgr>,
}

impl FontManagerImpl {
    /// Creates a font manager over the system's fonts.
    pub fn new() -> Self {
        Self { sk_font_manager: RefCell::new(FontMgr::new()) }
    }

    fn try_match_character_typeface(
        &self,
        codepoint: i32,
        font_style: FontStyle,
        font_weight: FontWeight,
        font_stretch: FontStretch,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<skia_safe::Typeface> {
        let is_normal_stretch = font_stretch == FontStretch::Normal;

        let sk_font_style = if font_weight == FontWeight::Normal && font_style == FontStyle::Normal && is_normal_stretch
        {
            skia_safe::FontStyle::normal()
        } else if font_weight == FontWeight::Normal && font_style == FontStyle::Italic && is_normal_stretch {
            skia_safe::FontStyle::italic()
        } else if font_weight == FontWeight::Bold && font_style == FontStyle::Normal && is_normal_stretch {
            skia_safe::FontStyle::bold()
        } else if font_weight == FontWeight::Bold && font_style == FontStyle::Italic && is_normal_stretch {
            skia_safe::FontStyle::bold_italic()
        } else {
            skia_safe::FontStyle::new(
                Weight::from(font_weight.0),
                Width::from(font_stretch as i32),
                font_style_to_slant(font_style),
            )
        };

        let current_culture;
        let culture = match culture {
            Some(culture) => culture,
            None => {
                current_culture = CultureInfo::current_ui_culture();
                &current_culture
            }
        };

        self.sk_font_manager.borrow().match_family_style_character(
            family_name.unwrap_or(""),
            sk_font_style,
            &[culture.name()],
            codepoint,
        )
    }
}

impl Default for FontManagerImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl IFontManagerImpl for FontManagerImpl {
    fn get_default_font_family_name(&self) -> String {
        self.sk_font_manager
            .borrow()
            .legacy_make_typeface(None, skia_safe::FontStyle::normal())
            .map(|typeface| typeface.family_name())
            .unwrap_or_default()
    }

    fn get_installed_font_family_names(&self, check_for_updates: bool) -> Vec<String> {
        if check_for_updates {
            *self.sk_font_manager.borrow_mut() = FontMgr::new();
        }

        self.sk_font_manager.borrow().family_names().collect()
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
        let sk_typeface = self.try_match_character_typeface(
            codepoint,
            font_style,
            font_weight,
            font_stretch,
            family_name,
            culture,
        )?;

        Some(SkiaTypeface::new(sk_typeface, FontSimulations::None))
    }

    fn try_create_glyph_typeface(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let font_style =
            skia_safe::FontStyle::new(Weight::from(weight.0), Width::from(stretch as i32), font_style_to_slant(style));

        let sk_typeface = self.sk_font_manager.borrow().match_family_style(family_name, font_style)?;

        let mut font_simulations = FontSimulations::None;

        if weight.0 >= 600 && !sk_typeface.is_bold() {
            font_simulations |= FontSimulations::Bold;
        }

        if style == FontStyle::Italic && !sk_typeface.is_italic() {
            font_simulations |= FontSimulations::Oblique;
        }

        Some(SkiaTypeface::new(sk_typeface, font_simulations))
    }

    fn try_create_glyph_typeface_from_stream(
        &self,
        stream: &mut dyn Read,
        font_simulations: FontSimulations,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).ok()?;

        let sk_typeface = self.sk_font_manager.borrow().new_from_data(Data::new_copy(&bytes), None)?;

        Some(SkiaTypeface::new(sk_typeface, font_simulations))
    }

    fn try_get_family_typefaces(&self, family_name: &str) -> Option<Vec<Typeface>> {
        let mut set = self.sk_font_manager.borrow().match_family(family_name);

        let count = set.count();
        if count == 0 {
            return None;
        }

        let mut typefaces = Vec::with_capacity(count);

        for index in 0..count {
            let (font_style, _) = set.style(index);
            typefaces.push(Typeface::from_name_with_style(
                family_name,
                slant_to_font_style(font_style.slant()),
                FontWeight(*font_style.weight()),
                FontStretch::from_i32(*font_style.width()).unwrap_or(FontStretch::Normal),
            ));
        }

        Some(typefaces)
    }
}
