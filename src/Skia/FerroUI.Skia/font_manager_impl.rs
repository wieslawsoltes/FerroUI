use crate::skia_sharp_extensions::{font_style_to_slant, slant_to_font_style};
use crate::skia_typeface::SkiaTypeface;
use ferroui_base::media::{
    FontSimulations, FontStretch, FontStyle, FontWeight, IPlatformTypeface, Typeface,
};
use ferroui_base::media::fonts::tables::{FontSelectionFlags, OS2Table};
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::utilities::{current_ui_locale, Locale};
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
        culture: Option<&Locale>,
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
                current_culture = current_ui_locale();
                &current_culture
            }
        };
        // The BCP-47 tag of the locale; empty for the invariant culture, the unknown locale.
        let culture_name = if *culture == Locale::UNKNOWN { String::new() } else { culture.to_string() };

        self.sk_font_manager.borrow().match_family_style_character(
            family_name.unwrap_or(""),
            sk_font_style,
            &[culture_name.as_str()],
            codepoint,
        )
    }
}

/// Whether the platform font manager simulates bold and slant inside the typefaces it matches:
/// DirectWrite and fontconfig do, Core Text never does, and its traits can disagree with the
/// OS/2 table (upstream 80659c4c3e).
const PLATFORM_SIMULATES: bool = !cfg!(any(target_os = "macos", target_os = "ios"));

/// The weight class of an OS/2 table on the scale of 1 to 1000: the legacy classes 1 to 9 are
/// hundreds.
fn os2_weight(weight_class: u16) -> i32 {
    match weight_class {
        1..=9 => i32::from(weight_class) * 100,
        other => i32::from(other),
    }
}

/// The simulations the platform applied inside `typeface`: the weight or the slant it reports
/// that its font file does not have. A variable face never reports a platform bold (its OS/2
/// table describes the default instance), and its slant is real when it has a `slnt` or `ital`
/// axis.
fn platform_simulations(typeface: &skia_safe::Typeface) -> FontSimulations {
    if !PLATFORM_SIMULATES {
        return FontSimulations::None;
    }
    let style = typeface.font_style();
    let slanted = style.slant() != skia_safe::font_style::Slant::Upright;
    let bold = *style.weight() >= 600;
    if !slanted && !bold {
        return FontSimulations::None;
    }
    let wrapper = SkiaTypeface::new(typeface.clone(), FontSimulations::None);
    let Ok(Some(os2)) = OS2Table::try_load(&*wrapper) else { return FontSimulations::None };
    let axes = typeface.variation_design_parameters().unwrap_or_default();
    let has_axis = |tag: &[u8; 4]| axes.iter().any(|axis| *axis.tag == u32::from_be_bytes(*tag));
    let mut simulations = FontSimulations::None;
    if bold && axes.is_empty() && os2_weight(os2.weight_class) < 600 {
        simulations |= FontSimulations::Bold;
    }
    let designed_slant = os2.selection.intersects(FontSelectionFlags::ITALIC | FontSelectionFlags::OBLIQUE)
        || has_axis(b"slnt")
        || has_axis(b"ital");
    if slanted && !designed_slant {
        simulations |= FontSimulations::Oblique;
    }
    simulations
}

/// The platform typeface of a face the platform matched to a request, for a family match and
/// for a character fallback alike (upstream 80659c4c3e): the simulations the platform applied
/// are reported, and a missing bold (a face below 600 for a request of 600 or more, the rule of
/// CSS) or a missing slant (an upright face for an italic or oblique request) is simulated.
fn matched_typeface(typeface: skia_safe::Typeface, style: FontStyle, weight: FontWeight) -> Rc<dyn IPlatformTypeface> {
    let platform = platform_simulations(&typeface);
    let face = typeface.font_style();
    let mut simulations = FontSimulations::None;
    if weight.0 >= 600 && *face.weight() < 600 {
        simulations |= FontSimulations::Bold;
    }
    if style != FontStyle::Normal && face.slant() == skia_safe::font_style::Slant::Upright {
        simulations |= FontSimulations::Oblique;
    }
    SkiaTypeface::matched(typeface, simulations, platform, Some((weight, style)))
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
        culture: Option<&Locale>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let sk_typeface = self.try_match_character_typeface(
            codepoint,
            font_style,
            font_weight,
            font_stretch,
            family_name,
            culture,
        )?;

        // The fallback face keeps the simulations of the request (upstream 80659c4c3e).
        Some(matched_typeface(sk_typeface, font_style, font_weight))
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

        Some(matched_typeface(sk_typeface, style, weight))
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
