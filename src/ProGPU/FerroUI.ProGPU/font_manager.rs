//! The font manager of the backend: the fonts of the system as the catalog of the engine finds
//! them, and faces made of font bytes an application brings.

use crate::font::{CatalogFace, FaceStyle, FontCatalog, FontFace};
use crate::typeface::ProGpuTypeface;
use ferroui_base::media::{FontSimulations, FontStretch, FontStyle, FontWeight, IPlatformTypeface, Typeface};
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::utilities::{current_ui_locale, Locale};
use std::cell::OnceCell;
use std::io::Read;
use std::rc::Rc;

/// The families tried, in this order, as the default family of the system.
const DEFAULT_FAMILIES: [&str; 6] = ["Helvetica", "Arial", "Segoe UI", "DejaVu Sans", "Liberation Sans", "Noto Sans"];

/// The font manager over the catalog of the engine.
#[derive(Default)]
pub struct ProGpuFontManager {
    families: OnceCell<Vec<String>>,
}

fn face_style(style: FontStyle, weight: FontWeight, stretch: FontStretch) -> FaceStyle {
    FaceStyle { weight: weight.0.clamp(1, 1000), width: stretch as i32, italic: style != FontStyle::Normal }
}

impl ProGpuFontManager {
    /// The font manager of the system fonts.
    pub fn new() -> Self {
        Self::default()
    }

    fn catalog_faces(catalog: &FontCatalog) -> impl Iterator<Item = CatalogFace> + '_ {
        (0..catalog.face_count()).filter_map(|index| catalog.face(index))
    }

    /// The typeface of a face of the catalog at `style`, with the simulations of what the face
    /// cannot give.
    fn typeface_of(catalog: &FontCatalog, entry: &CatalogFace, style: FaceStyle, simulate: bool) -> Option<Rc<dyn IPlatformTypeface>> {
        let face = catalog.load(entry.index)?;
        let (_, applied) = face.resolve_style(style);
        let mut font_simulations = FontSimulations::None;
        if simulate {
            if style.weight >= 600 && face.weight() < 600 && !applied.weight() {
                font_simulations |= FontSimulations::Bold;
            }
            if style.italic && !face.italic() && !applied.slant() {
                font_simulations |= FontSimulations::Oblique;
            }
        }
        Some(ProGpuTypeface::new(face, Some(entry.family.clone()), Some(style), font_simulations))
    }
}

impl IFontManagerImpl for ProGpuFontManager {
    fn get_default_font_family_name(&self) -> String {
        let Some(catalog) = FontCatalog::system() else {
            return String::new();
        };
        let regular = FaceStyle { weight: 400, width: 5, italic: false };
        DEFAULT_FAMILIES
            .iter()
            .find_map(|family| catalog.match_family(family, regular))
            .or_else(|| catalog.face(0))
            .map(|face| face.family)
            .unwrap_or_default()
    }

    fn get_installed_font_family_names(&self, _check_for_updates: bool) -> Vec<String> {
        // The catalog is discovered once for the process: fonts installed after are not seen.
        self.families
            .get_or_init(|| {
                let Some(catalog) = FontCatalog::system() else {
                    return Vec::new();
                };
                let mut families: Vec<String> = Self::catalog_faces(catalog).map(|face| face.family).filter(|family| !family.is_empty()).collect();
                families.sort();
                families.dedup();
                families
            })
            .clone()
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
        let catalog = FontCatalog::system()?;
        let code_point = u32::try_from(codepoint).ok()?;
        let current_culture;
        let culture = match culture {
            Some(culture) => culture,
            None => {
                current_culture = current_ui_locale();
                &current_culture
            }
        };
        // The BCP-47 tag of the locale; empty for the unknown locale.
        let language = if *culture == Locale::UNKNOWN { String::new() } else { culture.to_string() };
        let style = face_style(font_style, font_weight, font_stretch);
        let entry = catalog.match_character(family_name.unwrap_or(""), style, &language, code_point)?;
        Self::typeface_of(catalog, &entry, style, false)
    }

    fn try_create_glyph_typeface(&self, family_name: &str, style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Option<Rc<dyn IPlatformTypeface>> {
        let catalog = FontCatalog::system()?;
        let style = face_style(style, weight, stretch);
        let entry = catalog.match_family(family_name, style)?;
        Self::typeface_of(catalog, &entry, style, true)
    }

    fn try_create_glyph_typeface_from_stream(&self, stream: &mut dyn Read, font_simulations: FontSimulations) -> Option<Rc<dyn IPlatformTypeface>> {
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).ok()?;
        let face = FontFace::from_bytes(&bytes, 0)?;
        Some(ProGpuTypeface::new(face, None, None, font_simulations))
    }

    fn try_get_family_typefaces(&self, family_name: &str) -> Option<Vec<Typeface>> {
        let catalog = FontCatalog::system()?;
        let typefaces: Vec<Typeface> = Self::catalog_faces(catalog)
            .filter(|face| face.family.eq_ignore_ascii_case(family_name))
            .map(|face| {
                Typeface::from_name_with_style(
                    family_name,
                    if face.italic { FontStyle::Italic } else { FontStyle::Normal },
                    FontWeight(i32::from(face.weight)),
                    FontStretch::from_i32(i32::from(face.width)).unwrap_or(FontStretch::Normal),
                )
            })
            .collect();
        (!typefaces.is_empty()).then_some(typefaces)
    }
}
