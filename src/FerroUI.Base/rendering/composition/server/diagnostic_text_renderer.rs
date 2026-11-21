use crate::media::{GlyphRun, GlyphTypeface, IBrush, IImmutableBrush, Typeface};
use crate::platform::{IDrawingContextImpl, IGlyphRunImpl};
use crate::utilities::ReadOnlyMemory;
use crate::{Matrix, Rect, Size};
use std::rc::Rc;

const FIRST_CHAR: u16 = 32;
const LAST_CHAR: u16 = 126;

/// A glyph run as needed by [`DiagnosticTextRenderer`].
#[derive(Clone)]
pub struct DiagnosticGlyphRun {
    /// The layout bounds of the run (the C# `GlyphRun.Bounds`: advance width
    /// by line height), which is what text is measured and advanced with.
    /// This is not the ink bounding box reported by
    /// [`IGlyphRunImpl::bounds`].
    pub bounds: Rect,
    /// The platform glyph run that gets drawn.
    pub platform_impl: std::sync::Arc<dyn IGlyphRunImpl>,
}

/// The seam between [`DiagnosticTextRenderer`] and the text stack.
///
/// The C# constructor takes a glyph typeface and an em size and builds one
/// `GlyphRun` per printable ASCII character from them; `Shape` builds a
/// glyph run of a whole text from the same typeface. Those steps are
/// delegated to an implementation of this trait, which captures the glyph
/// typeface and the font rendering em size:
/// [`GlyphTypefaceDiagnosticGlyphRunSource`] is the implementation over the
/// text stack, used by [`DiagnosticTextRenderer::from_glyph_typeface`].
pub trait IDiagnosticGlyphRunSource {
    /// Creates the glyph run consisting of the single glyph the typeface
    /// maps `character` to. Called once for every character from `' '`
    /// (32) to `'~'` (126), in order.
    fn create_glyph_run(&self, character: char) -> DiagnosticGlyphRun;

    /// Creates the glyph run of `text`, one glyph per UTF-16 code unit as
    /// the typeface maps it. `text` is printable ASCII and not empty.
    fn create_text_glyph_run(&self, text: &[u16]) -> DiagnosticGlyphRun;

    /// The line spacing of the typeface scaled to the font rendering em
    /// size (`Metrics.LineSpacing * fontRenderingEmSize / Metrics.DesignEmHeight`).
    fn line_height(&self) -> f64;
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

    fn create_text_glyph_run(&self, text: &[u16]) -> DiagnosticGlyphRun {
        let character_to_glyph_map = self.glyph_typeface.character_to_glyph_map();
        let glyphs: Vec<u16> = text.iter().map(|&c| character_to_glyph_map.get_glyph(c as i32)).collect();
        let run = GlyphRun::from_glyph_indices(
            self.glyph_typeface.clone(),
            self.font_rendering_em_size,
            ReadOnlyMemory::from_vec(text.to_vec()),
            &glyphs,
            None,
            0,
        );
        DiagnosticGlyphRun { bounds: run.bounds(), platform_impl: run.platform_impl() }
    }

    fn line_height(&self) -> f64 {
        let metrics = self.glyph_typeface.metrics();
        metrics.line_spacing() as f64 * self.font_rendering_em_size / metrics.design_em_height as f64
    }
}

/// A class used to render diagnostic strings (only!), with caching of ASCII glyph runs.
/// Digits are laid out in equal-width cells so rapidly changing values do not make the text jump around.
pub struct DiagnosticTextRenderer {
    /// Labels shaped once and shared, by text ([`shape_label`](Self::shape_label)).
    shaped_labels: std::cell::RefCell<Vec<(&'static str, Rc<ShapedText>)>>,
    glyph_run_source: Rc<dyn IDiagnosticGlyphRunSource>,
    runs: Vec<DiagnosticGlyphRun>,
    digit_offsets: [f64; 10],
    line_height: f64,
    digit_cell_width: f64,
}

impl DiagnosticTextRenderer {
    pub fn get_max_height(&self) -> f64 {
        self.line_height
    }

