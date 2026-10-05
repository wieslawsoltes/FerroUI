use crate::media::{GlyphRun, GlyphTypeface, IBrush, IImmutableBrush, Typeface};
use crate::platform::{IDrawingContextImpl, IGlyphRunImpl};
use crate::utilities::ReadOnlyMemory;
use crate::{Matrix, Rect, Size};
use std::rc::Rc;

const FIRST_CHAR: u16 = 32;
const LAST_CHAR: u16 = 126;

/// A single-glyph run as needed by [`DiagnosticTextRenderer`].
#[derive(Clone)]
pub struct DiagnosticGlyphRun {
    /// The layout bounds of the run (the C# `GlyphRun.Bounds`: advance width
    /// by line height), which is what text is measured and advanced with.
    /// This is not the ink bounding box reported by
    /// [`IGlyphRunImpl::bounds`].
    pub bounds: Rect,
    /// The platform glyph run that gets drawn.
    pub platform_impl: Rc<dyn IGlyphRunImpl>,
}

/// The seam between [`DiagnosticTextRenderer`] and the text stack.
///
/// The C# constructor takes a glyph typeface and an em size and builds one
/// `GlyphRun` per printable ASCII character from them. That single step is
/// delegated to an implementation of this trait, which captures the glyph
/// typeface and the font rendering em size:
/// [`GlyphTypefaceDiagnosticGlyphRunSource`] is the implementation over the
/// text stack, used by [`DiagnosticTextRenderer::from_glyph_typeface`].
pub trait IDiagnosticGlyphRunSource {
    /// Creates the glyph run consisting of the single glyph the typeface
    /// maps `character` to. Called once for every character from `' '`
    /// (32) to `'~'` (126), in order.
    fn create_glyph_run(&self, character: char) -> DiagnosticGlyphRun;
}

/// Builds the glyph runs of a [`DiagnosticTextRenderer`] from a glyph
/// typeface and a font rendering em size, as the C# constructor does.
pub struct GlyphTypefaceDiagnosticGlyphRunSource {
    glyph_typeface: Rc<GlyphTypeface>,
    font_rendering_em_size: f64,
    /// The printable ASCII characters; every run refers to its character
    /// in this buffer.
    chars: ReadOnlyMemory<u16>,
}

impl GlyphTypefaceDiagnosticGlyphRunSource {
    pub fn new(glyph_typeface: Rc<GlyphTypeface>, font_rendering_em_size: f64) -> Self {
        let chars = ReadOnlyMemory::from_vec((FIRST_CHAR..=LAST_CHAR).collect());
        Self { glyph_typeface, font_rendering_em_size, chars }
    }
}

impl IDiagnosticGlyphRunSource for GlyphTypefaceDiagnosticGlyphRunSource {
    fn create_glyph_run(&self, character: char) -> DiagnosticGlyphRun {
        let code_point = character as u32;
        let effective_char =
            if (FIRST_CHAR as u32..=LAST_CHAR as u32).contains(&code_point) { code_point as u16 } else { FIRST_CHAR };
        let index = (effective_char - FIRST_CHAR) as usize;
        let glyph = self.glyph_typeface.character_to_glyph_map().get_glyph(character as i32);
        let run = GlyphRun::from_glyph_indices(
            self.glyph_typeface.clone(),
            self.font_rendering_em_size,
            self.chars.slice(index, 1),
            &[glyph],
            None,
            0,
        );
        DiagnosticGlyphRun { bounds: run.bounds(), platform_impl: run.platform_impl() }
    }
}

/// A class used to render diagnostic strings (only!), with caching of ASCII glyph runs.
pub struct DiagnosticTextRenderer {
    runs: Vec<DiagnosticGlyphRun>,
}

impl DiagnosticTextRenderer {
    pub fn get_max_height(&self) -> f64 {
        let mut max_height = 0.0;

        for run in &self.runs {
            let height = run.bounds.height;
            if height > max_height {
                max_height = height;
            }
        }

        max_height
    }

