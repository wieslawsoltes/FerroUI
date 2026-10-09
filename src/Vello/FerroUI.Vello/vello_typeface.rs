use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::{FontSimulations, FontStretch, FontStyle, FontWeight, IFontMemory, IPlatformTypeface};
use ferroui_base::utilities::ReadOnlyMemory;
use kurbo::{Affine, BezPath, Diagonal2, Join};
use peniko::{Blob, FontData};
use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::tables::os2::SelectionFlags;
use skrifa::raw::types::Tag;
use skrifa::raw::TableProvider;
use skrifa::string::StringId;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use std::any::Any;
use std::io::{Cursor, Read};
use std::rc::Rc;
use std::sync::Arc;

/// The slant of the oblique simulation: the horizontal shear of a glyph in
/// a space of which y points down, as the Skia backend sets it on its font.
pub const OBLIQUE_SKEW: f64 = -0.3;

/// The width of the outline the bold simulation adds around a glyph, for an
/// em size, in the units of the em size.
///
/// This is the rule of Skia's fake bold, which the Skia backend uses: a
/// twenty-fourth of the em size at 9 and below, a thirty-second at 36 and
/// above, interpolated between; the outline is stroked with that width, so
/// a glyph grows by half of it on every side.
pub fn bold_simulation_outline_width(em_size: f64) -> f64 {
    const KEYS: (f64, f64) = (9.0, 36.0);
    const VALUES: (f64, f64) = (1.0 / 24.0, 1.0 / 32.0);

    let scale = if em_size <= KEYS.0 {
        VALUES.0
    } else if em_size >= KEYS.1 {
        VALUES.1
    } else {
        VALUES.0 + (VALUES.1 - VALUES.0) * (em_size - KEYS.0) / (KEYS.1 - KEYS.0)
    };

    em_size * scale
}

/// The font of a typeface as the renderers and the glyph runs use it: the
/// bytes of the font file, the font in it, the position in the variation
/// space and the simulations. It is what a glyph run keeps of its typeface,
/// and unlike the typeface it may be sent to the thread that draws.
#[derive(Debug)]
pub struct VelloFontFace {
    data: FontData,
    normalized_coords: Vec<NormalizedCoord>,
    normalized_coord_bits: Vec<i16>,
    units_per_em: u16,
    has_head_table: bool,
    font_simulations: FontSimulations,
}

impl VelloFontFace {
    /// Whether the renderers can draw glyphs of the font.
    ///
    /// They read the em square from the `head` table and fail without one.
    /// A font without it is a bitmap font of Apple (`bhed`, `bdat`), of
    /// which no renderer of the Vello project draws the glyphs: such a font
    /// is a typeface with metrics, and its glyph runs draw nothing.
    pub fn is_drawable(&self) -> bool {
        self.has_head_table
    }

    /// The font file and the index of the font in it, as the renderers take
    /// them.
    pub fn data(&self) -> &FontData {
        &self.data
    }

    /// The position in the variation space of the font, a coordinate for
    /// each axis in the order of the font; empty for the default instance.
    pub fn normalized_coords(&self) -> &[NormalizedCoord] {
        &self.normalized_coords
    }

    /// The position in the variation space as the renderers take it: the
    /// coordinates as fixed-point numbers of 14 fractional bits.
    pub fn normalized_coord_bits(&self) -> &[i16] {
        &self.normalized_coord_bits
    }

    /// The units of the em square of the font.
    pub fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    /// The simulations that are applied when the font is drawn.
    pub fn font_simulations(&self) -> FontSimulations {
        self.font_simulations
    }