    /// The width of the cell every digit is centered in.
    pub fn digit_cell_width(&self) -> f64 {
        self.digit_cell_width
    }

    pub fn new(glyph_run_source: Rc<dyn IDiagnosticGlyphRunSource>) -> Self {
        let line_height = glyph_run_source.line_height();

        let mut runs = Vec::with_capacity((LAST_CHAR - FIRST_CHAR + 1) as usize);
        for c in FIRST_CHAR..=LAST_CHAR {
            runs.push(glyph_run_source.create_glyph_run(c as u8 as char));
        }

        // Digits share the width of the widest one so changing values do not shift the text that follows
        let mut digit_cell_width = 0.0_f64;
        for c in b'0'..=b'9' {
            digit_cell_width = digit_cell_width.max(runs[(c as u16 - FIRST_CHAR) as usize].bounds.width);
        }

        let mut digit_offsets = [0.0; 10];
        for (digit, offset) in digit_offsets.iter_mut().enumerate() {
            *offset = (digit_cell_width - runs[(b'0' as u16 + digit as u16 - FIRST_CHAR) as usize].bounds.width) / 2.0;
        }

        Self { glyph_run_source, runs, digit_offsets, line_height, digit_cell_width, shaped_labels: Default::default() }
    }

    /// Creates a renderer whose glyph runs are built from `glyph_typeface`
    /// at `font_rendering_em_size` (the C# constructor).
    pub fn from_glyph_typeface(glyph_typeface: Rc<GlyphTypeface>, font_rendering_em_size: f64) -> Self {
        Self::new(Rc::new(GlyphTypefaceDiagnosticGlyphRunSource::new(glyph_typeface, font_rendering_em_size)))
    }

    /// Creates the renderer the compositor's debug overlays use: the glyph
    /// typeface of the default typeface at an em size of 12.
    pub fn create_default() -> Self {
        Self::from_glyph_typeface(Typeface::default_typeface().glyph_typeface(), 12.0)
    }

    fn effective_char(c: u16) -> u16 {
        if (FIRST_CHAR..=LAST_CHAR).contains(&c) {
            c
        } else {
            b' ' as u16
        }
    }

    fn run_for(&self, c: u16) -> &DiagnosticGlyphRun {
        &self.runs[(Self::effective_char(c) - FIRST_CHAR) as usize]
    }

    fn is_digit(c: u16) -> bool {
        (b'0' as u16..=b'9' as u16).contains(&c)
    }

    /// Shapes `text` into a single glyph run that can be drawn with
    /// [`draw_shaped_text`](Self::draw_shaped_text) without any per-frame
    /// measuring or shaping. Every UTF-16 code unit outside the printable
    /// ASCII range is shaped as a space. Glyphs advance by their natural
    /// width, so digits are not placed in cells; use
    /// [`draw_digits`](Self::draw_digits) for values that change.
    pub fn shape(&self, text: &str) -> ShapedText {
        let chars: Vec<u16> = text.encode_utf16().map(Self::effective_char).collect();

        if chars.is_empty() {
            return ShapedText { glyph_run: None, width: 0.0 };
        }

        ShapedText::new(self.glyph_run_source.create_text_glyph_run(&chars))
    }

    /// The label `text` shaped by [`shape`](Self::shape), shaped the first
    /// time and shared after that. Not in the reference: the port shapes the
    /// fixed labels of the FPS counter on the UI thread with this, where the
    /// glyph runs can be created (`ServerCompositor::diagnostic_text_renderer`).
    pub fn shape_label(&self, text: &'static str) -> Rc<ShapedText> {
        if let Some((_, shaped)) = self.shaped_labels.borrow().iter().find(|(label, _)| *label == text) {
            return shaped.clone();
        }
        let shaped = Rc::new(self.shape(text));
        self.shaped_labels.borrow_mut().push((text, shaped.clone()));
        shaped
    }

