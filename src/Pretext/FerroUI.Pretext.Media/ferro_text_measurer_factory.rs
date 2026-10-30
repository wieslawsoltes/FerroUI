use std::rc::Rc;

use ferroui_base::media::text_formatting::unicode::{BidiAlgorithm, BidiData};
use ferroui_base::media::text_formatting::{
    BidiReorderer, GenericTextRunProperties, ShapedTextRun, TextCharacters, TextRun,
    TextRunProperties, TextShaper, TextShaperOptions, UnshapedTextRun,
};
use ferroui_base::media::{
    BaselineAlignment, FlowDirection, FontFamily, FontManager, FontManagerOptions, FontStretch,
    FontStyle, FontWeight, GlyphInfoList, GlyphRun, Typeface,
};
use ferroui_base::platform::{IFontManagerImpl, ITextShaperImpl};
use ferroui_base::utilities::{CultureInfo, ReadOnlyMemory};
use ferroui_base::{FerroLocator, LocatorExtensions, Point};
use ferroui_pretext::{
    IPretextTextMeasurer, IPretextTextMeasurerFactory, IPretextTextShaper,
    IPretextTextShaperFactory, PretextError, PretextFontParser, PretextGlyphRunKind, PretextLayout,
    PretextShapeOptions, PretextShapedFontRun, PretextShapedGlyph, PretextShapedRun,
    PretextTextDirection,
};

use crate::FerroShapedText;

/// Connects Pretext to the application's configured FerroUI text services.
///
/// Instances are UI-thread-affine and belong to their font-service scope. If
/// that scope is replaced, creation/shaping returns an error; create a new
/// factory for the new scope. Replacing a Pretext engine's backend does not
/// invalidate the native services retained by an existing prepared shaper.
#[derive(Clone)]
pub struct FerroTextMeasurerFactory {
    font_manager: Rc<FontManager>,
    text_shaper: Rc<TextShaper>,
    typeface_shaper: Rc<dyn ITextShaperImpl>,
    culture: Option<CultureInfo>,
}

impl FerroTextMeasurerFactory {
    /// Resolves the configured native services without panicking when the
    /// application has not installed a font manager or text shaper.
    pub fn try_current() -> Result<Self, PretextError> {
        let resolver = FerroLocator::current();
        // GlyphTypeface::text_shaper_typeface uses this raw service when font
        // fallback checks script support, even if TextShaper itself was bound
        // directly. Requiring it avoids a later panic on an uncached face.
        let typeface_shaper = resolver
            .get_service::<dyn ITextShaperImpl>()
            .ok_or_else(|| {
                PretextError::Backend("FerroUI text shaping backend is not configured".into())
            })?;
        let font_manager = match resolver.get_service::<FontManager>() {
            Some(manager) => manager,
            None => {
                let backend = resolver
                    .get_service::<dyn IFontManagerImpl>()
                    .ok_or_else(|| {
                        PretextError::Backend("FerroUI font backend is not configured".into())
                    })?;
                let configured_name = resolver
                    .get_service::<FontManagerOptions>()
                    .and_then(|options| options.default_family_name.clone())
                    .unwrap_or_else(|| backend.get_default_font_family_name());
                if configured_name.is_empty()
                    || configured_name == FontFamily::DEFAULT_FONT_FAMILY_NAME
                {
                    return Err(PretextError::Backend(
                        "FerroUI requires a concrete default font family".into(),
                    ));
                }
                FontManager::current()
            }
        };
        let text_shaper = TextShaper::current();
        Ok(Self {
            font_manager,
            text_shaper,
            typeface_shaper,
            culture: None,
        })
    }

    /// Selects the culture used by native font fallback and shaping.
    pub fn with_culture(mut self, culture: CultureInfo) -> Self {
        self.culture = Some(culture);
        self
    }

    /// Creates an engine sharing this backend for measurement and shaping.
    pub fn create_layout(&self) -> PretextLayout {
        PretextLayout::new(Rc::new(self.clone()))
    }