    /// The tables of the font.
    ///
    /// # Panics
    /// Never for a face of a typeface: the font was read when the typeface
    /// was created.
    pub fn font_ref(&self) -> FontRef<'_> {
        FontRef::from_index(self.data.data.data(), self.data.index)
            .unwrap_or_else(|_| panic!("The font data of a typeface is a font"))
    }

    /// The outline of a glyph at an em size, in a space of which y points
    /// down and the origin is the origin of the glyph on the baseline,
    /// without hinting and with the simulations of the face applied: the
    /// bold simulation widens the outline, the oblique one shears it.
    ///
    /// Returns `None` for a glyph the font does not have or that has no
    /// outline (a bitmap glyph); a glyph without contours (a space) is an
    /// empty path.
    pub fn glyph_path(&self, glyph_index: u16, em_size: f64) -> Option<BezPath> {
        self.glyph_path_with(glyph_index, em_size, self.font_simulations)
    }

    fn glyph_path_with(&self, glyph_index: u16, em_size: f64, font_simulations: FontSimulations) -> Option<BezPath> {
        if !(em_size.is_finite() && em_size > 0.0) {
            return None;
        }

        let font = self.font_ref();
        let glyph = font.outline_glyphs().get(GlyphId::new(glyph_index as u32))?;

        // The outline in the units of the font, scaled afterwards: a glyph
        // of a size is the same curve, and the result is not rounded to the
        // single precision of a size.
        let mut pen = PathPen(BezPath::new());
        let settings = DrawSettings::unhinted(Size::unscaled(), LocationRef::new(&self.normalized_coords));
        glyph.draw(settings, &mut pen).ok()?;

        let scale = em_size / self.units_per_em.max(1) as f64;
        let mut path = Affine::scale_non_uniform(scale, -scale) * pen.0;

        if font_simulations.contains(FontSimulations::Bold) && !path.elements().is_empty() {
            let amount = bold_simulation_outline_width(em_size) / 2.0;
            path = kurbo::expand_path(&path, Diagonal2::new(amount, amount), Join::Miter, 4.0, 0.01);
        }

        if font_simulations.contains(FontSimulations::Oblique) {
            path = Affine::skew(OBLIQUE_SKEW, 0.0) * path;
        }

        Some(path)
    }
}

/// Collects the outline of a glyph as a path.
struct PathPen(BezPath);

impl OutlinePen for PathPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to((x as f64, y as f64));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to((x as f64, y as f64));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.quad_to((cx0 as f64, cy0 as f64), (x as f64, y as f64));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.curve_to((cx0 as f64, cy0 as f64), (cx1 as f64, cy1 as f64), (x as f64, y as f64));
    }

    fn close(&mut self) {
        self.0.close_path();
    }
}

/// A platform typeface over the bytes of a font file.
///
/// The Skia backend wraps a typeface of Skia, which reads the font; here
/// the font is read with `skrifa`: its identity (family name, weight, style,
/// stretch) from the `name`, `OS/2` and `head` tables, its tables from the
/// table directory, the outlines of its glyphs by the scaler of `skrifa`.
/// The metrics of the glyph typeface are the base library's own, from the
/// tables this typeface serves, and the shaper reads the same tables.
pub struct VelloTypeface {
    face: Arc<VelloFontFace>,
    family_name: String,
    font_simulations: FontSimulations,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
}

impl VelloTypeface {
    /// Creates the typeface of the first font of a font file, applying the
    /// given style simulations when it is drawn.
    ///
    /// Returns `None` when the bytes are not a font.
    pub fn from_bytes(bytes: Vec<u8>, font_simulations: FontSimulations) -> Option<Rc<Self>> {
        Self::new(Blob::new(Arc::new(bytes)), 0, font_simulations, &[])
    }