    /// Draws text shaped by [`shape`](Self::shape) at the origin of the
    /// current transform of `context`.
    pub fn draw_shaped_text(
        &self,
        context: &mut dyn IDrawingContextImpl,
        text: &ShapedText,
        foreground: &dyn IImmutableBrush,
    ) {
        if let Some(glyph_run) = &text.glyph_run {
            let foreground: &dyn IBrush = foreground;
            context.draw_glyph_run(Some(foreground), &*glyph_run.platform_impl);
        }
    }

    /// Draws a non-negative `value` zero padded to `digit_count` digits, each
    /// in its own [`digit_cell_width`](Self::digit_cell_width) wide cell,
    /// without formatting a string. Values that do not fit are clamped to the
    /// largest representable value.
    pub fn draw_digits(
        &self,
        context: &mut dyn IDrawingContextImpl,
        value: i32,
        digit_count: i32,
        foreground: &dyn IImmutableBrush,
    ) {
        let foreground: &dyn IBrush = foreground;

        let mut divisor = 1_i64;
        for _ in 1..digit_count {
            divisor *= 10;
        }

        let mut remaining = (value as i64).clamp(0, divisor * 10 - 1);

        for i in 0..digit_count {
            let digit = (remaining / divisor) as usize;
            remaining %= divisor;
            divisor /= 10;

            let run = &self.runs[(b'0' as u16 + digit as u16 - FIRST_CHAR) as usize];
            let old_transform = context.transform();
            context.set_transform(
                Matrix::create_translation(i as f64 * self.digit_cell_width + self.digit_offsets[digit], 0.0)
                    * old_transform,
            );
            context.draw_glyph_run(Some(foreground), &*run.platform_impl);
            context.set_transform(old_transform);
        }
    }

    /// Measures `text`. Every UTF-16 code unit outside the printable ASCII
    /// range is measured as a space.
    pub fn measure_ascii_text(&self, text: &str) -> Size {
        let mut width = 0.0;

        for c in text.encode_utf16() {
            let run = self.run_for(c);
            width += if Self::is_digit(c) { self.digit_cell_width } else { run.bounds.width };
        }

        Size::new(width, self.line_height)
    }

