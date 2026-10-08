use std::cell::{Cell, OnceCell};
use std::rc::Rc;

use crate::media::glyph_run::GlyphInfoList;
use crate::media::text_formatting::text_run::{text_run_any, TextRun};
use crate::media::text_formatting::{
    DrawableTextRun, ITextDrawingSink, ShapedBuffer, SplitResult, TextMetrics, TextRunProperties,
};
use crate::media::GlyphRun;
use crate::utilities::ReadOnlyMemory;
use crate::{Matrix, Point, Size};

/// A text run that holds shaped characters.
pub struct ShapedTextRun {
    glyph_run: OnceCell<Rc<GlyphRun>>,
    ref_count: Cell<i32>,
    shaped_buffer: Rc<ShapedBuffer>,
    properties: Rc<dyn TextRunProperties>,
    text_metrics: TextMetrics,
}

impl ShapedTextRun {
    pub fn new(shaped_buffer: Rc<ShapedBuffer>, properties: Rc<dyn TextRunProperties>) -> Self {
        let text_metrics = TextMetrics::new(&properties.cached_glyph_typeface(), properties.font_rendering_em_size());

        Self { glyph_run: OnceCell::new(), ref_count: Cell::new(1), shaped_buffer, properties, text_metrics }
    }

    /// The bidi level of the run.
    pub fn bidi_level(&self) -> i8 {
        self.shaped_buffer.bidi_level()
    }

    /// The shaped glyphs of the run.
    pub fn shaped_buffer(&self) -> &Rc<ShapedBuffer> {
        &self.shaped_buffer
    }

    /// The run's properties (never absent for a shaped run).
    pub fn run_properties(&self) -> &Rc<dyn TextRunProperties> {
        &self.properties
    }

    /// The metrics of the run's font at the run's em size.
    pub fn text_metrics(&self) -> TextMetrics {
        self.text_metrics
    }

    /// The glyph run of the shaped glyphs (created on first use).
    pub fn glyph_run(&self) -> &Rc<GlyphRun> {
        self.glyph_run.get_or_init(|| self.create_glyph_run())
    }

    /// Registers another owner of the run; every owner calls [`ShapedTextRun::dispose`] once.
    #[allow(dead_code)] // upstream's form; the formatter holds runs as `Rc<dyn TextRun>` and uses `add_reference`
    pub(crate) fn add_ref(self: &Rc<Self>) -> Rc<Self> {
        self.add_reference();
        self.clone()
    }

    /// Registers another owner of the run without producing a handle (for
    /// callers that already hold the run as `Rc<dyn TextRun>` and clone that).
    pub(crate) fn add_reference(&self) {
        self.ref_count.set(self.ref_count.get() + 1);
    }

    /// Measures the number of characters that fit into available width.
    pub fn try_measure_characters(&self, available_width: f64) -> Option<i32> {
        let length = self.shaped_buffer.find_leading_char_count_within_width(available_width);

        (length > 0).then_some(length)
    }

    /// Measures the number of characters that fit into available width when
    /// measuring from the end; returns the count and the width they take.
    ///
    /// Internal upstream; public so that the Skia unit tests reach it.
    pub fn try_measure_characters_backwards(&self, available_width: f64) -> Option<(i32, f64)> {
        let (length, width) = self.shaped_buffer.find_trailing_char_count_within_width(available_width);

        (length > 0).then_some((length, width))
    }

    /// Splits the run at `length` characters.
    ///
    /// Panics when `length` is zero or smaller than the first cluster.
    pub(crate) fn split(&self, length: i32) -> SplitResult<Rc<ShapedTextRun>> {
        if length == 0 {
            panic!("length must be greater than zero.");
        }

        let split_buffer = self.shaped_buffer.split(length);

        let first = Rc::new(ShapedTextRun::new(
            split_buffer.first.expect("a split always has a first part"),
            self.properties.clone(),
        ));

        if first.length() < length {
            panic!("Split length too small.");
        }

        match split_buffer.second {
            None => SplitResult::new(Some(first), None),
            Some(second) => {
                let second = Rc::new(ShapedTextRun::new(second, self.properties.clone()));
                SplitResult::new(Some(first), Some(second))
            }
        }
    }

    pub(crate) fn create_glyph_run(&self) -> Rc<GlyphRun> {
        GlyphRun::new(
            self.shaped_buffer.glyph_typeface().clone(),
            self.shaped_buffer.font_rendering_em_size(),
            self.shaped_buffer.text().clone(),
            GlyphInfoList::ShapedBuffer(self.shaped_buffer.clone()),
            None,
            self.bidi_level() as i32,
        )
    }

    /// Releases one reference; the glyph run and the shaped buffer are
    /// disposed with the last one.
    pub fn dispose(&self) {
        let ref_count = self.ref_count.get() - 1;
        self.ref_count.set(ref_count);

        if ref_count != 0 {
            return;
        }

        if let Some(glyph_run) = self.glyph_run.get() {
            glyph_run.dispose();
        }

        self.shaped_buffer.dispose();
    }

    fn draw_content(&self, drawing_context: &mut dyn ITextDrawingSink) {
        let glyph_run = self.glyph_run();

        if glyph_run.glyph_infos().count() == 0 {
            return;
        }

        // Upstream also returns when `Properties.Typeface == default`; a typeface
        // always has a font family here, so there is no such state.

        let Some(foreground_brush) = self.properties.foreground_brush() else {
            return;
        };

        if let Some(background_brush) = self.properties.background_brush() {
            drawing_context.draw_rectangle(Some(background_brush), None, glyph_run.bounds());
        }

        drawing_context.draw_glyph_run(Some(foreground_brush), glyph_run);

        let Some(text_decorations) = self.properties.text_decorations() else {
            return;
        };

        for text_decoration in text_decorations.iter() {
            text_decoration.draw(drawing_context, glyph_run, &self.text_metrics, foreground_brush);
        }
    }
}

impl TextRun for ShapedTextRun {
    fn length(&self) -> i32 {
        self.shaped_buffer.text().len() as i32
    }

    fn text(&self) -> ReadOnlyMemory<u16> {
        self.shaped_buffer.text().clone()
    }

    fn text_span(&self) -> &[u16] {
        self.shaped_buffer.text().span()
    }

    fn properties(&self) -> Option<&Rc<dyn TextRunProperties>> {
        Some(&self.properties)
    }

    fn as_drawable(&self) -> Option<&dyn DrawableTextRun> {
        Some(self)
    }

    text_run_any!();
}

impl DrawableTextRun for ShapedTextRun {
    fn size(&self) -> Size {
        self.glyph_run().bounds().size()
    }

    fn baseline(&self) -> f64 {
        -self.text_metrics.ascent + self.text_metrics.line_gap * 0.5
    }

    fn draw(&self, drawing_context: &mut dyn ITextDrawingSink, origin: Point) {
        drawing_context.push_transform(Matrix::create_translation(origin.x, origin.y));
        self.draw_content(drawing_context);
        drawing_context.pop_transform();
    }
}
