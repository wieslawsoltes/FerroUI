use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::media::fonts::tables::cmap::{CharacterToGlyphMap, CmapFormat, CmapTable};
use crate::media::fonts::tables::glyf::GlyfTable;
use crate::media::fonts::tables::metrics::{
    HorizontalGlyphMetric, HorizontalMetricsTable, VerticalGlyphMetric, VerticalMetricsTable,
};
use crate::media::fonts::tables::name::NameTable;
use crate::media::fonts::tables::{
    FeatureListTable, FontSelectionFlags, FontTableError, HeadFlags, HeadTable, HorizontalHeaderTable, KnownNameIds,
    MacStyleFlags, MaxpTable, MetaTable, OS2Table, PanoseFamilyKind, PanoseProportion, PanoseWeight, PlatformID,
    PostTable, ScriptListTable, VerticalHeaderTable,
};
use crate::media::fonts::{FontCodePageCoverage, FontFallbackScriptHints, OpenTypeTag};
use crate::media::text_formatting::unicode::Script;
use crate::media::{
    FontMetrics, FontSimulations, FontStretch, FontStyle, FontWeight, GlyphBounds, GlyphMetrics, IFontMemory,
    IPlatformTypeface, ITextShaperTypeface, UnicodeRange, UnicodeRangeSegment,
};
use crate::platform::ITextShaperImpl;
use crate::utilities::CultureInfo;
use crate::{FerroLocator, LocatorExtensions};

/// LCID of the invariant culture as stored in name records.
const INVARIANT_LCID: u16 = 0x007F;

/// Represents a glyph typeface, providing access to font metrics, glyph
/// mappings, and other font-related properties.
///
/// The `GlyphTypeface` class is used to encapsulate font data, including
/// metrics, character-to-glyph mappings, and supported OpenType features. It
/// supports platform-specific typefaces and applies optional font simulations
/// such as bold or oblique.
pub struct GlyphTypeface {
    platform_typeface: Rc<dyn IPlatformTypeface>,
    is_disposed: Cell<bool>,

    os2_table: Option<OS2Table>,
    cmap_table: CharacterToGlyphMap,
    hm_table: Option<HorizontalMetricsTable>,
    vm_table: Option<VerticalMetricsTable>,
    glyf_table: Option<GlyfTable>,

    design_languages: Vec<String>,
    supported_languages: Vec<String>,

    family_name: String,
    typographic_family_name: String,
    family_names: HashMap<CultureInfo, String>,
    face_names: HashMap<CultureInfo, String>,
    metrics: FontMetrics,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
    font_simulations: FontSimulations,
    glyph_count: i32,
    code_page_coverage: FontCodePageCoverage,
    is_last_resort: bool,

    supported_features: OnceCell<Vec<OpenTypeTag>>,
    text_shaper_typeface: RefCell<Option<Rc<dyn ITextShaperTypeface>>>,
    supported_unicode_range: OnceCell<UnicodeRange>,

    /// Lazily-built set of OpenType script tags the font declares in
    /// GSUB/GPOS, used by `can_shape_script`, and whether a present table
    /// could not be parsed (capability unknown). Parsing copies the layout
    /// tables, so it is deferred until a complex script is actually queried.
    shaping_script_tags: OnceCell<(HashSet<OpenTypeTag>, bool)>,
}