    /// Draws `text` at the origin of the current transform of `context`.
    /// Every UTF-16 code unit outside the printable ASCII range is drawn as
    /// a space.
    pub fn draw_ascii_text(&self, context: &mut dyn IDrawingContextImpl, text: &str, foreground: &dyn IImmutableBrush) {
        let mut offset = 0.0;
        let foreground: &dyn IBrush = foreground;

        for c in text.encode_utf16() {
            let char_is_number = Self::is_digit(c);
            let run = self.run_for(c);

            // Center digits inside the fixed-width digit cell
            let centering_offset = if char_is_number { (self.digit_cell_width - run.bounds.width) / 2.0 } else { 0.0 };

            let old_transform = context.transform();
            context.set_transform(Matrix::create_translation(offset + centering_offset, 0.0) * old_transform);
            context.draw_glyph_run(Some(foreground), &*run.platform_impl);
            context.set_transform(old_transform);

            offset += if char_is_number { self.digit_cell_width } else { run.bounds.width };
        }
    }
}

/// A piece of text shaped once by [`DiagnosticTextRenderer::shape`].
pub struct ShapedText {
    /// The glyph run of the text; `None` for an empty text, which draws
    /// nothing.
    glyph_run: Option<DiagnosticGlyphRun>,
    width: f64,
}

impl ShapedText {
    /// `ShapedText(GlyphRun glyphRun)`: the text of `glyph_run`, as wide as
    /// its bounds.
    pub(crate) fn new(glyph_run: DiagnosticGlyphRun) -> Self {
        Self { width: glyph_run.bounds.width, glyph_run: Some(glyph_run) }
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn dispose(&self) {
        if let Some(glyph_run) = &self.glyph_run {
            glyph_run.platform_impl.dispose();
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::dirty_rects::test_mocks::MockDrawingContext;
    use super::super::FpsCounter;
    use super::*;
    use crate::media::Brushes;
    use crate::Point;
    use std::cell::RefCell;
    use std::time::Duration;

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
    }

    /// The glyph run of a shaped text; the mock drawing context logs it with
    /// its text.
    pub(crate) struct MockTextGlyphRunImpl(pub String);

    impl IGlyphRunImpl for MockTextGlyphRunImpl {
        fn font_rendering_em_size(&self) -> f64 {
            0.0
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
    }

    /// Every glyph is 6 wide and 10 high, except `'|'` (3 x 14); the line
    /// height is 14. With `variable_digits`, the digit `d` is `4 + d / 2`
    /// wide.
    #[derive(Default)]
    pub(crate) struct MockGlyphRunSource {
        pub requested: RefCell<Vec<char>>,
        pub variable_digits: bool,
    }

    impl MockGlyphRunSource {
        fn width(&self, character: char) -> f64 {
            match character {
                '|' => 3.0,
                '0'..='9' if self.variable_digits => 4.0 + (character as u32 - '0' as u32) as f64 / 2.0,
                _ => 6.0,
            }
        }
    }

    impl IDiagnosticGlyphRunSource for MockGlyphRunSource {
        fn create_glyph_run(&self, character: char) -> DiagnosticGlyphRun {
            self.requested.borrow_mut().push(character);
            let height = if character == '|' { 14.0 } else { 10.0 };
            DiagnosticGlyphRun {
                bounds: Rect::new(0.0, 0.0, self.width(character), height),
                platform_impl: std::sync::Arc::new(MockGlyphRunImpl(character)),
            }
        }

        fn create_text_glyph_run(&self, text: &[u16]) -> DiagnosticGlyphRun {
            let text = String::from_utf16(text).unwrap();
            let width = text.chars().map(|c| self.width(c)).sum();
            DiagnosticGlyphRun {
                bounds: Rect::new(0.0, 0.0, width, 10.0),
                platform_impl: std::sync::Arc::new(MockTextGlyphRunImpl(text)),
            }
        }

        fn line_height(&self) -> f64 {
            14.0
        }
    }

    pub(crate) fn create_renderer() -> DiagnosticTextRenderer {
        DiagnosticTextRenderer::new(Rc::new(MockGlyphRunSource::default()))
    }

    /// A renderer whose digits differ in width (the font of the upstream
    /// tests has proportional digits; the fonts of the test harness do not).
    fn create_variable_digits_renderer() -> DiagnosticTextRenderer {
        DiagnosticTextRenderer::new(Rc::new(MockGlyphRunSource { variable_digits: true, ..Default::default() }))
    }

    /// The glyph runs drawn by `draw`: the x of their origin and the glyph
    /// (its character, or the text of a shaped run).
    fn capture(draw: impl FnOnce(&mut MockDrawingContext)) -> Vec<(f64, String)> {
        let mut context = MockDrawingContext::default();
        draw(&mut context);
        context
            .log
            .iter()
            .filter_map(|line| {
                let (glyph, rest) = if let Some(rest) = line.strip_prefix("glyph ") {
                    let (code, rest) = rest.split_once(' ').unwrap();
                    (char::from_u32(code.parse::<u32>().unwrap()).unwrap().to_string(), rest)
                } else {
                    let rest = line.strip_prefix("text ")?;
                    let at = rest.rfind(" at ").unwrap();
                    let (text, _) = rest[..at].rsplit_once(' ').unwrap();
                    (text.to_string(), &rest[at..])
                };
                let at = rest.rfind(" at ").unwrap();
                let x = rest[at + 4..].split(',').next().unwrap().parse::<f64>().unwrap();
                Some((x, glyph))
            })
            .collect()
    }

    fn draw(renderer: &DiagnosticTextRenderer, text: &str) -> Vec<f64> {
        capture(|context| renderer.draw_ascii_text(context, text, &*Brushes::white()))
            .into_iter()
            .map(|(x, _)| x)
            .collect()
    }

    fn assert_close(expected: f64, actual: f64) {
        assert!((expected - actual).abs() < 1e-6, "expected {expected}, actual {actual}");
    }

    // --- Ports of upstream's `Rendering/DiagnosticTextRendererTests.cs`.

    #[test]
    fn digits_measure_to_the_same_width() {
        let renderer = create_variable_digits_renderer();

        let width = renderer.measure_ascii_text("0").width;

        for c in '1'..='9' {
            assert_eq!(width, renderer.measure_ascii_text(&c.to_string()).width);
        }

        assert_eq!(width * 3.0, renderer.measure_ascii_text("108").width);
    }

    #[test]
    fn height_is_the_scaled_line_spacing() {
        use crate::media::text_formatting::testing::TextTestScope;

        let _scope = TextTestScope::new();

        let glyph_typeface = Typeface::default_typeface().glyph_typeface();
        let renderer = DiagnosticTextRenderer::from_glyph_typeface(glyph_typeface.clone(), 12.0);
        let metrics = glyph_typeface.metrics();
        let expected = metrics.line_spacing() as f64 * 12.0 / metrics.design_em_height as f64;

        assert_eq!(expected, renderer.get_max_height());
        assert_eq!(expected, renderer.measure_ascii_text("FPS: 60").height);
    }

    #[test]
    fn digits_are_centered_inside_the_digit_cell() {
        let source = Rc::new(MockGlyphRunSource { variable_digits: true, ..Default::default() });
        let renderer = DiagnosticTextRenderer::new(source.clone());
        let cell_width = renderer.measure_ascii_text("0").width;

        // Narrow digits shift to the center of their cell, they must never leave it.
        const DIGITS: &str = "1234567890";
        let drawn = draw(&renderer, DIGITS);

        assert_eq!(DIGITS.len(), drawn.len());

        for (i, digit) in DIGITS.chars().enumerate() {
            let cell_start = i as f64 * cell_width;
            let glyph_width = source.width(digit);

            assert!(drawn[i] >= cell_start - 1e-6, "Digit {i} starts before its cell.");
            assert!(drawn[i] + glyph_width <= cell_start + cell_width + 1e-6, "Digit {i} ends after its cell.");
            assert_close(cell_start + (cell_width - glyph_width) / 2.0, drawn[i]);
        }
    }

    #[test]
    fn text_after_digits_does_not_depend_on_the_digit_values() {
        let renderer = create_variable_digits_renderer();

        let narrow = draw(&renderer, "111 ms");
        let wide = draw(&renderer, "888 ms");

        assert_eq!(narrow.len(), wide.len());
        assert_close(*narrow.last().unwrap(), *wide.last().unwrap());
    }

    #[test]
    fn draw_digits_matches_drawing_the_padded_text() {
        let rows: [(i32, i32, &str); 5] =
            [(42, 5, "00042"), (0, 3, "000"), (12345678, 8, "12345678"), (1000, 3, "999"), (-5, 3, "000")];

        for (value, digit_count, expected_text) in rows {
            let renderer = create_variable_digits_renderer();

            let digits = capture(|c| renderer.draw_digits(c, value, digit_count, &*Brushes::white()));
            let text = capture(|c| renderer.draw_ascii_text(c, expected_text, &*Brushes::white()));

            assert_eq!(text.len(), digits.len(), "{value} {digit_count}");

            for i in 0..text.len() {
                assert_eq!(text[i].1, digits[i].1, "{value} {digit_count}: glyph {i}");
                assert_close(text[i].0, digits[i].0);
            }
        }
    }

    #[test]
    fn shaped_text_is_measured_once_and_drawn_as_a_single_run() {
        let renderer = create_variable_digits_renderer();
        let shaped = renderer.shape(" FPS: ");

        assert_close(renderer.measure_ascii_text(" FPS: ").width, shaped.width());
        assert_eq!(1, capture(|c| renderer.draw_shaped_text(c, &shaped, &*Brushes::white())).len());

        shaped.dispose();
    }

    #[test]
    fn fps_counter_layout_does_not_depend_on_the_numbers() {
        let counter = FpsCounter::new(Rc::new(create_variable_digits_renderer()), Duration::ZERO);
        let mut rects = Vec::new();

        let first = capture(|c| {
            rects.push(counter.render_fps(c, Some((1000, 1000)), 1, 1, false, None, Duration::from_millis(10)))
        });
        let second = capture(|c| {
            rects.push(counter.render_fps(c, Some((1000, 1000)), 99999, 88888, false, None, Duration::from_millis(20)))
        });

        assert_eq!(rects[0], rects[1]);
        assert_eq!(first.len(), second.len());

        // Labels, and every digit cell, stay where they were no matter which digits are shown.
        for i in 0..first.len() {
            if first[i].1 == second[i].1 {
                assert_close(first[i].0, second[i].0);
            }
        }
    }

    // --- Not upstream tests: the seam to the text stack and the mock.

    #[test]
    fn creates_one_run_per_printable_ascii_char_in_order() {
        let source = Rc::new(MockGlyphRunSource::default());
        let _renderer = DiagnosticTextRenderer::new(source.clone());
        let requested = source.requested.borrow();
        assert_eq!(95, requested.len());
        assert_eq!(' ', requested[0]);
        assert_eq!('~', requested[94]);
        assert!(requested.windows(2).all(|w| w[0] as u32 + 1 == w[1] as u32));
    }

    #[test]
    fn glyph_runs_are_built_from_a_glyph_typeface_and_an_em_size() {
        use crate::media::text_formatting::testing::{advance, TextTestScope};
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

        // A text is one run of its glyphs.
        let text: Vec<u16> = "FPS".encode_utf16().collect();
        let text_run = source.create_text_glyph_run(&text);
        assert_eq!(text_run.bounds.width, 3.0 * advance(12.0));

        let renderer = DiagnosticTextRenderer::from_glyph_typeface(glyph_typeface, 12.0);
        assert_eq!(3.0 * advance(12.0), renderer.measure_ascii_text("FPS").width);
        assert_eq!(renderer.shape("FPS").width(), renderer.measure_ascii_text("FPS").width);

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
    fn max_height_is_the_line_height() {
        assert_eq!(14.0, create_renderer().get_max_height());
    }

    #[test]
    fn measures_widths_and_the_line_height() {
        let renderer = create_renderer();
        assert_eq!(Size::new(0.0, 14.0), renderer.measure_ascii_text(""));
        assert_eq!(Size::new(18.0, 14.0), renderer.measure_ascii_text("abc"));
        assert_eq!(Size::new(15.0, 14.0), renderer.measure_ascii_text("a|b"));
        // Non-printable and non-ASCII characters count as spaces; characters
        // outside the BMP are two UTF-16 units and so two spaces.
        assert_eq!(Size::new(12.0, 14.0), renderer.measure_ascii_text("\n\u{e9}"));
        assert_eq!(Size::new(12.0, 14.0), renderer.measure_ascii_text("\u{1F600}"));
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

    #[test]
    fn an_empty_shaped_text_draws_nothing() {
        let renderer = create_renderer();
        let shaped = renderer.shape("");

        assert_eq!(0.0, shaped.width());
        assert!(capture(|c| renderer.draw_shaped_text(c, &shaped, &*Brushes::white())).is_empty());
    }
}