    pub fn new(glyph_run_source: &dyn IDiagnosticGlyphRunSource) -> Self {
        let mut runs = Vec::with_capacity((LAST_CHAR - FIRST_CHAR + 1) as usize);
        for c in FIRST_CHAR..=LAST_CHAR {
            runs.push(glyph_run_source.create_glyph_run(c as u8 as char));
        }
        Self { runs }
    }

    /// Creates a renderer whose glyph runs are built from `glyph_typeface`
    /// at `font_rendering_em_size` (the C# constructor).
    pub fn from_glyph_typeface(glyph_typeface: Rc<GlyphTypeface>, font_rendering_em_size: f64) -> Self {
        Self::new(&GlyphTypefaceDiagnosticGlyphRunSource::new(glyph_typeface, font_rendering_em_size))
    }

    /// Creates the renderer the compositor's debug overlays use: the glyph
    /// typeface of the default typeface at an em size of 12.
    pub fn create_default() -> Self {
        Self::from_glyph_typeface(Typeface::default_typeface().glyph_typeface(), 12.0)
    }

    fn run_for(&self, c: u16) -> &DiagnosticGlyphRun {
        let effective_char = if (FIRST_CHAR..=LAST_CHAR).contains(&c) { c } else { b' ' as u16 };
        &self.runs[(effective_char - FIRST_CHAR) as usize]
    }

    /// Measures `text`. Every UTF-16 code unit outside the printable ASCII
    /// range is measured as a space.
    pub fn measure_ascii_text(&self, text: &str) -> Size {
        let mut width = 0.0;
        let mut height = 0.0_f64;

        for c in text.encode_utf16() {
            let run = self.run_for(c);
            width += run.bounds.width;
            height = height.max(run.bounds.height);
        }

        Size::new(width, height)
    }

