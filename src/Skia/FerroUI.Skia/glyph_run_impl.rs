use crate::sk_text_blob_builder_cache::SkTextBlobBuilderCache;
use crate::two_level_cache::TwoLevelCache;
use crate::skia_typeface::SkiaTypeface;
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::{
    BaselinePixelAlignment, EdgeMode, GlyphTypeface, RenderOptions, TextHintingMode, TextOptions, TextRenderingMode,
};
use ferroui_base::platform::IGlyphRunImpl;
use ferroui_base::{Point, Rect, Vector};
use skia_safe::font::Edging;
use skia_safe::{Font, FontHinting, TextBlob};
use std::any::Any;
use std::cell::RefCell;

/// How the font of a text blob is configured: the options the blob is
/// built for.
fn create_font(font: &Font, text_options: TextOptions) -> Font {
    // Determine edging from the text rendering mode.
    let edging = match text_options.text_rendering_mode {
        TextRenderingMode::Alias => Edging::Alias,
        TextRenderingMode::Antialias => Edging::AntiAlias,
        TextRenderingMode::SubpixelAntialias => Edging::SubpixelAntiAlias,
        _ => Edging::SubpixelAntiAlias,
    };

    // Determine hinting.
    let hinting = match text_options.text_hinting_mode {
        TextHintingMode::None => FontHinting::None,
        TextHintingMode::Light => FontHinting::Slight,
        TextHintingMode::Strong => FontHinting::Full,
        _ => FontHinting::Full,
    };

    // Force auto-hinting for the light mode (prefer the autohinter over
    // bytecode hints), otherwise default.
    let force_auto_hinting = text_options.text_hinting_mode == TextHintingMode::Light;

    // Subpixel positioning is enabled when the edging is not alias.
    let subpixel = edging != Edging::Alias;

    // Baseline snap defaults to true unless explicitly disabled.
    let baseline_snap = text_options.baseline_pixel_alignment != BaselinePixelAlignment::Unaligned;

    let mut font = font.clone();

    font.set_force_auto_hinting(force_auto_hinting);
    font.set_hinting(hinting);
    font.set_subpixel(subpixel);
    font.set_edging(edging);
    font.set_baseline_snap(baseline_snap);

    font
}

/// The Skia implementation of a glyph run: glyph indices and positions that
/// are turned into text blobs for drawing.
pub struct GlyphRunImpl {
    font: Font,
    glyph_indices: Vec<u16>,
    glyph_positions: Vec<skia_safe::Point>,
    // A two level cache optimized for single-entry read, keyed by the font
    // configuration.
    text_blob_cache: RefCell<TwoLevelCache<TextOptions, Option<TextBlob>>>,
    font_rendering_em_size: f64,
    baseline_origin: Point,
    bounds: Rect,
}

impl GlyphRunImpl {
    /// Creates the glyph run of the given glyphs of a glyph typeface.
    ///
    /// # Panics
    /// Panics when the platform typeface of the glyph typeface was not
    /// created by this backend.
    pub fn new(
        glyph_typeface: &GlyphTypeface,
        font_rendering_em_size: f64,
        glyph_infos: &[GlyphInfo],
        baseline_origin: Point,
    ) -> Self {
        let glyph_typeface_impl = SkiaTypeface::try_get(&**glyph_typeface.platform_typeface())
            .unwrap_or_else(|| panic!("The platform typeface was not created by the Skia backend"));

        let font = glyph_typeface_impl.create_sk_font(font_rendering_em_size as f32);

        Self::from_font(&font, font_rendering_em_size, glyph_infos, baseline_origin)
    }