impl GlyphTypeface {
    /// Initializes a new instance of the `GlyphTypeface` class with the
    /// specified platform typeface and font simulations.
    ///
    /// This reads the font's metrics, names and character map from its
    /// OpenType tables. Fails when a required table (`cmap`, `maxp`) is
    /// missing or malformed.
    pub fn new(
        typeface: Rc<dyn IPlatformTypeface>,
        font_simulations: FontSimulations,
    ) -> Result<Rc<GlyphTypeface>, FontTableError> {
        let font: &dyn IFontMemory = &*typeface;

        let os2_table = OS2Table::try_load(font)?;
        let cmap_table = CmapTable::load(font)?;

        let (design_languages, supported_languages) = match MetaTable::try_load(font) {
            Some(meta_table) => (meta_table.design_languages().to_vec(), meta_table.supported_languages().to_vec()),
            None => (Vec::new(), Vec::new()),
        };

        let code_page_coverage = match &os2_table {
            Some(os2) if os2.version >= 1 => FontCodePageCoverage::from_bits_retain(
                os2.code_page_range1 as u64 | ((os2.code_page_range2 as u64) << 32),
            ),
            _ => FontCodePageCoverage::None,
        };

        let maxp_table = MaxpTable::load(font)?;

        let glyph_count = maxp_table.num_glyphs as i32;

        let hh_table = HorizontalHeaderTable::try_load(font)?;

        let hm_table = match &hh_table {
            Some(hh_table) => HorizontalMetricsTable::load(font, hh_table.number_of_h_metrics, glyph_count),
            None => None,
        };

        let vh_table = VerticalHeaderTable::try_load(font)?;

        let vm_table = match &vh_table {
            Some(vh_table) => VerticalMetricsTable::load(font, vh_table.number_of_v_metrics, glyph_count),
            None => None,
        };

        let mut ascent = 0i32;
        let mut descent = 0i32;
        let mut line_gap = 0i32;

        match &os2_table {
            Some(os2) if os2.selection.contains(FontSelectionFlags::USE_TYPO_METRICS) => {
                ascent = -(os2.typo_ascender as i32);
                descent = -(os2.typo_descender as i32);
                line_gap = os2.typo_line_gap as i32;
            }
            _ => {
                if let Some(hh_table) = &hh_table {
                    ascent = -(hh_table.ascender as i32);
                    descent = -(hh_table.descender as i32);
                    line_gap = hh_table.line_gap as i32;
                }
            }
        }

        if let Some(os2) = &os2_table {
            if ascent == 0 || descent == 0 {
                if os2.typo_ascender != 0 || os2.typo_descender != 0 {
                    ascent = -(os2.typo_ascender as i32);
                    descent = -(os2.typo_descender as i32);
                    line_gap = os2.typo_line_gap as i32;
                } else {
                    ascent = -(os2.win_ascent as i32);
                    descent = os2.win_descent as i32;
                }
            }
        }

        let head_table = HeadTable::try_load(font)?;

        // Load glyf table once and cache for reuse by glyph bounds and outlines.
        let glyf_table = match &head_table {
            Some(head_table) => GlyfTable::try_load(font, head_table, &maxp_table),
            None => None,
        };

        let is_last_resort = head_table
            .as_ref()
            .is_some_and(|head_table| head_table.flags.contains(HeadFlags::LastResortFont))
            || cmap_table.format() == CmapFormat::Format13;

        let post_table = PostTable::load(font);

        let is_fixed_pitch = post_table.is_fixed_pitch;
        let underline_offset = post_table.underline_position as i32;
        let underline_size = post_table.underline_thickness as i32;

        let design_em_height = Self::get_font_design_em_height(head_table.as_ref());

        let metrics = FontMetrics {
            design_em_height,
            ascent,
            descent,
            line_gap,
            underline_position: -underline_offset,
            underline_thickness: underline_size,
            strikethrough_position: os2_table.as_ref().map_or(0, |os2| -(os2.strikeout_position as i32)),
            strikethrough_thickness: os2_table.as_ref().map_or(0, |os2| os2.strikeout_size as i32),
            is_fixed_pitch,
        };

        let font_weight = Self::get_font_weight(os2_table.as_ref(), head_table.as_ref());

        let weight = if font_simulations.contains(FontSimulations::Bold) { FontWeight::Bold } else { font_weight };

        let font_style = Self::get_font_style(os2_table.as_ref(), head_table.as_ref(), &post_table);

        let style = if font_simulations.contains(FontSimulations::Oblique) { FontStyle::Italic } else { font_style };

        let stretch = Self::get_font_stretch(os2_table.as_ref());

        let name_table = NameTable::load(font);

        let family_name = match &name_table {
            Some(names) => names.font_family_name(INVARIANT_LCID),
            None => "unknown".to_owned(),
        };

        let typographic_family_name = match &name_table {
            Some(names) => names.get_name_by_id(INVARIANT_LCID, KnownNameIds::TypographicFamilyName),
            None => family_name.clone(),
        };

        let mut family_names = HashMap::new();
        let mut face_names = HashMap::new();

        match &name_table {
            Some(name_table) => {
                for name_record in name_table.iter() {
                    let names = if name_record.name_id() == KnownNameIds::FontFamilyName {
                        &mut family_names
                    } else if name_record.name_id() == KnownNameIds::FontSubfamilyName {
                        &mut face_names
                    } else {
                        continue;
                    };

                    if name_record.platform() != PlatformID::Windows || name_record.language_id() == 0 {
                        continue;
                    }

                    let culture = Self::get_culture(name_record.language_id() as i32);

                    names.entry(culture).or_insert_with(|| name_record.get_value());
                }
            }
            None => {
                family_names.insert(CultureInfo::invariant_culture(), family_name.clone());
                face_names.insert(CultureInfo::invariant_culture(), weight.to_string());
            }
        }

        Ok(Rc::new(GlyphTypeface {
            platform_typeface: typeface,
            is_disposed: Cell::new(false),
            os2_table,
            cmap_table,
            hm_table,
            vm_table,
            glyf_table,
            design_languages,
            supported_languages,
            family_name,
            typographic_family_name,
            family_names,
            face_names,
            metrics,
            weight,
            style,
            stretch,
            font_simulations,
            glyph_count,
            code_page_coverage,
            is_last_resort,
            supported_features: OnceCell::new(),
            text_shaper_typeface: RefCell::new(None),
            supported_unicode_range: OnceCell::new(),
            shaping_script_tags: OnceCell::new(),
        }))
    }