    /// Creates the typeface of a font of a font file or collection.
    ///
    /// `variation_settings` are values of axes of a variable font by their
    /// tags, in the units of the axes (a weight of 700 for `wght`); an axis
    /// the font does not have is ignored and an axis that is not named
    /// keeps its default.
    ///
    /// Returns `None` when the bytes are not a font or the collection has no
    /// font of that index.
    pub fn new(
        data: Blob<u8>,
        index: u32,
        font_simulations: FontSimulations,
        variation_settings: &[(&str, f32)],
    ) -> Option<Rc<Self>> {
        let font = FontRef::from_index(data.data(), index).ok()?;

        // The em square: of the `head` table, or of the table a bitmap font
        // of Apple has in its place, which has the same layout.
        let head = font.head().ok();
        let units_per_em = match &head {
            Some(head) => head.units_per_em(),
            None => font
                .table_data(Tag::new(b"bhed"))
                .and_then(|table| table.as_bytes().get(18..20).map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]])))
                .unwrap_or(1000),
        };

        let settings: Vec<(Tag, f32)> = variation_settings
            .iter()
            .filter_map(|(tag, value)| Some((Tag::new_checked(tag.as_bytes()).ok()?, *value)))
            .collect();

        let axes = font.axes();
        let normalized_coords = if settings.iter().any(|(tag, _)| axes.get_by_tag(*tag).is_some()) {
            axes.location(settings.iter().copied()).coords().to_vec()
        } else {
            Vec::new()
        };

        let os2 = font.os2().ok();
        let has_head_table = head.is_some();
        let mac_style = head.as_ref().map(|head| head.mac_style());

        let mut weight = match &os2 {
            Some(os2) => FontWeight(os2.us_weight_class() as i32),
            None if mac_style.is_some_and(|style| style.bits() & 1 != 0) => FontWeight::Bold,
            None => FontWeight::Normal,
        };

        let mut style = match &os2 {
            Some(os2) if os2.fs_selection().contains(SelectionFlags::OBLIQUE) => FontStyle::Oblique,
            Some(os2) if os2.fs_selection().contains(SelectionFlags::ITALIC) => FontStyle::Italic,
            None if mac_style.is_some_and(|style| style.bits() & 2 != 0) => FontStyle::Italic,
            _ => FontStyle::Normal,
        };

        let mut stretch = os2
            .as_ref()
            .and_then(|os2| FontStretch::from_i32(os2.us_width_class() as i32))
            .unwrap_or(FontStretch::Normal);

        // An instance of a variable font is the weight and the width it was
        // asked for.
        for (tag, value) in &settings {
            if axes.get_by_tag(*tag).is_none() {
                continue;
            }

            if *tag == Tag::new(b"wght") {
                weight = FontWeight(value.round() as i32);
            } else if *tag == Tag::new(b"wdth") {
                stretch = stretch_of_percentage(*value);
            } else if (*tag == Tag::new(b"ital") && *value >= 0.5) || (*tag == Tag::new(b"slnt") && *value != 0.0) {
                if style == FontStyle::Normal {
                    style = FontStyle::Italic;
                }
            }
        }

        // The family as a user of the font names it: the typographic family
        // where the font has one, else the family of its style group.
        let family_name = [StringId::TYPOGRAPHIC_FAMILY_NAME, StringId::FAMILY_NAME]
            .into_iter()
            .find_map(|id| font.localized_strings(id).english_or_first())
            .map(|name| name.chars().collect::<String>())
            .unwrap_or_default();

        Some(Rc::new(Self {
            face: Arc::new(VelloFontFace {
                data: FontData::new(data, index),
                normalized_coord_bits: normalized_coords.iter().map(|coord| coord.to_bits()).collect(),
                normalized_coords,
                units_per_em,
                has_head_table,
                font_simulations,
            }),
            family_name,
            font_simulations,
            weight,
            style,
            stretch,
        }))
    }

    /// The font as the renderers and the glyph runs use it.
    pub fn face(&self) -> &Arc<VelloFontFace> {
        &self.face
    }

    /// Recovers the backend typeface behind a platform typeface.
    ///
    /// Returns `None` for typefaces created by another backend.
    pub fn try_get(platform_typeface: &dyn IPlatformTypeface) -> Option<&VelloTypeface> {
        platform_typeface.as_any().downcast_ref::<VelloTypeface>()
    }
}

/// The stretch of a width in percent of the normal width, as the `wdth`
/// axis of a variable font has it: the nearest of the nine classes.
fn stretch_of_percentage(percentage: f32) -> FontStretch {
    const CLASSES: [f32; 9] = [50.0, 62.5, 75.0, 87.5, 100.0, 112.5, 125.0, 150.0, 200.0];

    let mut nearest = 4;
    for (index, class) in CLASSES.iter().enumerate() {
        if (class - percentage).abs() < (CLASSES[nearest] - percentage).abs() {
            nearest = index;
        }
    }

    FontStretch::from_i32(nearest as i32 + 1).unwrap_or(FontStretch::Normal)
}

impl IFontMemory for VelloTypeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        let data = self.face.font_ref().table_data(Tag::from_u32(tag.value()))?;
        let bytes = data.as_bytes();

        if bytes.is_empty() {
            return None;
        }

        Some(ReadOnlyMemory::from_slice(bytes))
    }

    fn dispose(&self) {
        // The font data is reference counted and released with the last
        // handle to it: this object, a glyph run, a scene.
    }
}

impl IPlatformTypeface for VelloTypeface {
    fn family_name(&self) -> String {
        self.family_name.clone()
    }

    fn weight(&self) -> FontWeight {
        self.weight
    }

    fn style(&self) -> FontStyle {
        self.style
    }

    fn stretch(&self) -> FontStretch {
        self.stretch
    }

    fn font_simulations(&self) -> FontSimulations {
        self.font_simulations
    }

    fn try_get_stream(&self) -> Option<Box<dyn Read>> {
        let bytes = self.face.data.data.data();

        if bytes.is_empty() {
            return None;
        }

        Some(Box::new(Cursor::new(bytes.to_vec())))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
