use std::cell::OnceCell;
use std::ops::Deref;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WhiteSpaceMode {
    #[default]
    Normal,
    PreWrap,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WordBreakMode {
    #[default]
    Normal,
    KeepAll,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SegmentBreakKind {
    Text,
    Space,
    PreservedSpace,
    Tab,
    Glue,
    ZeroWidthBreak,
    SoftHyphen,
    HardBreak,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PrepareOptions {
    pub white_space: WhiteSpaceMode,
    pub word_break: WordBreakMode,
}

/// A position in prepared text, independent of UTF-8 and UTF-16 encoding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayoutCursor {
    pub segment_index: usize,
    pub grapheme_index: usize,
}

impl LayoutCursor {
    pub const fn new(segment_index: usize, grapheme_index: usize) -> Self {
        Self { segment_index, grapheme_index }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutResult {
    pub line_count: usize,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LineStats {
    pub line_count: usize,
    pub max_line_width: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PrepareProfile {
    pub analysis_ms: f64,
    pub measure_ms: f64,
    pub total_ms: f64,
    pub analysis_segments: usize,
    pub prepared_segments: usize,
    pub breakable_segments: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparedLineChunk {
    pub start_segment_index: usize,
    pub end_segment_index: usize,
    pub consumed_end_segment_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutLine {
    pub text: String,
    pub width: f64,
    pub start: LayoutCursor,
    pub end: LayoutCursor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutLineRange {
    pub width: f64,
    pub start: LayoutCursor,
    pub end: LayoutCursor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutLinesResult {
    pub line_count: usize,
    pub height: f64,
    pub lines: Vec<LayoutLine>,
}

/// Immutable metrics required by allocation-free line walking.
#[derive(Clone, Debug)]
pub struct PreparedText {
    pub(crate) font: String,
    pub(crate) white_space: WhiteSpaceMode,
    pub(crate) widths: Vec<f64>,
    pub(crate) line_end_fit_advances: Vec<f64>,
    pub(crate) line_end_paint_advances: Vec<f64>,
    pub(crate) kinds: Vec<SegmentBreakKind>,
    pub(crate) breakable_widths: Vec<Option<Vec<f64>>>,
    pub(crate) breakable_prefix_widths: Vec<Option<Vec<f64>>>,
    pub(crate) discretionary_hyphen_width: f64,
    pub(crate) tab_stop_advance: f64,
    pub(crate) chunks: Vec<PreparedLineChunk>,
    pub(crate) simple_line_walk_fast_path: bool,
}

impl PreparedText {
    pub fn font(&self) -> &str { &self.font }
    pub fn white_space(&self) -> WhiteSpaceMode { self.white_space }
    pub fn discretionary_hyphen_width(&self) -> f64 { self.discretionary_hyphen_width }
    pub fn tab_stop_advance(&self) -> f64 { self.tab_stop_advance }
    pub fn widths(&self) -> &[f64] { &self.widths }
    pub fn line_end_fit_advances(&self) -> &[f64] { &self.line_end_fit_advances }
    pub fn line_end_paint_advances(&self) -> &[f64] { &self.line_end_paint_advances }
    pub fn kinds(&self) -> &[SegmentBreakKind] { &self.kinds }
    pub fn breakable_widths(&self) -> &[Option<Vec<f64>>] { &self.breakable_widths }
    pub fn breakable_prefix_widths(&self) -> &[Option<Vec<f64>>] { &self.breakable_prefix_widths }
    pub fn chunks(&self) -> &[PreparedLineChunk] { &self.chunks }
    pub fn simple_line_walk_fast_path(&self) -> bool { self.simple_line_walk_fast_path }
}

/// Prepared metrics with source segments for lazy text and glyph materialization.
#[derive(Debug)]
pub struct PreparedTextWithSegments {
    pub(crate) base: PreparedText,
    pub(crate) segments: Vec<String>,
    pub(crate) segment_levels: Option<Vec<i8>>,
    pub(crate) grapheme_offsets: Vec<OnceCell<Vec<usize>>>,
}

impl PreparedTextWithSegments {
    pub fn segments(&self) -> &[String] { &self.segments }
    pub fn segment_levels(&self) -> Option<&[i8]> { self.segment_levels.as_deref() }
}

impl Deref for PreparedTextWithSegments {
    type Target = PreparedText;

    fn deref(&self) -> &Self::Target { &self.base }
}