    /// Creates a glyph run from a Skia font.
    ///
    /// `font` carries the typeface and whatever the typeface needs applied
    /// to render as intended (synthetic bold or italic); its size is replaced
    /// with `font_rendering_em_size`, its edging and hinting per text blob.
    pub fn from_font(font: &Font, font_rendering_em_size: f64, glyphs: &[GlyphInfo], baseline_origin: Point) -> Self {
        let count = glyphs.len();

        let mut font = font.clone();
        font.set_size(font_rendering_em_size as f32);

        // The glyph bounds can only be fetched once the indices are known,
        // so this walk has to come first.
        let glyph_indices: Vec<u16> = glyphs.iter().map(|glyph| glyph.glyph_index).collect();
        let mut glyph_positions = Vec::with_capacity(count);

        // Ideally the requested edging should be passed to the glyph run.
        // Currently the edging is computed dynamically inside the drawing
        // context, so we can't know it in advance. But the bounds depend on
        // the edging: for now, always use subpixel antialiasing so we have
        // consistent values. The resulting bounds may be shifted by 1px on
        // some fonts.
        let default_text_options = TextOptions {
            text_rendering_mode: TextRenderingMode::SubpixelAntialias,
            text_hinting_mode: TextHintingMode::Strong,
            baseline_pixel_alignment: BaselinePixelAlignment::Unaligned,
        };
        let measuring_font = create_font(&font, default_text_options);

        let mut glyph_bounds = vec![skia_safe::Rect::default(); count];
        measuring_font.get_bounds(&glyph_indices, &mut glyph_bounds, None);

        // Build the glyph positions and union the run bounds in a single
        // pass: one accumulator covers both outputs.
        let mut current_x = 0.0;
        let mut run_bounds = Rect::default();

        for (glyph, g_bounds) in glyphs.iter().zip(&glyph_bounds) {
            let offset = glyph.glyph_offset;

            glyph_positions.push(skia_safe::Point::new((current_x + offset.x) as f32, offset.y as f32));

            run_bounds = run_bounds.union(Rect::new(
                current_x + g_bounds.left as f64,
                g_bounds.top as f64,
                g_bounds.width() as f64,
                g_bounds.height() as f64,
            ));

            current_x += glyph.glyph_advance;
        }

        Self {
            font,
            glyph_indices,
            glyph_positions,
            text_blob_cache: RefCell::new(TwoLevelCache::with_secondary_size(3)),
            font_rendering_em_size,
            baseline_origin,
            bounds: run_bounds.translate(Vector::new(baseline_origin.x, baseline_origin.y)),
        }
    }

    /// The text blob of the run for the given text options. An unspecified
    /// text rendering mode is derived from the edge mode of the render
    /// options.
    ///
    /// Returns `None` for an empty run.
    pub fn get_text_blob(&self, text_options: TextOptions, render_options: RenderOptions) -> Option<TextBlob> {
        let mut text_options = text_options;

        if text_options.text_rendering_mode == TextRenderingMode::Unspecified {
            text_options.text_rendering_mode = if render_options.edge_mode == EdgeMode::Aliased {
                TextRenderingMode::Alias
            } else {
                TextRenderingMode::SubpixelAntialias
            };
        }

        self.text_blob_cache.borrow_mut().get_or_add(text_options, |text_options| {
            if self.glyph_indices.is_empty() {
                return None;
            }

            let font = create_font(&self.font, *text_options);

            let mut builder = SkTextBlobBuilderCache::get();

            let (glyphs, positions) = builder.alloc_run_pos(&font, self.glyph_indices.len(), None);
            positions.copy_from_slice(&self.glyph_positions);
            glyphs.copy_from_slice(&self.glyph_indices);

            let text_blob = builder.make();

            SkTextBlobBuilderCache::return_item(builder);

            text_blob
        })
    }
}

impl IGlyphRunImpl for GlyphRunImpl {
    fn font_rendering_em_size(&self) -> f64 {
        self.font_rendering_em_size
    }

    fn baseline_origin(&self) -> Point {
        self.baseline_origin
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn get_intersections(&self, lower_limit: f32, upper_limit: f32) -> Vec<f32> {
        match self.get_text_blob(TextOptions::default(), RenderOptions::default()) {
            Some(text_blob) => text_blob.get_intercepts([lower_limit, upper_limit], None),
            None => Vec::new(),
        }
    }

    fn dispose(&self) {
        self.text_blob_cache.borrow_mut().clear_and_dispose();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