    /// Creates a concrete service for direct native drawing as well as the
    /// portable measurement/shaping contracts.
    pub fn create_text_service(&self, font: &str) -> Result<Rc<FerroTextService>, PretextError> {
        self.ensure_service_scope()?;
        let descriptor = PretextFontParser::parse(font);
        if !descriptor.size.is_finite() || descriptor.size <= 0.0 {
            return Err(PretextError::InvalidInput(
                "font size must be finite and positive".into(),
            ));
        }
        if descriptor.weight <= 0 {
            return Err(PretextError::InvalidInput(
                "font weight must be positive".into(),
            ));
        }
        let family = PretextFontParser::map_generic_family(
            &descriptor.primary_family,
            self.font_manager.default_font_family().name(),
            "serif",
            "monospace",
        );
        let family = FontFamily::parse(family)
            .map_err(|error| PretextError::InvalidInput(error.to_string()))?;
        let style = if descriptor.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        };
        let mut typeface = Typeface::with_style(
            family,
            style,
            FontWeight(descriptor.weight),
            FontStretch::Normal,
        );
        if self
            .font_manager
            .try_get_glyph_typeface(&typeface)
            .is_none()
        {
            typeface = Typeface::with_style(
                self.font_manager.default_font_family().clone(),
                style,
                FontWeight(descriptor.weight),
                FontStretch::Normal,
            );
        }
        if self
            .font_manager
            .try_get_glyph_typeface(&typeface)
            .is_none()
        {
            return Err(PretextError::Backend(format!(
                "could not resolve a FerroUI glyph typeface for {font}"
            )));
        }
        let properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
            typeface,
            descriptor.size,
            None,
            None,
            None,
            BaselineAlignment::Baseline,
            self.culture.clone(),
            None,
        ));
        // Resolve while the verified font service scope is active. The cached
        // face is then reused for all measurements of this font.
        let _ = properties.cached_glyph_typeface();
        Ok(Rc::new(FerroTextService {
            factory: self.clone(),
            properties,
        }))
    }

    fn ensure_service_scope(&self) -> Result<(), PretextError> {
        let resolver = FerroLocator::current();
        let current_manager = resolver.get_service::<FontManager>();
        let current_shaper = resolver.get_service::<TextShaper>();
        let current_typeface_shaper = resolver.get_service::<dyn ITextShaperImpl>();
        if current_manager
            .as_ref()
            .is_some_and(|manager| Rc::ptr_eq(manager, &self.font_manager))
            && current_shaper
                .as_ref()
                .is_some_and(|shaper| Rc::ptr_eq(shaper, &self.text_shaper))
            && current_typeface_shaper
                .as_ref()
                .is_some_and(|shaper| Rc::ptr_eq(shaper, &self.typeface_shaper))
        {
            Ok(())
        } else {
            Err(PretextError::Backend(
                "FerroUI font service scope changed; create a factory in the current scope".into(),
            ))
        }
    }
}

impl IPretextTextMeasurerFactory for FerroTextMeasurerFactory {
    fn name(&self) -> &str {
        "FerroUI"
    }
    fn priority(&self) -> i32 {
        100
    }
    fn is_supported(&self) -> bool {
        self.ensure_service_scope().is_ok()
    }
    fn create(&self, font: &str) -> Result<Rc<dyn IPretextTextMeasurer>, PretextError> {
        Ok(self.create_text_service(font)?)
    }
    fn shaper_factory(&self) -> Option<Rc<dyn IPretextTextShaperFactory>> {
        Some(Rc::new(self.clone()))
    }
}

impl IPretextTextShaperFactory for FerroTextMeasurerFactory {
    fn name(&self) -> &str {
        "FerroUI"
    }
    fn priority(&self) -> i32 {
        100
    }
    fn is_supported(&self) -> bool {
        self.ensure_service_scope().is_ok()
    }
    fn create_shaper(&self, font: &str) -> Result<Rc<dyn IPretextTextShaper>, PretextError> {
        Ok(self.create_text_service(font)?)
    }
}

/// Creates a Pretext engine using the current FerroUI text services.
pub fn create_layout() -> Result<PretextLayout, PretextError> {
    Ok(FerroTextMeasurerFactory::try_current()?.create_layout())
}

/// A fixed-font native measurement and glyph-shaping service.
pub struct FerroTextService {
    factory: FerroTextMeasurerFactory,
    properties: Rc<dyn TextRunProperties>,
}

struct ShapedRuns {
    runs: Vec<Rc<dyn TextRun>>,
    source_starts: Vec<usize>,
}