    fn get_culture(lcid: i32) -> CultureInfo {
        if lcid == u16::MAX as i32 {
            return CultureInfo::invariant_culture();
        }

        CultureInfo::get_culture_info_by_lcid(lcid).unwrap_or_else(CultureInfo::invariant_culture)
    }

    fn get_font_design_em_height(head_table: Option<&HeadTable>) -> u16 {
        let units_per_em = head_table.map_or(0, |head_table| head_table.units_per_em);

        // Bitmap fonts may specify 0 or miss the head table completely.
        // Use 2048 as sensible default (used by most fonts).
        if units_per_em == 0 {
            2048
        } else {
            units_per_em
        }
    }

    /// Creates a glyph typeface; a font whose tables cannot be read is logged
    /// and gives `None`.
    ///
    /// Internal upstream, where the Skia unit tests see it; public here so
    /// that the tests of the Skia crate reach it.
    pub fn try_create(
        typeface: Rc<dyn IPlatformTypeface>,
        font_simulations: FontSimulations,
    ) -> Option<Rc<GlyphTypeface>> {
        let family_name = typeface.family_name();

        match GlyphTypeface::new(typeface, font_simulations) {
            Ok(glyph_typeface) => Some(glyph_typeface),
            Err(error) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::FONTS) {
                    logger.log_with_values(
                        None,
                        "Could not create glyph typeface from platform typeface named {FamilyName} with simulations {Simulations}: {Exception}",
                        &[&family_name, &format!("{font_simulations:?}"), &error],
                    );
                }
                None
            }
        }
    }

    /// Gets the family name of the font.
    pub fn family_name(&self) -> &str {
        &self.family_name
    }

    /// Gets the typographic family name of the font.
    pub fn typographic_family_name(&self) -> &str {
        &self.typographic_family_name
    }

    /// Gets a mapping of culture-specific family names.
    pub fn family_names(&self) -> &HashMap<CultureInfo, String> {
        &self.family_names
    }

    /// Gets a mapping of culture-specific face names.
    pub fn face_names(&self) -> &HashMap<CultureInfo, String> {
        &self.face_names
    }

    /// Gets a mapping of Unicode code points to glyph indices.
    pub fn character_to_glyph_map(&self) -> &CharacterToGlyphMap {
        &self.cmap_table
    }

    /// Gets the font metrics associated with this font.
    pub fn metrics(&self) -> FontMetrics {
        self.metrics
    }

    /// Gets the font weight.
    pub fn weight(&self) -> FontWeight {
        self.weight
    }

    /// Gets the font style.
    pub fn style(&self) -> FontStyle {
        self.style
    }

    /// Gets the font stretch.
    pub fn stretch(&self) -> FontStretch {
        self.stretch
    }

    /// Gets the font simulation settings applied to the glyph typeface.
    pub fn font_simulations(&self) -> FontSimulations {
        self.font_simulations
    }

    /// Gets the number of glyphs held by this font.
    pub fn glyph_count(&self) -> i32 {
        self.glyph_count
    }

    /// Gets the supported OpenType features (GPOS first, then GSUB, without duplicates).
    pub fn supported_features(&self) -> &[OpenTypeTag] {
        self.supported_features.get_or_init(|| self.load_supported_features())
    }

    /// Gets the Unicode ranges covered by the font's character map.
    pub fn supported_unicode_range(&self) -> &UnicodeRange {
        self.supported_unicode_range.get_or_init(|| self.build_supported_unicode_range())
    }

    /// Gets the code pages the font declares support for in its OS/2 table.
    ///
    /// `None` when the font does not ship an OS/2 table (or only a version 0
    /// table, which has no code page fields).
    pub fn code_page_coverage(&self) -> FontCodePageCoverage {
        self.code_page_coverage
    }

    /// BCP-47 tags of the languages the font is designed for (`meta` table `dlng`).
    pub fn design_languages(&self) -> &[String] {
        &self.design_languages
    }

    /// BCP-47 tags of the languages the font supports (`meta` table `slng`).
    pub fn supported_languages(&self) -> &[String] {
        &self.supported_languages
    }

    /// Whether the font's `meta` table declares coverage of the culture's language.
    pub fn declares_language_coverage(&self, culture: Option<&CultureInfo>) -> bool {
        let Some(culture) = culture else {
            return false;
        };

        if culture.is_invariant() {
            return false;
        }

        if self.design_languages.is_empty() && self.supported_languages.is_empty() {
            return false;
        }

        let name = culture.name();

        let matches_any =
            |tags: &[String]| tags.iter().any(|tag| Self::is_bcp47_prefix_match(tag, name));

        matches_any(&self.design_languages) || matches_any(&self.supported_languages)
    }

    fn is_bcp47_prefix_match(tag: &str, culture_name: &str) -> bool {
        fn is_prefix(prefix: &str, candidate: &str) -> bool {
            let prefix = prefix.as_bytes();
            let candidate = candidate.as_bytes();

            if prefix.is_empty() || prefix.len() > candidate.len() {
                return false;
            }

            if !candidate[..prefix.len()].eq_ignore_ascii_case(prefix) {
                return false;
            }

            // Either exact match, or the next character is a subtag separator.
            prefix.len() == candidate.len() || candidate[prefix.len()] == b'-' || candidate[prefix.len()] == b'_'
        }

        // Either side may be the narrower one — match if one is a subtag-prefix of the other.
        is_prefix(tag, culture_name) || is_prefix(culture_name, tag)
    }

    /// Whether the font declares (OS/2 Unicode range bit) or demonstrably has
    /// (probe codepoint) support for the script.
    pub fn supports_script(&self, script: Script) -> bool {
        let bit = FontFallbackScriptHints::try_get_os2_bit(script);
        let probe = FontFallbackScriptHints::get_probe_codepoint(script);

        // For scripts we don't track per-script, treat the font as supporting
        // them — the cmap is still the final authority at the call site.
        if bit.is_none() && probe == 0 {
            return true;
        }

        if let (Some(os2), Some(bit)) = (&self.os2_table, bit) {
            let range = match bit {
                0..=31 => os2.unicode_range1,
                32..=63 => os2.unicode_range2,
                64..=95 => os2.unicode_range3,
                _ => os2.unicode_range4,
            };

            if (range & (1u32 << (bit & 31))) != 0 {
                return true;
            }
        }

        probe != 0 && self.cmap_table.try_get_glyph(probe).is_some()
    }

    /// Whether the font can shape the script: simple scripts only need cmap
    /// coverage; complex scripts need the script's tag in GSUB or GPOS.
    pub fn can_shape_script(&self, script: Script) -> bool {
        let Some((primary, secondary)) = FontFallbackScriptHints::try_get_complex_shaping_tags(script) else {
            // Simple script: cmap coverage (checked by the caller) is sufficient.
            return true;
        };

        let (tags, unknown) = self.shaping_script_tags.get_or_init(|| {
            let mut set = HashSet::new();
            let unknown = !ScriptListTable::try_read_script_tags(&*self.platform_typeface, &mut set);
            (set, unknown)
        });

        // A present-but-unparseable GSUB/GPOS leaves capability unknown —
        // don't reject on that basis; cmap remains the authority.
        if *unknown {
            return true;
        }

        tags.contains(&primary) || tags.contains(&secondary)
    }

    fn build_supported_unicode_range(&self) -> UnicodeRange {
        let mut segments = Vec::new();

        let mut enumerator = self.cmap_table.get_mapped_ranges();

        while enumerator.move_next() {
            let range = enumerator.current();
            segments.push(UnicodeRangeSegment::new(range.start, range.end));
        }

        if segments.is_empty() {
            return UnicodeRange::new(0, -1);
        }

        UnicodeRange::from_segments(segments)
    }

    /// Gets the platform-specific typeface associated with this font.
    pub fn platform_typeface(&self) -> &Rc<dyn IPlatformTypeface> {
        &self.platform_typeface
    }

    /// Gets the typeface information used by the text shaper for this font.
    ///
    /// Panics when no text shaper backend is registered.
    pub fn text_shaper_typeface(&self) -> Rc<dyn ITextShaperTypeface> {
        if let Some(typeface) = self.text_shaper_typeface.borrow().as_ref() {
            return typeface.clone();
        }

        let text_shaper = FerroLocator::current().get_required_service::<dyn ITextShaperImpl>();

        let typeface = text_shaper.create_typeface(self);

        *self.text_shaper_typeface.borrow_mut() = Some(typeface.clone());

        typeface
    }

    /// Whether the font is a last resort font (one that maps every codepoint
    /// to a placeholder glyph).
    pub(crate) fn is_last_resort(&self) -> bool {
        self.is_last_resort
    }

    /// Attempts to retrieve the horizontal advance width for the specified glyph.
    pub fn try_get_horizontal_glyph_advance(&self, glyph_index: u16) -> Option<u16> {
        self.hm_table.as_ref()?.try_get_advance(glyph_index).ok().flatten()
    }

    /// Attempts to retrieve the horizontal advance widths for a batch of
    /// glyphs. `advances` must be at least as long as `glyph_indices`.
    pub fn try_get_horizontal_glyph_advances(&self, glyph_indices: &[u16], advances: &mut [u16]) -> bool {
        match &self.hm_table {
            Some(hm_table) => hm_table.try_get_advances(glyph_indices, advances).unwrap_or(false),
            None => false,
        }
    }

    /// Attempts to retrieve the vertical advance height for the specified glyph.
    pub fn try_get_vertical_glyph_advance(&self, glyph_index: u16) -> Option<u16> {
        self.vm_table.as_ref()?.try_get_advance(glyph_index).ok().flatten()
    }

    /// Attempts to retrieve the vertical advance heights for a batch of
    /// glyphs. `advances` must be at least as long as `glyph_indices`.
    pub fn try_get_vertical_glyph_advances(&self, glyph_indices: &[u16], advances: &mut [u16]) -> bool {
        match &self.vm_table {
            Some(vm_table) => vm_table.try_get_advances(glyph_indices, advances).unwrap_or(false),
            None => false,
        }
    }

    /// Attempts to retrieve the metrics for the specified glyph.
    pub fn try_get_glyph_metrics(&self, glyph: u16) -> Option<GlyphMetrics> {
        let h_metric: Option<HorizontalGlyphMetric> =
            self.hm_table.as_ref().and_then(|table| table.try_get_metrics(glyph).ok().flatten());

        let v_metric: Option<VerticalGlyphMetric> =
            self.vm_table.as_ref().and_then(|table| table.try_get_metrics(glyph).ok().flatten());

        let bounds = self.glyf_table.as_ref().and_then(|table| table.try_get_glyph_bounds(glyph as i32));

        if h_metric.is_none() && v_metric.is_none() && bounds.is_none() {
            return None;
        }

        // Funnel the raw header values through `GlyphBounds` so the ink extent
        // is computed (and clamped to non-negative) the same way as the batch
        // path below — a malformed header with x_max < x_min must not wrap
        // when narrowed to the u16 width/height.
        let bbox = bounds.map(|(x_min, y_min, x_max, y_max)| GlyphBounds::new(x_min, y_min, x_max, y_max));

        Some(GlyphMetrics {
            // Bounding box (ink extent) from the glyf header; side bearings
            // fall back to hmtx/vmtx when the glyph has no outline data.
            x_bearing: match (&bbox, &h_metric) {
                (Some(bbox), _) => bbox.x_min as i32,
                (None, Some(h_metric)) => h_metric.left_side_bearing as i32,
                _ => 0,
            },
            y_bearing: match (&bbox, &v_metric) {
                (Some(bbox), _) => bbox.y_max as i32,
                (None, Some(v_metric)) => v_metric.top_side_bearing as i32,
                _ => 0,
            },
            width: bbox.map_or(0, |bbox| bbox.width() as u16),
            height: bbox.map_or(0, |bbox| bbox.height() as u16),
            // Advances come from the metrics tables.
            advance_width: h_metric.map_or(0, |h_metric| h_metric.advance_width),
            advance_height: v_metric.map_or(0, |v_metric| v_metric.advance_height),
            ..GlyphMetrics::default()
        })
    }

    /// Attempts to retrieve the metrics for a batch of glyphs.
    ///
    /// Panics when `metrics` is shorter than `glyph_indices`.
    pub fn try_get_glyph_metrics_batch(&self, glyph_indices: &[u16], metrics: &mut [GlyphMetrics]) -> bool {
        if metrics.len() < glyph_indices.len() {
            panic!("Output span must be at least as long as input span");
        }

        if self.hm_table.is_none() && self.vm_table.is_none() {
            return false;
        }

        let count = glyph_indices.len();

        let mut h_metrics = vec![HorizontalGlyphMetric::default(); if self.hm_table.is_some() { count } else { 0 }];
        let mut v_metrics = vec![VerticalGlyphMetric::default(); if self.vm_table.is_some() { count } else { 0 }];

        // Batch retrieve horizontal and vertical metrics.
        let has_horizontal = self
            .hm_table
            .as_ref()
            .is_some_and(|table| table.try_get_metrics_batch(glyph_indices, &mut h_metrics).unwrap_or(false));

        let has_vertical = self
            .vm_table
            .as_ref()
            .is_some_and(|table| table.try_get_metrics_batch(glyph_indices, &mut v_metrics).unwrap_or(false));

        if !has_horizontal && !has_vertical {
            return false;
        }

        if let Some(glyf_table) = &self.glyf_table {
            // Read all bounding boxes in one batch, so the glyf and loca data
            // are fetched once for the whole run rather than per glyph.
            let mut bounds = vec![GlyphBounds::default(); count];

            glyf_table.get_glyph_bounds(glyph_indices, &mut bounds);

            for i in 0..count {
                let b = bounds[i];

                metrics[i] = GlyphMetrics {
                    x_bearing: b.x_min as i32,
                    y_bearing: b.y_max as i32,
                    width: b.width() as u16,
                    height: b.height() as u16,
                    advance_width: if has_horizontal { h_metrics[i].advance_width } else { 0 },
                    advance_height: if has_vertical { v_metrics[i].advance_height } else { 0 },
                    ..GlyphMetrics::default()
                };
            }
        } else {
            // No glyf table (CFF / CFF2): there are no ink bounds to read, so
            // bearings fall back to hmtx/vmtx and the box stays zero.
            for i in 0..count {
                metrics[i] = GlyphMetrics {
                    x_bearing: if has_horizontal { h_metrics[i].left_side_bearing as i32 } else { 0 },
                    y_bearing: if has_vertical { v_metrics[i].top_side_bearing as i32 } else { 0 },
                    width: 0,
                    height: 0,
                    advance_width: if has_horizontal { h_metrics[i].advance_width } else { 0 },
                    advance_height: if has_vertical { v_metrics[i].advance_height } else { 0 },
                    ..GlyphMetrics::default()
                };
            }
        }

        true
    }

    /// Reads the glyph-header bounding boxes for a batch of glyphs.
    ///
    /// Panics when `bounds` is shorter than `glyph_indices`.
    #[allow(dead_code)] // used by the rendering backends' batch bounds path
    pub(crate) fn try_get_glyph_bounds(&self, glyph_indices: &[u16], bounds: &mut [GlyphBounds]) -> bool {
        if bounds.len() < glyph_indices.len() {
            panic!("Output span must be at least as long as input span");
        }

        match &self.glyf_table {
            Some(glyf_table) => {
                glyf_table.get_glyph_bounds(glyph_indices, bounds);
                true
            }
            None => false,
        }
    }

    /// The font's `glyf` table, when it has TrueType outlines.
    #[allow(dead_code)] // used by glyph outline building
    pub(crate) fn glyf_table(&self) -> Option<&GlyfTable> {
        self.glyf_table.as_ref()
    }

    /// Releases the platform typeface. Idempotent.
    pub fn dispose(&self) {
        if self.is_disposed.replace(true) {
            return;
        }

        if let Some(typeface) = self.text_shaper_typeface.borrow_mut().take() {
            typeface.dispose();
        }

        self.platform_typeface.dispose();
    }

    fn load_supported_features(&self) -> Vec<OpenTypeTag> {
        let font: &dyn IFontMemory = &*self.platform_typeface;

        // A malformed layout table reports no features.
        let g_pos_features = FeatureListTable::load_gpos(font).ok().flatten();
        let g_sub_features = FeatureListTable::load_gsub(font).ok().flatten();

        let mut supported_features = Vec::new();

        for table in [&g_pos_features, &g_sub_features].into_iter().flatten() {
            for feature in table.features() {
                if !supported_features.contains(feature) {
                    supported_features.push(*feature);
                }
            }
        }

        supported_features
    }

    fn get_font_style(os2_table: Option<&OS2Table>, head_table: Option<&HeadTable>, post_table: &PostTable) -> FontStyle {
        let mut is_italic = false;
        let mut is_oblique = false;

        if let Some(os2) = os2_table {
            is_italic = os2.selection.contains(FontSelectionFlags::ITALIC);
            is_oblique = os2.selection.contains(FontSelectionFlags::OBLIQUE);
        }

        if !is_italic {
            if let Some(head_table) = head_table {
                is_italic = head_table.mac_style.contains(MacStyleFlags::Italic);
            }
        }

        let italic_angle = post_table.italic_angle;

        if is_oblique {
            return FontStyle::Oblique;
        }

        if italic_angle.abs() > 0.01 && !is_italic {
            return FontStyle::Oblique;
        }

        if is_italic {
            return FontStyle::Italic;
        }

        FontStyle::Normal
    }

    fn get_font_weight(os2_table: Option<&OS2Table>, head_table: Option<&HeadTable>) -> FontWeight {
        if let Some(os2) = os2_table {
            if (1..=1000).contains(&os2.weight_class) {
                return FontWeight(os2.weight_class as i32);
            }
        }

        if head_table.is_some_and(|head_table| head_table.mac_style.contains(MacStyleFlags::Bold)) {
            return FontWeight::Bold;
        }

        if let Some(os2) = os2_table {
            if os2.panose.family_kind() == PanoseFamilyKind::LatinText {
                let weight = os2.panose.weight();

                return if weight == PanoseWeight::VeryLight {
                    FontWeight::Thin
                } else if weight == PanoseWeight::Light {
                    FontWeight::Light
                } else if weight == PanoseWeight::Thin {
                    FontWeight::ExtraLight
                } else if weight == PanoseWeight::Book {
                    FontWeight::Normal
                } else if weight == PanoseWeight::Medium {
                    FontWeight::Medium
                } else if weight == PanoseWeight::Demi {
                    FontWeight::SemiBold
                } else if weight == PanoseWeight::Bold {
                    FontWeight::Bold
                } else if weight == PanoseWeight::Heavy {
                    FontWeight::ExtraBold
                } else if weight == PanoseWeight::Black {
                    FontWeight::Black
                } else if weight == PanoseWeight::ExtraBlack {
                    FontWeight::ExtraBlack
                } else {
                    FontWeight::Normal
                };
            }
        }

        FontWeight::Normal
    }

    fn get_font_stretch(os2_table: Option<&OS2Table>) -> FontStretch {
        if let Some(os2) = os2_table {
            if let Some(stretch) = FontStretch::from_i32(os2.width_class as i32) {
                return stretch;
            }

            if os2.panose.family_kind() == PanoseFamilyKind::LatinText {
                let proportion = os2.panose.proportion();

                return if proportion == PanoseProportion::VeryCondensed {
                    FontStretch::UltraCondensed
                } else if proportion == PanoseProportion::Condensed {
                    FontStretch::Condensed
                } else if proportion == PanoseProportion::Extended {
                    FontStretch::Expanded
                } else if proportion == PanoseProportion::VeryExtended {
                    FontStretch::UltraExpanded
                } else {
                    FontStretch::Normal
                };
            }
        }

        FontStretch::Normal
    }
}