    /// Draws `text` at the origin of the current transform of `context`.
    /// Every UTF-16 code unit outside the printable ASCII range is drawn as
    /// a space.
    pub fn draw_ascii_text(&self, context: &mut dyn IDrawingContextImpl, text: &str, foreground: &dyn IImmutableBrush) {
        let mut offset = 0.0;
        let foreground: &dyn IBrush = foreground;

        for c in text.encode_utf16() {
            let run = self.run_for(c);
            let old_transform = context.transform();
            context.set_transform(Matrix::create_translation(offset, 0.0) * old_transform);
            context.draw_glyph_run(Some(foreground), &*run.platform_impl);
            context.set_transform(old_transform);
            offset += run.bounds.width;
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::dirty_rects::test_mocks::MockDrawingContext;
    use super::*;
    use crate::media::Brushes;
    use crate::Point;
    use std::cell::RefCell;

    pub(crate) struct MockGlyphRunImpl(pub char);

    impl IGlyphRunImpl for MockGlyphRunImpl {
        fn font_rendering_em_size(&self) -> f64 {
            // Lets the mock drawing context tell the glyphs apart.
            self.0 as u32 as f64
        }
        fn baseline_origin(&self) -> Point {
            Point::default()
        }
        fn bounds(&self) -> Rect {
            Rect::default()
        }
        fn get_intersections(&self, _: f32, _: f32) -> Vec<f32> {
            Vec::new()
        }
        fn dispose(&self) {}
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    /// Every glyph is 6 wide and 10 high, except `'|'` (3 x 14).
    #[derive(Default)]
    pub(crate) struct MockGlyphRunSource {
        pub requested: RefCell<Vec<char>>,
    }

    impl IDiagnosticGlyphRunSource for MockGlyphRunSource {
        fn create_glyph_run(&self, character: char) -> DiagnosticGlyphRun {
            self.requested.borrow_mut().push(character);
            let bounds = if character == '|' { Rect::new(0.0, 0.0, 3.0, 14.0) } else { Rect::new(0.0, 0.0, 6.0, 10.0) };
            DiagnosticGlyphRun { bounds, platform_impl: Rc::new(MockGlyphRunImpl(character)) }
        }
    }

    pub(crate) fn create_renderer() -> DiagnosticTextRenderer {
        DiagnosticTextRenderer::new(&MockGlyphRunSource::default())
    }

    #[test]
    fn creates_one_run_per_printable_ascii_char_in_order() {
        let source = MockGlyphRunSource::default();
        let _renderer = DiagnosticTextRenderer::new(&source);
        let requested = source.requested.borrow();
        assert_eq!(95, requested.len());
        assert_eq!(' ', requested[0]);
        assert_eq!('~', requested[94]);
        assert!(requested.windows(2).all(|w| w[0] as u32 + 1 == w[1] as u32));
    }

    #[test]
    fn glyph_runs_are_built_from_a_glyph_typeface_and_an_em_size() {
        use crate::media::text_formatting::testing::{advance, TextTestScope};
        use crate::media::Typeface;
        use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};

        let _scope = TextTestScope::new();
        let glyph_typeface = Typeface::default_typeface().glyph_typeface();

        let source = GlyphTypefaceDiagnosticGlyphRunSource::new(glyph_typeface.clone(), 12.0);
        let run = source.create_glyph_run('A');
        // The layout bounds: the advance by the line height of the font.
        let expected = GlyphRun::from_glyph_indices(
            glyph_typeface.clone(),
            12.0,
            ReadOnlyMemory::from_vec(vec!['A' as u16]),
            &[glyph_typeface.character_to_glyph_map().get_glyph('A' as i32)],
            None,
            0,
        );
        assert_eq!(run.bounds, expected.bounds());
        assert_eq!(run.bounds.width, advance(12.0));
        assert_eq!(run.platform_impl.font_rendering_em_size(), 12.0);
        assert_eq!(run.platform_impl.bounds(), expected.platform_impl().bounds());

        let renderer = DiagnosticTextRenderer::from_glyph_typeface(glyph_typeface, 12.0);
        assert_eq!(Size::new(3.0 * advance(12.0), run.bounds.height), renderer.measure_ascii_text("FPS"));
        assert_eq!(run.bounds.height, renderer.get_max_height());

        // The default renderer uses the default typeface at an em size of 12.
        let default_renderer = DiagnosticTextRenderer::create_default();
        assert_eq!(renderer.measure_ascii_text("12 ms"), default_renderer.measure_ascii_text("12 ms"));

        let log = DrawingLog::new();
        let mut context = MockDrawingContextImpl::new(log.clone());
        context.log_transforms = false;
        default_renderer.draw_ascii_text(&mut context, "ab", &*Brushes::white());
        assert_eq!(2, log.count("DrawGlyphRun White "));
    }

    #[test]
    fn max_height_is_the_tallest_run() {
        assert_eq!(14.0, create_renderer().get_max_height());
    }

    #[test]
    fn measures_widths_and_max_height() {
        let renderer = create_renderer();
        assert_eq!(Size::new(0.0, 0.0), renderer.measure_ascii_text(""));
        assert_eq!(Size::new(18.0, 10.0), renderer.measure_ascii_text("abc"));
        assert_eq!(Size::new(15.0, 14.0), renderer.measure_ascii_text("a|b"));
        // Non-printable and non-ASCII characters count as spaces; characters
        // outside the BMP are two UTF-16 units and so two spaces.
        assert_eq!(Size::new(12.0, 10.0), renderer.measure_ascii_text("\n\u{e9}"));
        assert_eq!(Size::new(12.0, 10.0), renderer.measure_ascii_text("\u{1F600}"));
    }

    #[test]
    fn draws_each_glyph_advanced_by_the_previous_widths() {
        let renderer = create_renderer();
        let mut ctx =
            MockDrawingContext { transform: Matrix::create_translation(100.0, 50.0), ..Default::default() };
        renderer.draw_ascii_text(&mut ctx, "a|\u{7f}b", &*Brushes::white());
        assert_eq!(
            vec![
                "glyph 97 #ffffffff@1 at 100,50",
                "glyph 124 #ffffffff@1 at 106,50",
                "glyph 32 #ffffffff@1 at 109,50",
                "glyph 98 #ffffffff@1 at 115,50",
            ],
            ctx.log
        );
        // The transform is restored.
        assert_eq!(Matrix::create_translation(100.0, 50.0), ctx.transform);
    }
}