impl FerroTextService {
    /// Measures with the same shaping and font fallback as native drawing,
    /// without allocating portable glyph records or drawing resources.
    pub fn try_measure_text(&self, text: &str) -> Result<f64, PretextError> {
        let shaped = self.shape_runs(text, PretextShapeOptions::default())?;
        let mut width = 0.0;
        for run in &shaped.runs {
            let run = run.downcast_ref::<ShapedTextRun>().ok_or_else(|| {
                PretextError::Backend("FerroUI returned an unexpected shaped run".into())
            })?;
            for glyph in run.shaped_buffer().glyph_infos().iter() {
                width += glyph.glyph_advance;
            }
        }
        if !width.is_finite() {
            return Err(PretextError::Backend(
                "FerroUI shaped text exceeds the finite width range".into(),
            ));
        }
        Ok(width)
    }

    /// Shapes through the framework's Unicode bidi, font fallback and platform
    /// shaping services and retains the actual native font faces for drawing.
    pub fn shape_native(
        &self,
        text: &str,
        options: PretextShapeOptions,
    ) -> Result<FerroShapedText, PretextError> {
        self.shape_result(text, options, true)
    }

    fn shape_runs(
        &self,
        text: &str,
        options: PretextShapeOptions,
    ) -> Result<ShapedRuns, PretextError> {
        self.factory.ensure_service_scope()?;
        if text.is_empty() {
            return Ok(ShapedRuns {
                runs: Vec::new(),
                source_starts: Vec::new(),
            });
        }
        let memory = ReadOnlyMemory::<u16>::from_str(text);
        if memory.len() > i32::MAX as usize {
            return Err(PretextError::InvalidInput(
                "text exceeds the FerroUI UTF-16 index range".into(),
            ));
        }
        let mut bidi_data = BidiData::new();
        bidi_data.set_paragraph_embedding_level(match options.direction {
            PretextTextDirection::Auto => 2,
            PretextTextDirection::LeftToRight => 0,
            PretextTextDirection::RightToLeft => 1,
        });
        bidi_data.append(memory.span());
        let mut bidi = BidiAlgorithm::new();
        bidi.process(&mut bidi_data);
        let levels = bidi.resolved_levels();
        let flow = if bidi.resolved_paragraph_embedding_level() & 1 == 0 {
            FlowDirection::LeftToRight
        } else {
            FlowDirection::RightToLeft
        };
        let characters = TextCharacters::new(memory.clone(), self.properties.clone());
        let mut unshaped: Vec<Rc<dyn TextRun>> = Vec::new();
        let mut previous_properties = None;
        let mut offset = 0;
        let mut run_start = 0;
        let mut run_level = levels[0];
        // Bidi levels index Unicode scalars; text service ranges index UTF-16.
        for (level_index, ch) in text.chars().enumerate() {
            let level = levels[level_index];
            if level != run_level {
                characters.get_shapeable_characters(
                    memory.slice(run_start, offset - run_start),
                    run_level,
                    &self.factory.font_manager,
                    &mut previous_properties,
                    &mut unshaped,
                );
                run_start = offset;
                run_level = level;
            }
            offset += ch.len_utf16();
        }
        characters.get_shapeable_characters(
            memory.slice(run_start, offset - run_start),
            run_level,
            &self.factory.font_manager,
            &mut previous_properties,
            &mut unshaped,
        );

        let mut shaped_runs: Vec<Rc<dyn TextRun>> = Vec::with_capacity(unshaped.len());
        for run in &unshaped {
            let run = run.downcast_ref::<UnshapedTextRun>().ok_or_else(|| {
                PretextError::Backend("FerroUI returned an unexpected shapeable run".into())
            })?;
            let properties = run.run_properties();
            let options = TextShaperOptions::with_all(
                properties.cached_glyph_typeface(),
                properties.font_rendering_em_size(),
                run.bidi_level(),
                properties.culture_info().cloned(),
                0.0,
                0.0,
                properties
                    .font_features()
                    .map(|features| Rc::new(features.to_vec())),
            );
            let input = run.text();
            let buffer = self.factory.text_shaper.shape_text(&input, &options);
            if buffer.text().span() != input.span() || buffer.bidi_level() != run.bidi_level() {
                return Err(PretextError::Backend(
                    "FerroUI shaper returned incompatible source text or bidi level".into(),
                ));
            }
            for glyph in buffer.glyph_infos().iter() {
                let cluster = usize::try_from(glyph.glyph_cluster).map_err(|_| {
                    PretextError::Backend("FerroUI shaper returned a negative cluster".into())
                })?;
                if cluster >= input.len() || (0xdc00..=0xdfff).contains(&input.span()[cluster]) {
                    return Err(PretextError::Backend(
                        "FerroUI shaper returned an invalid UTF-16 cluster".into(),
                    ));
                }
                if !glyph.glyph_advance.is_finite()
                    || !glyph.glyph_offset.x.is_finite()
                    || !glyph.glyph_offset.y.is_finite()
                {
                    return Err(PretextError::Backend(
                        "FerroUI shaper returned non-finite glyph metrics".into(),
                    ));
                }
            }
            shaped_runs.push(Rc::new(ShapedTextRun::new(buffer, properties.clone())));
        }
        let logical = BidiReorderer::bidi_reorder(&mut shaped_runs, flow, 0);
        let mut visual_source_starts = vec![0usize; shaped_runs.len()];
        for entry in logical.iter() {
            visual_source_starts[entry.run_index.get() as usize] =
                entry.text_source_character_index as usize;
        }

        Ok(ShapedRuns {
            runs: shaped_runs,
            source_starts: visual_source_starts,
        })
    }

