use std::cell::{Cell, OnceCell};
use std::rc::Rc;

use crate::media::glyph_run::GlyphInfoList;
use crate::media::text_formatting::text_run::{text_run_any, TextRun};
use crate::media::text_formatting::{
    DrawableTextRun, ITextDrawingSink, ShapedBuffer, SplitResult, TabAlignment, TabLeader, TextMetrics,
    TextRunProperties,
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
    tab_leader: TabLeader,
    tab_alignment: Option<TabAlignment>,
    tab_presence: Cell<Option<bool>>,
}

impl ShapedTextRun {
    pub fn new(shaped_buffer: Rc<ShapedBuffer>, properties: Rc<dyn TextRunProperties>) -> Self {
        let text_metrics = TextMetrics::new(&properties.cached_glyph_typeface(), properties.effective_font_rendering_em_size());

        Self {
            glyph_run: OnceCell::new(),
            ref_count: Cell::new(1),
            shaped_buffer,
            properties,
            text_metrics,
            tab_leader: TabLeader::Spaces,
            tab_alignment: None,
            tab_presence: Cell::new(None),
        }
    }

    pub(super) fn with_tab_layout(
        shaped_buffer: Rc<ShapedBuffer>,
        properties: Rc<dyn TextRunProperties>,
        leader: TabLeader,
        alignment: TabAlignment,
    ) -> Self {
        let mut result = Self::new(shaped_buffer, properties);
        result.tab_leader = leader;
        result.tab_alignment = Some(alignment);
        result.tab_presence.set(Some(true));
        result
    }

    pub(super) fn contains_tab(&self) -> bool {
        if let Some(value) = self.tab_presence.get() {
            return value;
        }
        let value = self.text_span().contains(&(b'\t' as u16));
        self.tab_presence.set(Some(value));
        value
    }

    pub(super) fn set_tab_presence(&self, value: bool) {
        self.tab_presence.set(Some(value));
    }

    pub(super) fn tab_alignment(&self) -> Option<TabAlignment> {
        self.tab_alignment
    }

    /// Preserves tab layout when an already shaped run is copied or split. The
    /// marker belongs only to the one-character tab run, never adjacent text.
    pub(super) fn with_shaped_buffer(&self, buffer: Rc<ShapedBuffer>) -> Self {
        let mut result = Self::new(buffer, self.properties.clone());
        if self.tab_alignment.is_some() && result.text_span() == [b'\t' as u16] {
            result.tab_leader = self.tab_leader;
            result.tab_alignment = self.tab_alignment;
            result.set_tab_presence(true);
        } else if self.tab_presence.get() == Some(false) {
            result.set_tab_presence(false);
        }
        result
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
    // Internal upstream; public so the Skia unit tests reach it.
    pub fn add_ref(self: &Rc<Self>) -> Rc<Self> {
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
        let (length, width) = self
            .shaped_buffer
            .find_trailing_char_count_within_width(available_width);

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

        let first = Rc::new(self.with_shaped_buffer(split_buffer.first.expect("a split always has a first part")));

        if first.length() < length {
            panic!("Split length too small.");
        }

        match split_buffer.second {
            None => SplitResult::new(Some(first), None),
            Some(second) => {
                let second = Rc::new(self.with_shaped_buffer(second));
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
        self.draw_tab_leader(drawing_context, foreground_brush);

        let Some(text_decorations) = self.properties.text_decorations() else {
            return;
        };

        for text_decoration in text_decorations.iter() {
            text_decoration.draw(drawing_context, glyph_run, &self.text_metrics, foreground_brush);
        }
    }
    fn draw_tab_leader(&self, sink: &mut dyn ITextDrawingSink, brush: &Rc<dyn crate::media::IBrush>) {
        use crate::media::immutable::ImmutableDashStyle;
        use crate::media::{IDashStyle, IPen, Pen, PenLineCap, PenLineJoin};
        if self.tab_leader == TabLeader::Spaces {
            return;
        }
        let width = self.shaped_buffer.total_glyph_advance();
        if width <= 0.0 || !width.is_finite() {
            return;
        }
        let normal = self
            .text_metrics
            .underline_thickness
            .abs()
            .max(self.text_metrics.font_rendering_em_size / 16.0);
        let thickness = if self.tab_leader == TabLeader::ThickLines {
            normal * 2.0
        } else {
            normal
        };
        let (pattern, cap): (Option<Rc<dyn IDashStyle>>, _) = match self.tab_leader {
            TabLeader::Dots => (
                Some(Rc::new(ImmutableDashStyle::new(Some(&[0.0, 3.0]), 0.0))),
                PenLineCap::Round,
            ),
            TabLeader::Dashes => (
                Some(Rc::new(ImmutableDashStyle::new(Some(&[3.0, 2.0]), 0.0))),
                PenLineCap::Flat,
            ),
            _ => (None, PenLineCap::Flat),
        };
        let pen: Rc<dyn IPen> =
            Pen::with_all(Some(brush.clone()), thickness, pattern, cap, PenLineJoin::Miter, 10.0).into();
        let inset = (normal * 1.5).min(width * 0.5);
        let baseline = self.baseline();
        let y = if matches!(self.tab_leader, TabLeader::Dashes | TabLeader::Equals) {
            baseline - self.text_metrics.font_rendering_em_size * 0.3
        } else {
            baseline + self.text_metrics.underline_position
        };
        sink.draw_line(&pen, Point::new(inset, y), Point::new(width - inset, y));
        if self.tab_leader == TabLeader::Equals {
            sink.draw_line(
                &pen,
                Point::new(inset, y + normal * 3.0),
                Point::new(width - inset, y + normal * 3.0),
            );
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