    fn shape_result(
        &self,
        text: &str,
        options: PretextShapeOptions,
        retain_native_runs: bool,
    ) -> Result<FerroShapedText, PretextError> {
        let shaped = self.shape_runs(text, options)?;
        let mut glyphs = Vec::new();
        let mut font_runs = Vec::new();
        let mut native_runs = Vec::new();
        let mut pen_x = 0.0;
        for (visual_index, run) in shaped.runs.iter().enumerate() {
            let run = run.downcast_ref::<ShapedTextRun>().ok_or_else(|| {
                PretextError::Backend("FerroUI returned an unexpected shaped run".into())
            })?;
            let buffer = run.shaped_buffer();
            let infos = buffer.glyph_infos();
            if infos.is_empty() {
                continue;
            }
            let source_start = shaped.source_starts[visual_index];
            let font_run_index = font_runs.len();
            let first_glyph_index = glyphs.len();
            let origin_x = pen_x;
            for info in infos.iter() {
                // Validated while building the native shaped runs.
                let local_cluster = info.glyph_cluster as usize;
                glyphs.push(PretextShapedGlyph {
                    glyph_id: u32::from(info.glyph_index),
                    cluster: source_start + local_cluster,
                    x: pen_x + info.glyph_offset.x,
                    y: info.glyph_offset.y,
                    x_advance: info.glyph_advance,
                    y_advance: 0.0,
                    x_offset: info.glyph_offset.x,
                    y_offset: info.glyph_offset.y,
                    font_run_index,
                });
                pen_x += info.glyph_advance;
            }
            let face = buffer.glyph_typeface();
            let font = format!(
                "{}{} {}px \"{}\"",
                if face.style() == FontStyle::Italic {
                    "italic "
                } else {
                    ""
                },
                face.weight().value(),
                buffer.font_rendering_em_size(),
                face.family_name()
            );
            font_runs.push(PretextShapedFontRun {
                index: font_run_index,
                font,
                first_glyph_index,
                glyph_count: infos.len(),
            });
            if retain_native_runs {
                native_runs.push(GlyphRun::new(
                    face.clone(),
                    buffer.font_rendering_em_size(),
                    buffer.text().clone(),
                    GlyphInfoList::ShapedBuffer(buffer.clone()),
                    Some(Point::new(origin_x, 0.0)),
                    i32::from(buffer.bidi_level()),
                ));
            }
        }
        if !pen_x.is_finite() || glyphs.iter().any(|glyph| !glyph.x.is_finite()) {
            return Err(PretextError::Backend(
                "FerroUI shaped text exceeds the finite coordinate range".into(),
            ));
        }
        Ok(FerroShapedText {
            shaped_run: PretextShapedRun::new(
                PretextGlyphRunKind::Shaped,
                glyphs,
                font_runs,
                pen_x,
                0.0,
            ),
            glyph_runs: native_runs,
        })
    }
}

impl IPretextTextMeasurer for FerroTextService {
    fn measure_text(&self, text: &str) -> f64 {
        // The upstream measurement contract has no error channel. Preparation
        // rejects non-finite metrics; direct users can call try_measure_text.
        self.try_measure_text(text).unwrap_or(f64::NAN)
    }
}

impl IPretextTextShaper for FerroTextService {
    fn shape_text(
        &self,
        text: &str,
        options: PretextShapeOptions,
    ) -> Result<PretextShapedRun, PretextError> {
        self.shape_result(text, options, false)
            .map(FerroShapedText::into_shaped_run)
    }
}
