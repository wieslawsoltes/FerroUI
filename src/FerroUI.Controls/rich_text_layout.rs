//! Block layout and reusable page continuations over the framework text formatter.
//! Source/dependency mapping and validation: docs/porting/winui-rich-text.

use crate::documents::{
    Block, EmbeddedControlRun, Inline, LineSpacingRule, LineStackingStrategy, Paragraph,
    RichTextDocument, TextHighlightRegion, Typography,
};
use crate::text_block::InlinesTextSource;
use ferroui_base::media::text_formatting::TextRun;
use ferroui_base::media::text_formatting::{
    FontFamilyLineMetrics, GenericTextParagraphProperties, GenericTextRunProperties,
    ITextDrawingSink, InterWordJustification, LogicalTextRunEnumerator, TextCollapsingProperties,
    TextEndOfParagraph, TextFormatter, TextFormatterImpl, TextLine, TextLineBreak, TextLineImpl,
    TextLineMetricsMode, TextParagraphProperties, TextRunCache, TextRunProperties, TextTabStops,
};
use ferroui_base::media::{
    BaselineAlignment, CharacterHit, DrawingContext, GlyphRun, IBrush, IPen, Pen, TextAlignment,
    TextCollapsingCreateInfo, TextLineBounds, TextTrimming, TextWrapping, Typeface,
};
use ferroui_base::utilities::ValueSpan;
use ferroui_base::{Matrix, Point, Rect, Ref, Size, Vector};
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

pub(crate) struct RichParagraphSource {
    pub block: Ref<Block>,
    pub source: InlinesTextSource,
    pub offset: usize,
    pub length: usize,
    pub properties: Rc<dyn TextRunProperties>,
    pub default_tab_stop: f64,
    /// The shaping cache of the paragraph, shared with the next source while the paragraph
    /// does not change (F1, #243 review S7).
    pub cache: Rc<TextRunCache>,
    /// The formatting overrides of the paragraph (highlighters), relative to it. The selection
    /// is not one: it is drawn over the lines (`RichTextLayout::draw_with_selection`, #439).
    modifiers: Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>>,
    /// The lines of the whole paragraph from its last layout, shared with the next source while
    /// the paragraph does not change: a page that formats it again with the same width and
    /// paragraph properties takes them instead (F1).
    lines: Rc<RefCell<Option<ParagraphLines>>>,
    has_embedded_controls: bool,
}

/// What a paragraph's lines were formatted with, apart from the runs (shared with the source).
#[derive(Clone, PartialEq)]
struct ParagraphLinesKey {
    width: u64,
    wrapping: TextWrapping,
    first_indent: u64,
    rtl: bool,
    alignment: TextAlignment,
    line_height: u64,
    strategy: LineStackingStrategy,
    spacing_rule: LineSpacingRule,
    spacing: u64,
    letter_spacing: u64,
    tabs: TextTabStops,
    margin_left: u64,
    margin_right: u64,
    default_indent: u64,
    text_line_bounds: TextLineBounds,
}

struct CachedParagraphLine {
    line: Rc<dyn TextLine>,
    advance: f64,
    offset: f64,
    origin_x: f64,
    right_margin: f64,
    use_line_advance_for_bounds: bool,
    extent: f64,
}

struct ParagraphLines {
    key: ParagraphLinesKey,
    lines: Vec<CachedParagraphLine>,
}

impl Drop for ParagraphLines {
    fn drop(&mut self) {
        for cached in &self.lines {
            if Rc::strong_count(&cached.line) == 1 {
                cached.line.dispose();
            }
        }
    }
}

impl RichParagraphSource {
    /// Whether this paragraph of an earlier source formats as the runs, properties and overrides
    /// of a new one: then its runs and shaping cache serve the new source.
    fn formats_as(
        &self,
        runs: &[Rc<dyn TextRun>],
        properties: &Rc<dyn TextRunProperties>,
        default_tab_stop: f64,
        modifiers: &Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>>,
    ) -> bool {
        fn same_properties(a: &Rc<dyn TextRunProperties>, b: &Rc<dyn TextRunProperties>) -> bool {
            **a == **b && a.baseline_alignment() == b.baseline_alignment()
        }
        let old_runs = self.source.text_runs();
        !self.has_embedded_controls
            && self.default_tab_stop.to_bits() == default_tab_stop.to_bits()
            && same_properties(&self.properties, properties)
            && old_runs.len() == runs.len()
            && old_runs.iter().zip(runs).all(|(old, new)| {
                old.as_any().type_id() == new.as_any().type_id()
                    && old.length() == new.length()
                    && old.text_span() == new.text_span()
                    && match (old.properties(), new.properties()) {
                        (None, None) => true,
                        (Some(a), Some(b)) => same_properties(a, b),
                        _ => false,
                    }
            })
            && match (&self.modifiers, modifiers) {
                (None, None) => true,
                (Some(a), Some(b)) => {
                    a.len() == b.len()
                        && a.iter().zip(b.iter()).all(|(a, b)| {
                            a.start() == b.start() && a.length() == b.length() && same_properties(a.value(), b.value())
                        })
                }
                _ => false,
            }
    }
}

/// A revision of the document's formatter input, shared by all overflow views.
pub(crate) struct RichTextSource {
    pub paragraphs: Vec<RichParagraphSource>,
    pub length: usize,
    pub default_indent: f64,
    pub text_line_bounds: TextLineBounds,
    pub highlights: Vec<TextHighlightRegion>,
    pub collapse_paragraph_margins: bool,
    has_embedded_controls: bool,
}

impl RichTextSource {
    /// The source of the document for the view. The paragraphs of `previous` (the source the
    /// view had before its document changed) that format as before keep their runs and shaping
    /// caches, so an edit reshapes the paragraphs it changed, not the document (F1, #243 review
    /// S7). The selection is not part of the source: a selection change keeps it (#439).
    pub fn new(document: &RichTextDocument, view: &crate::RichTextBlock, previous: Option<&RichTextSource>) -> Rc<Self> {
        let previous_paragraphs: FxHashMap<usize, &RichParagraphSource> = previous
            .map(|previous| {
                previous
                    .paragraphs
                    .iter()
                    .map(|paragraph| (&*paragraph.block as *const Block as *const () as usize, paragraph))
                    .collect()
            })
            .unwrap_or_default();
        let mut blocks = document.blocks().snapshot().to_vec();
        // The empty editor still has a line with a measurable caret.
        let empty = blocks.is_empty();
        if empty {
            let paragraph = Paragraph::new();
            paragraph.set_flow_direction(view.flow_direction());
            paragraph.set_text_alignment(view.text_alignment());
            paragraph.set_line_height(view.line_height());
            paragraph.set_line_stacking_strategy(view.line_stacking_strategy());
            paragraph.set_letter_spacing(view.letter_spacing());
            blocks.push(paragraph.upcast());
        }
        let mut offset = 0;
        let highlights = view.highlight_regions();
        let mut paragraphs = Vec::with_capacity(blocks.len());
        for block in blocks {
            let mut runs = Vec::new();
            block.build_text_runs(&mut runs);
            for run in &mut runs {
                if let Some(embedded) = run.downcast_ref::<EmbeddedControlRun>() {
                    *run = Rc::new(embedded.with_measured_metrics());
                }
            }
            let length = block.text_length();
            let properties: Rc<dyn TextRunProperties> = if empty {
                Rc::new(GenericTextRunProperties::with_all(
                    Typeface::with_style(
                        view.font_family(),
                        view.font_style(),
                        view.font_weight(),
                        view.font_stretch(),
                    ),
                    view.font_size(),
                    view.text_decorations(),
                    view.foreground(),
                    None,
                    BaselineAlignment::Baseline,
                    Some(crate::documents::text_language::resolved_language(
                        &view.language(),
                    )),
                    Typography::effective_font_features(view),
                ))
            } else {
                Rc::new(GenericTextRunProperties::with_all(
                    Typeface::with_style(
                        block.font_family(),
                        block.font_style(),
                        block.font_weight(),
                        block.font_stretch(),
                    ),
                    block.font_size(),
                    block.get_value(Inline::text_decorations_property()),
                    block.foreground(),
                    block.background(),
                    BaselineAlignment::Baseline,
                    Some(block.resolved_language()),
                    Typography::effective_font_features(&block),
                ))
            };
            let default_tab_stop = block
                .to_ref()
                .cast::<Paragraph>()
                .filter(|paragraph| {
                    paragraph.is_set(Paragraph::default_tab_stop_property().as_property())
                })
                .map_or_else(
                    || view.default_incremental_tab(properties.font_rendering_em_size()),
                    |paragraph| paragraph.default_tab_stop(),
                );
            let modifiers = formatting_overrides(&runs, offset, &highlights);
            let has_embedded_controls = runs.iter().any(|run| run.is::<EmbeddedControlRun>());
            let unchanged = (!empty && !has_embedded_controls)
                .then(|| previous_paragraphs.get(&(&*block as *const Block as *const () as usize)))
                .flatten()
                .filter(|old| old.formats_as(&runs, &properties, default_tab_stop, &modifiers));
            if let Some(old) = unchanged {
                paragraphs.push(RichParagraphSource {
                    block,
                    source: old.source.clone(),
                    offset,
                    length,
                    properties: old.properties.clone(),
                    default_tab_stop,
                    cache: old.cache.clone(),
                    modifiers: old.modifiers.clone(),
                    lines: old.lines.clone(),
                    has_embedded_controls,
                });
            } else {
                paragraphs.push(RichParagraphSource {
                    block,
                    source: InlinesTextSource::new(Rc::new(runs), modifiers.clone()),
                    offset,
                    length,
                    properties,
                    default_tab_stop,
                    cache: Rc::new(TextRunCache::new()),
                    modifiers,
                    lines: Rc::new(RefCell::new(None)),
                    has_embedded_controls,
                });
            }
            offset += length + 1;
        }
        let has_embedded_controls = paragraphs
            .iter()
            .any(|paragraph| paragraph.has_embedded_controls);
        Rc::new(Self {
            paragraphs,
            length: document.text_length(),
            default_indent: view.text_indent(),
            text_line_bounds: view.text_line_bounds(),
            highlights,
            collapse_paragraph_margins: view.collapses_paragraph_margins(),
            has_embedded_controls,
        })
    }
}

/// The highlighters of a paragraph as formatting overrides of its runs: the foreground of the
/// highlight, and no background (the highlight's background is drawn behind the text). The
/// selection is drawn over the lines instead (`RichTextLayout::draw_with_selection`).
fn formatting_overrides(
    runs: &[Rc<dyn TextRun>],
    paragraph_offset: usize,
    highlights: &[TextHighlightRegion],
) -> Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>> {
    if highlights.is_empty() {
        return None;
    }
    let mut modifiers = Vec::new();
    let mut position = paragraph_offset;
    for run in runs {
        let end = position + run.length().max(0) as usize;
        if let Some(properties) = run.properties() {
            let mut boundaries = vec![position, end];
            let regions: Vec<_> = highlights
                .iter()
                .filter(|region| region.range.start < end && region.range.end > position)
                .collect();
            for region in &regions {
                boundaries.push(region.range.start.max(position));
                boundaries.push(region.range.end.min(end));
            }
            boundaries.sort_unstable();
            boundaries.dedup();
            for boundary in boundaries.windows(2) {
                let start = boundary[0];
                let end = boundary[1];
                let Some(highlight) = regions
                    .iter()
                    .find(|region| region.range.start <= start && region.range.end >= end)
                else {
                    continue;
                };
                let foreground = highlight
                    .foreground
                    .clone()
                    .or_else(|| properties.foreground_brush().cloned());
                let override_properties: Rc<dyn TextRunProperties> =
                    Rc::new(GenericTextRunProperties::with_all(
                        properties.typeface().clone(),
                        properties.font_rendering_em_size(),
                        properties.text_decorations().cloned(),
                        foreground,
                        None,
                        properties.baseline_alignment(),
                        properties.culture_info().cloned(),
                        properties.font_features().cloned(),
                    ));
                let (Ok(start), Ok(length)) = (
                    i32::try_from(start - paragraph_offset),
                    i32::try_from(end - start),
                ) else {
                    continue;
                };
                modifiers.push(ValueSpan::new(start, length, override_properties));
            }
        }
        position = end;
    }
    (!modifiers.is_empty()).then(|| modifiers.into())
}

impl Drop for RichTextSource {
    fn drop(&mut self) {
        for paragraph in &self.paragraphs {
            // A cache the next source took over stays alive with it.
            if Rc::strong_count(&paragraph.cache) == 1 {
                paragraph.cache.dispose();
            }
        }
    }
}

/// State at a page boundary. The break keeps the shaped, bidi-resolved remainder
/// of the paragraph; each reflow consumes a fork, never the stored continuation.
#[derive(Clone, Default)]
pub(crate) struct RichTextBreak {
    pub paragraph: usize,
    pub local_position: usize,
    pub position: usize,
    pub previous_line_break: Option<Rc<TextLineBreak>>,
}

/// Public metrics in document UTF-16 coordinates and view-local pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct RichTextLineMetrics {
    pub start: usize,
    pub length: usize,
    /// The allocated line space used by layout and caret geometry.
    pub bounds: Rect,
    /// The baseline after the glyph box has been positioned within the line.
    pub baseline: f64,
}

pub(crate) struct RichTextLine {
    pub line: Rc<dyn TextLine>,
    pub paragraph_offset: usize,
    pub paragraph_length: usize,
    /// Glyph origin; line stacking may place it outside the allocated line.
    pub origin: Point,
    pub advance: f64,
    /// The allocation top, excluding the glyph-box offset.
    pub(crate) allocation_y: f64,
    use_line_advance_for_bounds: bool,
    right_margin: f64,
    source_spans: Option<Vec<LineSourceSpan>>,
}

/// Retained source positions and their compacted formatter indices. The
/// synthetic ellipsis is deliberately absent from these spans.
struct LineSourceSpan {
    source: Range<usize>,
    display: Range<usize>,
}

impl RichTextLine {
    pub fn start(&self) -> usize {
        self.paragraph_offset + self.line.first_text_source_index().max(0) as usize
    }

    pub fn end(&self) -> usize {
        (self.start() + self.line.length().max(0) as usize)
            .min(self.paragraph_offset + self.paragraph_length)
    }

    /// Places formatter range bounds in this layout's allocation space.
    /// Native block bounds fill the line advance; explicit rich-edit spacing
    /// retains the formatter's natural range height.
    pub(crate) fn range_bounds(&self, rectangle: Rect) -> Rect {
        let mut rectangle = rectangle.translate(Vector::new(self.origin.x, self.allocation_y));
        if self.use_line_advance_for_bounds {
            rectangle.height = self.advance;
        }
        rectangle
    }

    pub(crate) fn visible_ranges(&self) -> Vec<Range<usize>> {
        match &self.source_spans {
            None => vec![self.start()..self.end()],
            Some(spans) => spans
                .iter()
                .filter_map(|span| {
                    let start = (self.paragraph_offset + span.source.start).max(self.start());
                    let end = (self.paragraph_offset + span.source.end).min(self.end());
                    (start < end).then_some(start..end)
                })
                .collect(),
        }
    }

    fn source_position(&self, display: usize) -> usize {
        let Some(spans) = &self.source_spans else {
            return display;
        };
        for span in spans {
            if display >= span.display.start && display <= span.display.end {
                return span.source.start + display - span.display.start;
            }
        }
        spans
            .iter()
            .rev()
            .find(|span| span.display.end <= display)
            .map(|span| span.source.end)
            .or_else(|| spans.first().map(|span| span.source.start))
            .unwrap_or(self.line.first_text_source_index().max(0) as usize)
    }

    pub(crate) fn display_position(&self, source: usize) -> usize {
        let Some(spans) = &self.source_spans else {
            return source;
        };
        for span in spans {
            if source >= span.source.start && source <= span.source.end {
                return span.display.start + source - span.source.start;
            }
        }
        spans
            .iter()
            .rev()
            .find(|span| span.source.end <= source)
            .map(|span| span.display.end)
            .or_else(|| spans.first().map(|span| span.display.start))
            .unwrap_or(self.line.first_text_source_index().max(0) as usize)
    }
}

fn collapse_line(
    line: Rc<dyn TextLine>,
    properties: Rc<dyn TextCollapsingProperties>,
) -> (Rc<dyn TextLine>, Option<Vec<LineSourceSpan>>) {
    let mut originals = Vec::new();
    let mut cursor = line.first_text_source_index().max(0) as usize;
    let mut enumerator = LogicalTextRunEnumerator::new(&*line);
    while let Some(run) = enumerator.move_next() {
        let end = cursor + run.length().max(0) as usize;
        originals.push((cursor..end, run.text().clone(), run));
        cursor = end;
    }
    drop(enumerator);
    // The formatter transfers retained shaped-run ownership during collapse,
    // just as BaseTextLayout does. Disposing the old line would release it twice.
    let collapsed = line.collapse(&[Some(properties)]);
    if !collapsed.has_collapsed() {
        return (collapsed, None);
    }
    let mut spans = Vec::new();
    let mut cursor = collapsed.first_text_source_index().max(0) as usize;
    let mut enumerator = LogicalTextRunEnumerator::new(&*collapsed);
    while let Some(run) = enumerator.move_next() {
        let length = run.length().max(0) as usize;
        let text = run.text();
        let original = originals.iter().find_map(|(range, memory, original)| {
            if Rc::ptr_eq(original, &run) {
                return Some(range.start..range.start + length);
            }
            if length > 0
                && !text.is_empty()
                && memory.shares_owner_with(&text)
                && text.offset_in_owner() >= memory.offset_in_owner()
            {
                let skip = text.offset_in_owner() - memory.offset_in_owner();
                if skip + length <= range.len() {
                    return Some(range.start + skip..range.start + skip + length);
                }
            }
            None
        });
        if let Some(source) = original {
            spans.push(LineSourceSpan {
                source,
                display: cursor..cursor + length,
            });
        }
        cursor += length;
    }
    drop(enumerator);
    (collapsed, Some(spans))
}

impl Drop for RichTextLine {
    fn drop(&mut self) {
        // A line a paragraph's line cache or a later layout still holds stays alive.
        if Rc::strong_count(&self.line) == 1 {
            self.line.dispose();
        }
    }
}

pub(crate) struct RichTextLayout {
    // Retain input and caches until the last line that uses them is dropped.
    pub lines: Vec<RichTextLine>,
    pub source: Rc<RichTextSource>,
    pub size: Size,
    pub next: RichTextBreak,
    pub has_overflow: bool,
    pub is_trimmed: bool,
    hosted_objects_measured: bool,
}

/// Object runs are snapshots of one paragraph fragment's host. They cannot
/// enter the document-wide shaping cache: a later page has another width and
/// remaining height, and must not overwrite the preceding page's geometry.
struct HostedParagraphSource {
    source: InlinesTextSource,
    objects: FxHashMap<Ref<crate::Control>, Rc<dyn TextRun>>,
}

impl HostedParagraphSource {
    fn new(
        paragraph: &RichParagraphSource,
        local_start: usize,
        available_size: Size,
        host: Option<(&crate::RichTextBlock, u64)>,
    ) -> Option<Self> {
        if !paragraph.has_embedded_controls {
            return None;
        }
        let mut position = 0usize;
        let mut objects = FxHashMap::default();
        let runs = paragraph
            .source
            .text_runs()
            .iter()
            .map(|run| {
                let start = position;
                position = position.saturating_add(run.length().max(0) as usize);
                let Some(embedded) = run.downcast_ref::<EmbeddedControlRun>() else {
                    return run.clone();
                };
                if start < local_start {
                    return run.clone();
                }
                let can_capture = if let Some((host, generation)) = host {
                    if host.page_layout_generation() != generation {
                        return run.clone();
                    }
                    host.measure_hosted_control(embedded.control(), available_size, generation)
                } else {
                    false
                };
                let measured: Rc<dyn TextRun> = Rc::new(if can_capture {
                    embedded.with_hosted_measurement()
                } else {
                    embedded.with_cached_hosted_metrics()
                });
                objects.insert(embedded.control().clone(), measured.clone());
                measured
            })
            .collect();
        Some(Self {
            source: paragraph.source.with_text_runs(Rc::new(runs)),
            objects,
        })
    }

    fn replace_object(&self, run: &Rc<dyn TextRun>) -> Option<Rc<dyn TextRun>> {
        let embedded = run.downcast_ref::<EmbeddedControlRun>()?;
        self.objects.get(embedded.control()).cloned()
    }
}

/// The line's allocation and the position of its natural glyph box within it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct LineStackingMetrics {
    advance: f64,
    offset: f64,
}

/// Port of ParagraphNode::ApplyLineStackingStrategy at the pinned WinUI source.
/// The first line starts this page's paragraph fragment; the last line reaches
/// the paragraph end, rather than merely a page or MaxLines boundary.
fn apply_line_stacking_strategy(
    strategy: LineStackingStrategy,
    font_baseline_ratio: f64,
    line_height: f64,
    natural_height: f64,
    first_line: bool,
    last_line: bool,
) -> LineStackingMetrics {
    // FerroUI represents automatic height with NaN. Preserve its unbounded
    // height fallback too; neither sentinel is a concrete line height.
    if !line_height.is_finite() || line_height <= 0.0 {
        return LineStackingMetrics {
            advance: natural_height,
            offset: 0.0,
        };
    }
    match strategy {
        LineStackingStrategy::MaxHeight => LineStackingMetrics {
            advance: natural_height.max(line_height),
            offset: 0.0,
        },
        LineStackingStrategy::BlockLineHeight => LineStackingMetrics {
            advance: line_height,
            offset: (line_height - natural_height) * font_baseline_ratio,
        },
        LineStackingStrategy::BaselineToBaseline => {
            let mut metrics = LineStackingMetrics {
                advance: line_height,
                offset: (line_height - natural_height) * font_baseline_ratio,
            };
            if first_line {
                metrics.advance -= metrics.offset;
                metrics.offset = 0.0;
            } else if last_line {
                metrics.advance -= (line_height - natural_height) * (1.0 - font_baseline_ratio);
            }
            metrics
        }
    }
}

impl RichTextLayout {
    pub fn new(
        source: Rc<RichTextSource>,
        constraint: Size,
        wrapping: TextWrapping,
        max_lines: usize,
        trimming: &Rc<dyn TextTrimming>,
        terminal: bool,
        start: &RichTextBreak,
        host: Option<(&crate::RichTextBlock, u64)>,
    ) -> Rc<Self> {
        let formatter = <dyn TextFormatter>::current();
        let mut result = Self {
            lines: Vec::new(),
            source,
            size: Size::default(),
            next: start.clone(),
            has_overflow: false,
            is_trimmed: false,
            hosted_objects_measured: host.is_some(),
        };
        let width = if constraint.width.is_nan() {
            f64::INFINITY
        } else {
            constraint.width.max(0.0)
        };
        let height = if constraint.height.is_nan() {
            f64::INFINITY
        } else {
            constraint.height.max(0.0)
        };
        let mut y = 0.0_f64;
        let mut widest = 0.0_f64;
        let mut preceding_bottom_margin = 0.0_f64;

        'paragraphs: for paragraph_index in start.paragraph..result.source.paragraphs.len() {
            let paragraph = &result.source.paragraphs[paragraph_index];
            let block = &paragraph.block;
            let block_margin = block.margin();
            let mut margin = block_margin;
            if let Some(paragraph) = block.to_ref().cast::<Paragraph>() {
                margin.left += paragraph.left_indent();
                margin.right += paragraph.right_indent();
            }
            let mut local = if paragraph_index == start.paragraph {
                start.local_position
            } else {
                0
            };
            let mut previous = if paragraph_index == start.paragraph {
                start.previous_line_break.clone()
            } else {
                None
            };
            let mut previous_is_shared = previous.is_some();
            // A full page does not visit or measure objects in the next block.
            if !result.lines.is_empty()
                && ((height.is_finite() && y >= height)
                    || (max_lines > 0 && result.lines.len() >= max_lines))
            {
                result.next = RichTextBreak {
                    paragraph: paragraph_index,
                    local_position: local,
                    position: paragraph.offset + local,
                    previous_line_break: previous,
                };
                result.has_overflow = true;
                break;
            }
            let paragraph_y = y;
            if result.source.collapse_paragraph_margins {
                // BlockNode::Measure suppresses the first page margin, carries
                // only the preceding bottom margin, and applies their maximum.
                margin.top = if result.lines.is_empty() {
                    0.0
                } else {
                    margin.top.max(preceding_bottom_margin)
                };
                margin.bottom = 0.0;
                y += margin.top;
            } else if local == 0 {
                y += margin.top;
            }
            let paragraph_width = (width - margin.left - margin.right).max(0.0);
            // The hosted object receives the paragraph's fixed available
            // measure size, independent of pen position and first-line indent.
            let hosted = HostedParagraphSource::new(
                paragraph,
                local,
                Size::new(
                    (width - block_margin.left - block_margin.right).max(0.0),
                    (height - y - margin.bottom).max(0.0),
                ),
                host,
            );
            // A whole paragraph that fits on the page: the lines of its last layout serve again
            // while its runs (the source keeps them), its width and its properties are the same.
            let lines_key = (local == 0
                && previous.is_none()
                && hosted.is_none()
                && max_lines == 0
                && Rc::ptr_eq(trimming, &<dyn TextTrimming>::none())
                && !paragraph.has_embedded_controls)
                .then(|| {
                    let rich = block.to_ref().cast::<Paragraph>();
                    let (spacing_rule, spacing) = rich
                        .as_ref()
                        .map(|p| (p.line_spacing_rule(), p.line_spacing()))
                        .unwrap_or((LineSpacingRule::Undefined, 1.0));
                    ParagraphLinesKey {
                        width: paragraph_width.to_bits(),
                        wrapping,
                        first_indent: rich
                            .as_ref()
                            .filter(|p| p.is_set(Paragraph::text_indent_property().as_property()))
                            .map_or(result.source.default_indent, |p| p.text_indent())
                            .to_bits(),
                        rtl: block.flow_direction() == ferroui_base::media::FlowDirection::RightToLeft,
                        alignment: block.text_alignment(),
                        line_height: block.line_height().to_bits(),
                        strategy: block.line_stacking_strategy(),
                        spacing_rule,
                        spacing: spacing.to_bits(),
                        letter_spacing: block.letter_spacing().to_bits(),
                        tabs: rich.as_ref().map_or_else(TextTabStops::default, |p| p.tab_stops()),
                        margin_left: margin.left.to_bits(),
                        margin_right: margin.right.to_bits(),
                        default_indent: result.source.default_indent.to_bits(),
                        text_line_bounds: result.source.text_line_bounds,
                    }
                });
            if let Some(key) = &lines_key {
                let cached = paragraph.lines.borrow();
                let fits = |cached: &&ParagraphLines| {
                    !height.is_finite()
                        || y + cached.lines.iter().map(|entry| entry.advance.max(f64::MIN_POSITIVE)).sum::<f64>()
                            <= height + 1e-8
                };
                if let Some(cached) = cached.as_ref().filter(|cached| &cached.key == key).filter(fits) {
                    for entry in &cached.lines {
                        widest = widest.max(entry.extent);
                        result.lines.push(RichTextLine {
                            line: entry.line.clone(),
                            paragraph_offset: paragraph.offset,
                            paragraph_length: paragraph.length,
                            origin: Point::new(entry.origin_x, y + entry.offset),
                            advance: entry.advance,
                            allocation_y: y,
                            use_line_advance_for_bounds: entry.use_line_advance_for_bounds,
                            right_margin: entry.right_margin,
                            source_spans: None,
                        });
                        y += entry.advance;
                    }
                    y += margin.bottom;
                    preceding_bottom_margin = block_margin.bottom;
                    result.next = RichTextBreak {
                        paragraph: paragraph_index + 1,
                        local_position: 0,
                        position: (paragraph.offset + paragraph.length + 1).min(result.source.length),
                        previous_line_break: None,
                    };
                    continue 'paragraphs;
                }
            }
            let mut recorded: Option<Vec<CachedParagraphLine>> = lines_key.as_ref().map(|_| Vec::new());
            let line_height = block.line_height();
            let strategy = block.line_stacking_strategy();
            let (spacing_rule, spacing) = block
                .to_ref()
                .cast::<Paragraph>()
                .map(|paragraph| (paragraph.line_spacing_rule(), paragraph.line_spacing()))
                .unwrap_or((LineSpacingRule::Undefined, 1.0));
            // The native block layout uses the regular face of the paragraph's
            // font family, not the tallest run or the current glyph baseline.
            let font_baseline_ratio = if spacing_rule == LineSpacingRule::Undefined
                && strategy != LineStackingStrategy::MaxHeight
                && line_height.is_finite()
                && line_height > 0.0
            {
                let metrics = FontFamilyLineMetrics::new(
                    paragraph.properties.requested_font_family(),
                    paragraph.properties.font_rendering_em_size(),
                    result.source.text_line_bounds,
                );
                if metrics.height > 0.0 {
                    metrics.baseline / metrics.height
                } else {
                    0.0
                }
            } else {
                0.0
            };
            // A continuation starts a new paragraph fragment. Its first line
            // gets the same leading-space adjustment as a new paragraph.
            let mut first_line = true;

            loop {
                let indent = if local == 0 {
                    block
                        .to_ref()
                        .cast::<Paragraph>()
                        .filter(|p| p.is_set(Paragraph::text_indent_property().as_property()))
                        .map_or(result.source.default_indent, |p| p.text_indent())
                } else {
                    0.0
                };
                let rtl = block.flow_direction() == ferroui_base::media::FlowDirection::RightToLeft;
                let line_origin_x = margin.left + if rtl { 0.0 } else { indent };
                let line_right_margin = margin.right + if rtl { indent } else { 0.0 };
                // Indent is placed outside the formatter so both wrapping and
                // embedded controls use the same available width and origin.
                let line_width = (paragraph_width - indent).max(0.0);
                let properties = GenericTextParagraphProperties::with_all(
                    block.flow_direction(),
                    block.text_alignment(),
                    local == 0,
                    false,
                    paragraph.properties.clone(),
                    wrapping,
                    if spacing_rule == LineSpacingRule::Exactly {
                        spacing
                    } else {
                        // LineStackingStrategy positions the natural glyph
                        // box after formatting; it does not resize that box.
                        f64::NAN
                    },
                    0.0,
                    block.letter_spacing(),
                )
                .with_tabs(
                    block
                        .to_ref()
                        .cast::<Paragraph>()
                        .map_or_else(TextTabStops::default, |paragraph| paragraph.tab_stops()),
                    paragraph.default_tab_stop,
                    indent + if rtl { margin.right } else { margin.left },
                );
                properties.set_text_line_bounds(result.source.text_line_bounds);
                properties.set_line_metrics_mode(TextLineMetricsMode::FontFamily);
                let properties: Rc<dyn TextParagraphProperties> = Rc::new(properties);
                let input_break = previous.as_ref().map(|value| {
                    if first_line {
                        if let Some(hosted) = &hosted {
                            return value.fork_with_object_runs(|run| hosted.replace_object(run));
                        }
                    }
                    value.fork()
                });
                let Some(index) = i32::try_from(local).ok() else {
                    result.has_overflow = true;
                    break 'paragraphs;
                };
                let formatted = formatter.format_line_with_cache(
                    hosted
                        .as_ref()
                        .map_or(&paragraph.source, |hosted| &hosted.source),
                    index,
                    line_width.max(f64::MIN_POSITIVE),
                    &properties,
                    input_break.as_ref(),
                    hosted.is_none().then_some(&paragraph.cache),
                );
                let empty_line = formatted.is_none();
                let mut line = formatted.unwrap_or_else(|| {
                    TextFormatterImpl::create_empty_text_line(index, line_width, &properties)
                });
                let line_break = line.text_line_break();
                let end_of_paragraph = empty_line
                    || line_break.as_ref().is_some_and(|b| {
                        b.text_end_of_line()
                            .is_some_and(|run| run.is::<TextEndOfParagraph>())
                    });
                let stacking = if spacing_rule != LineSpacingRule::Undefined {
                    LineStackingMetrics {
                        advance: spacing_rule.advance(line.height(), spacing),
                        offset: 0.0,
                    }
                } else {
                    apply_line_stacking_strategy(
                        strategy,
                        font_baseline_ratio,
                        line_height,
                        line.height(),
                        first_line,
                        end_of_paragraph,
                    )
                };
                let advance = stacking.advance.max(f64::MIN_POSITIVE);
                let origin_y = y + stacking.offset;
                // A first oversized line is accepted to guarantee forward progress
                // through columns smaller than a font's metrics. The control clips it.
                if !result.lines.is_empty()
                    && ((height.is_finite() && y + advance > height + 1e-8)
                        || (max_lines > 0 && result.lines.len() >= max_lines))
                {
                    line.dispose();
                    if first_line {
                        // An empty paragraph fragment contributes no margin.
                        y = paragraph_y;
                    }
                    result.next = RichTextBreak {
                        paragraph: paragraph_index,
                        local_position: local,
                        position: paragraph.offset + local,
                        previous_line_break: previous,
                    };
                    result.has_overflow = true;
                    break 'paragraphs;
                }

                if !previous_is_shared {
                    if let Some(previous) = &previous {
                        previous.release_remaining_runs();
                    }
                }

                if block.text_alignment() == TextAlignment::Justify
                    && line_width.is_finite()
                    && !end_of_paragraph
                    && line.new_line_length() == 0
                {
                    line.justify(&InterWordJustification::new(line_width));
                }
                let consumed = line.length().max(0) as usize;
                let mut source_spans = None;
                if line.has_overflowed() && !Rc::ptr_eq(trimming, &<dyn TextTrimming>::none()) {
                    let collapsing =
                        trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
                            line_width,
                            paragraph.properties.clone(),
                            block.flow_direction(),
                        ));
                    (line, source_spans) = collapse_line(line, collapsing);
                }
                let extent = line_origin_x
                    + line.start()
                    + line.width_including_trailing_whitespace()
                    + line_right_margin;
                widest = widest.max(extent);
                if let Some(recorded) = &mut recorded {
                    recorded.push(CachedParagraphLine {
                        line: line.clone(),
                        advance,
                        offset: stacking.offset,
                        origin_x: line_origin_x,
                        right_margin: line_right_margin,
                        use_line_advance_for_bounds: spacing_rule == LineSpacingRule::Undefined,
                        extent,
                    });
                }
                result.lines.push(RichTextLine {
                    line,
                    paragraph_offset: paragraph.offset,
                    paragraph_length: paragraph.length,
                    origin: Point::new(line_origin_x, origin_y),
                    advance,
                    allocation_y: y,
                    use_line_advance_for_bounds: spacing_rule == LineSpacingRule::Undefined,
                    right_margin: line_right_margin,
                    source_spans,
                });
                y += advance;
                first_line = false;
                local = local.saturating_add(consumed).min(paragraph.length);
                previous = line_break;
                previous_is_shared = false;
                if end_of_paragraph || consumed == 0 {
                    if end_of_paragraph {
                        if let (Some(key), Some(lines)) = (lines_key.clone(), recorded.take()) {
                            *paragraph.lines.borrow_mut() = Some(ParagraphLines { key, lines });
                        }
                    }
                    y += margin.bottom;
                    preceding_bottom_margin = block_margin.bottom;
                    result.next = RichTextBreak {
                        paragraph: paragraph_index + 1,
                        local_position: 0,
                        position: (paragraph.offset + paragraph.length + 1)
                            .min(result.source.length),
                        previous_line_break: None,
                    };
                    break;
                }
            }
        }
        if result.has_overflow && terminal && !Rc::ptr_eq(trimming, &<dyn TextTrimming>::none()) {
            if let Some(entry) = result.lines.last_mut() {
                if !entry.line.has_collapsed() {
                    let paragraph = result
                        .source
                        .paragraphs
                        .iter()
                        .find(|paragraph| paragraph.offset == entry.paragraph_offset)
                        .unwrap();
                    let collapsing =
                        trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
                            entry.line.width_including_trailing_whitespace(),
                            paragraph.properties.clone(),
                            paragraph.block.flow_direction(),
                        ));
                    let (line, spans) = collapse_line(entry.line.clone(), collapsing);
                    entry.line = line;
                    entry.source_spans = spans;
                }
            }
        }
        result.is_trimmed = result.lines.iter().any(|entry| entry.line.has_collapsed());
        widest = result
            .lines
            .iter()
            .map(|entry| {
                entry.origin.x
                    + entry.line.start()
                    + entry.line.width_including_trailing_whitespace()
                    + entry.right_margin
            })
            .fold(0.0_f64, f64::max);
        result.size = Size::new(widest.max(0.0), y.max(0.0));
        Rc::new(result)
    }

    pub(crate) fn hosted_objects_are_current(&self) -> bool {
        if !self.source.has_embedded_controls {
            return true;
        }
        self.lines.iter().all(|line| {
            line.line.text_runs().iter().all(|run| {
                run.downcast_ref::<EmbeddedControlRun>()
                    .is_none_or(|embedded| {
                        self.hosted_objects_measured && embedded.measured_metrics_are_current()
                    })
            })
        })
    }

    pub fn draw(&self, sink: &mut dyn ITextDrawingSink, origin: Point) {
        for entry in &self.lines {
            entry.line.draw(
                sink,
                Point::new(origin.x + entry.origin.x, origin.y + entry.origin.y),
            );
        }
    }

    /// Draws the lines with the selected text over the selection highlight (#439): the runs are
    /// formatted without the selection, so a selection change keeps every paragraph's runs,
    /// shaping cache and lines. A line the selection touches is drawn through a
    /// `SelectionDrawingSink`, which gives the selected parts of its glyph runs and
    /// decorations the selection foreground and leaves out the run backgrounds under the
    /// highlight; the other lines are drawn as they are.
    pub(crate) fn draw_with_selection(
        &self,
        context: &mut DrawingContext<'_>,
        origin: Point,
        selection: Option<&SelectionOverlay>,
    ) {
        let Some(selection) = selection.filter(|selection| {
            !selection.range.is_empty()
                && (selection.foreground.is_some() || selection.hides_backgrounds)
        }) else {
            self.draw(context, origin);
            return;
        };
        let mut rects = Vec::new();
        let mut spans = Vec::new();
        for entry in &self.lines {
            let line_origin = Point::new(origin.x + entry.origin.x, origin.y + entry.origin.y);
            rects.clear();
            // Without its paragraph mark, a selection that ends before the line or starts after
            // it selects nothing on it.
            if selection.range.end > entry.start() && selection.range.start < entry.end() {
                self.line_selection_rects(
                    entry,
                    selection.range.start,
                    selection.range.end,
                    false,
                    &mut rects,
                );
            }
            if rects.is_empty() {
                entry.line.draw(&mut *context, line_origin);
                continue;
            }
            spans.clear();
            spans.extend(
                rects
                    .iter()
                    .map(|rect| origin.x + rect.x..origin.x + rect.right()),
            );
            merge_spans(&mut spans);
            let mut sink = SelectionDrawingSink::new(&mut *context, &spans, selection);
            entry.line.draw(&mut sink, line_origin);
        }
    }

    pub fn metrics(&self, origin: Point) -> Vec<RichTextLineMetrics> {
        self.lines
            .iter()
            .map(|entry| RichTextLineMetrics {
                start: entry.start(),
                length: entry.end().saturating_sub(entry.start()),
                bounds: Rect::new(
                    origin.x + entry.origin.x + entry.line.start(),
                    origin.y + entry.allocation_y,
                    entry.line.width_including_trailing_whitespace(),
                    entry.advance,
                ),
                baseline: origin.y + entry.origin.y + entry.line.baseline(),
            })
            .collect()
    }

    pub fn hit_test(&self, point: Point) -> usize {
        let Some(entry) = self
            .lines
            .iter()
            .find(|entry| point.y < entry.origin.y + entry.advance)
            .or_else(|| self.lines.last())
        else {
            return self.next.position;
        };
        let hit = entry
            .line
            .get_character_hit_from_distance(point.x - entry.origin.x);
        let position = hit
            .first_character_index()
            .saturating_add(hit.trailing_length())
            .max(0) as usize;
        (entry.paragraph_offset + entry.source_position(position))
            .min(entry.end())
            .min(self.source.length)
    }

    pub fn caret_rect(&self, position: usize) -> Rect {
        self.caret_rect_with_gravity(position, crate::documents::TextGravity::Forward)
    }

    pub fn caret_rect_with_gravity(
        &self,
        position: usize,
        gravity: crate::documents::TextGravity,
    ) -> Rect {
        let Some(entry) = self
            .lines
            .iter()
            .find(|entry| position >= entry.start() && position < entry.end())
            .or_else(|| {
                self.lines
                    .iter()
                    .rev()
                    .find(|entry| position >= entry.start())
            })
            .or_else(|| self.lines.first())
        else {
            return Rect::default();
        };
        let local = position
            .saturating_sub(entry.paragraph_offset)
            .min(entry.paragraph_length);
        let local = entry.display_position(local).min(i32::MAX as usize) as i32;
        let leading = CharacterHit::new(local);
        let hit = if gravity == crate::documents::TextGravity::Backward && position > entry.start()
        {
            let previous = entry.line.get_previous_caret_character_hit(leading);
            CharacterHit::with_trailing_length(
                previous.first_character_index(),
                local.saturating_sub(previous.first_character_index()),
            )
        } else {
            leading
        };
        let x = entry.line.get_distance_from_character_hit(hit);
        Rect::new(entry.origin.x + x, entry.allocation_y, 1.0, entry.advance)
    }

    pub fn selection_rects(&self, start: usize, end: usize) -> Vec<Rect> {
        let mut result = Vec::new();
        for entry in &self.lines {
            self.line_selection_rects(entry, start, end, true, &mut result);
        }
        result
    }

    /// The bounds of the text of `start..end` on one line, from the line's hit test, and with
    /// `paragraph_mark` the bounds of the selected paragraph mark at the end of the paragraph.
    fn line_selection_rects(
        &self,
        entry: &RichTextLine,
        start: usize,
        end: usize,
        paragraph_mark: bool,
        result: &mut Vec<Rect>,
    ) {
        for visible in entry.visible_ranges() {
            let a = start.max(visible.start);
            let b = end.min(visible.end);
            if a < b {
                let a = entry.display_position(a - entry.paragraph_offset);
                let b = entry.display_position(b - entry.paragraph_offset);
                for bounds in entry.line.get_text_bounds(a as i32, (b - a) as i32) {
                    result.push(entry.range_bounds(bounds.rectangle()));
                }
            }
        }
        let paragraph_end = entry.paragraph_offset + entry.paragraph_length;
        if paragraph_mark
            && start <= paragraph_end
            && end > paragraph_end
            && entry.end() == paragraph_end
            && (entry.source_spans.is_none()
                || entry
                    .visible_ranges()
                    .iter()
                    .any(|range| range.end == paragraph_end))
        {
            let caret = self.caret_rect(paragraph_end);
            result.push(Rect::new(caret.x, caret.y, 4.0, caret.height));
        }
    }

    pub fn visible_text_ranges(&self) -> Vec<Range<usize>> {
        let mut ranges: Vec<_> = self
            .lines
            .iter()
            .flat_map(RichTextLine::visible_ranges)
            .collect();
        ranges.sort_by_key(|range| range.start);
        let mut merged: Vec<Range<usize>> = Vec::new();
        for range in ranges {
            if let Some(last) = merged.last_mut() {
                if range.start <= last.end {
                    last.end = last.end.max(range.end);
                    continue;
                }
            }
            merged.push(range);
        }
        merged
    }

    pub fn embedded_controls(&self, origin: Point) -> Vec<(Ref<crate::Control>, Rect)> {
        let mut result = Vec::new();
        for entry in &self.lines {
            let mut x = origin.x + entry.origin.x + entry.line.start();
            for run in entry.line.text_runs().iter() {
                if let Some(drawable) = run.as_drawable() {
                    if let Some(embedded) = run.downcast_ref::<EmbeddedControlRun>() {
                        let child = embedded.control().clone();
                        let y = origin.y
                            + entry.origin.y
                            + TextLineImpl::get_baseline_offset(&*entry.line, drawable);
                        result.push((
                            child.clone(),
                            Rect::from_position_size(Point::new(x, y), drawable.size()),
                        ));
                    }
                    x += drawable.size().width;
                }
            }
        }
        result
    }
}

/// How the selected text of a view is drawn over its selection highlight (#439).
pub(crate) struct SelectionOverlay {
    /// The selected range, in document UTF-16 offsets.
    pub range: Range<usize>,
    /// The foreground of the selected glyphs and decorations; `None` keeps their own.
    pub foreground: Option<Rc<dyn IBrush>>,
    /// Whether the run backgrounds under the selection are left out, so that the selection
    /// highlight drawn behind the text shows.
    pub hides_backgrounds: bool,
}

/// Sorts spans and merges those that overlap or touch.
fn merge_spans(spans: &mut Vec<Range<f64>>) {
    spans.sort_by(|a, b| a.start.total_cmp(&b.start));
    let mut merged: Vec<Range<f64>> = Vec::with_capacity(spans.len());
    for span in spans.drain(..) {
        match merged.last_mut() {
            Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
            _ => merged.push(span),
        }
    }
    *spans = merged;
}

/// The sink a line the selection touches is drawn through (#439), the way WinUI's
/// `D2DTextDrawingContext::DrawGlyphRun` gives the highlighted part of a glyph run the
/// highlight's foreground: the runs are shaped and formatted without the selection, and the
/// sink splits what they draw at the selection's spans.
///
/// - A glyph run, and a decoration or tab leader drawn with the run's foreground, that the
///   spans cover in part are drawn once for each part, clipped to it (rectangle clips of the
///   drawing context), with the selection foreground in the selected parts.
/// - A run background (a filled rectangle without a pen) is drawn without the parts the spans
///   cover, when the selection highlight is to show through.
///
/// What the line draws apart from its runs (bar tab stops) and anything drawn under a
/// transform that is not a translation are drawn as they are.
struct SelectionDrawingSink<'c, 'a> {
    context: &'c mut DrawingContext<'a>,
    /// The selected spans of the line in the coordinates of the context: sorted and disjoint.
    spans: &'c [Range<f64>],
    foreground: Option<&'c Rc<dyn IBrush>>,
    hides_backgrounds: bool,
    /// The offset of each transform the line pushed, from the coordinates of the context;
    /// `None` under a transform that is not a translation.
    transforms: Vec<Option<Vector>>,
    /// The foreground of the glyph run drawn last in the current run's transform: the brush
    /// of its decorations and tab leader.
    run_foreground: Option<Rc<dyn IBrush>>,
}

impl<'c, 'a> SelectionDrawingSink<'c, 'a> {
    fn new(
        context: &'c mut DrawingContext<'a>,
        spans: &'c [Range<f64>],
        selection: &'c SelectionOverlay,
    ) -> Self {
        Self {
            context,
            spans,
            foreground: selection.foreground.as_ref(),
            hides_backgrounds: selection.hides_backgrounds,
            transforms: Vec::new(),
            run_foreground: None,
        }
    }

    /// The offset from local coordinates to those of the context.
    fn offset(&self) -> Option<Vector> {
        self.transforms
            .last()
            .copied()
            .unwrap_or(Some(Vector::new(0.0, 0.0)))
    }

    /// The parts of `left..right` (context coordinates) inside and outside the spans, in order.
    fn parts(&self, left: f64, right: f64) -> Vec<(f64, f64, bool)> {
        let mut parts = Vec::new();
        let mut x = left;
        for span in self
            .spans
            .iter()
            .filter(|span| span.start < right && span.end > left)
        {
            let start = span.start.max(x);
            let end = span.end.min(right);
            if start > x {
                parts.push((x, start, false));
            }
            parts.push((start, end, true));
            x = end;
        }
        if parts.is_empty() {
            return parts;
        }
        if x < right {
            parts.push((x, right, false));
        }
        parts
    }

    /// Draws something whose extent is `extent` (local coordinates) once if the spans cover all
    /// or none of it, and otherwise once for each part, clipped to the part and told whether
    /// the part is selected. The first and the last part reach past the extent, so that ink
    /// outside the advance (an italic overhang) belongs to the part at that edge.
    fn draw_split(&mut self, extent: Rect, mut draw: impl FnMut(&mut DrawingContext<'a>, bool)) {
        let Some(offset) = self.offset() else {
            draw(&mut *self.context, false);
            return;
        };
        let parts = self.parts(extent.x + offset.x, extent.right() + offset.x);
        match parts.as_slice() {
            [] => draw(&mut *self.context, false),
            [(_, _, selected)] => draw(&mut *self.context, *selected),
            _ => {
                let margin = 2.0 * extent.height.abs() + extent.width.abs() + 16.0;
                let last = parts.len() - 1;
                for (index, &(start, end, selected)) in parts.iter().enumerate() {
                    let start = if index == 0 { start - margin } else { start };
                    let end = if index == last { end + margin } else { end };
                    let clip = Rect::new(
                        start - offset.x,
                        extent.y - margin,
                        end - start,
                        extent.height + 2.0 * margin,
                    );
                    let state = self.context.push_clip(clip);
                    draw(&mut *self.context, selected);
                    self.context.pop(state);
                }
            }
        }
    }
}

impl ITextDrawingSink for SelectionDrawingSink<'_, '_> {
    fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        self.run_foreground = foreground.cloned();
        let Some(selected) = self.foreground.cloned() else {
            self.context.draw_glyph_run(foreground, glyph_run);
            return;
        };
        self.draw_split(glyph_run.bounds(), |context, is_selected| {
            context.draw_glyph_run(
                if is_selected {
                    Some(&selected)
                } else {
                    foreground
                },
                glyph_run,
            );
        });
    }

    fn draw_rectangle(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, rect: Rect) {
        let offset = self.offset();
        let (Some(offset), true, Some(_), None) =
            (offset, self.hides_backgrounds, brush, pen)
        else {
            ITextDrawingSink::draw_rectangle(&mut *self.context, brush, pen, rect);
            return;
        };
        let parts = self.parts(rect.x + offset.x, rect.right() + offset.x);
        if parts.is_empty() {
            ITextDrawingSink::draw_rectangle(&mut *self.context, brush, pen, rect);
            return;
        }
        for (start, end, selected) in parts {
            if !selected && end > start {
                ITextDrawingSink::draw_rectangle(
                    &mut *self.context,
                    brush,
                    None,
                    Rect::new(start - offset.x, rect.y, end - start, rect.height),
                );
            }
        }
    }

    fn draw_line(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        let selected_pen: Option<Rc<dyn IPen>> = match (self.foreground, &self.run_foreground) {
            (Some(selected), Some(run)) if !self.transforms.is_empty() => pen
                .brush()
                .is_some_and(|brush| Rc::ptr_eq(&brush, run))
                .then(|| {
                    let selected_pen: Rc<dyn IPen> = Pen::with_all(
                        Some(selected.clone()),
                        pen.thickness(),
                        pen.dash_style(),
                        pen.line_cap(),
                        pen.line_join(),
                        pen.miter_limit(),
                    )
                    .into();
                    selected_pen
                }),
            _ => None,
        };
        let Some(selected_pen) = selected_pen else {
            self.context.draw_line(pen, p1, p2);
            return;
        };
        let half = pen.thickness().abs() / 2.0;
        let extent = Rect::new(
            p1.x.min(p2.x),
            p1.y.min(p2.y) - half,
            (p1.x - p2.x).abs(),
            (p1.y - p2.y).abs() + 2.0 * half,
        );
        self.draw_split(extent, |context, is_selected| {
            context.draw_line(if is_selected { &selected_pen } else { pen }, p1, p2);
        });
    }

    fn push_transform(&mut self, matrix: Matrix) {
        let translation = matrix.m11 == 1.0
            && matrix.m12 == 0.0
            && matrix.m13 == 0.0
            && matrix.m21 == 0.0
            && matrix.m22 == 1.0
            && matrix.m23 == 0.0
            && matrix.m33 == 1.0;
        let offset = self
            .offset()
            .filter(|_| translation)
            .map(|offset| Vector::new(offset.x + matrix.m31, offset.y + matrix.m32));
        self.transforms.push(offset);
        self.run_foreground = None;
        ITextDrawingSink::push_transform(&mut *self.context, matrix);
    }

    fn pop_transform(&mut self) {
        self.transforms.pop();
        self.run_foreground = None;
        ITextDrawingSink::pop_transform(&mut *self.context);
    }
}

#[cfg(test)]
#[path = "rich_text_line_stacking_tests.rs"]
mod line_stacking_tests;

#[cfg(test)]
#[path = "rich_text_line_bounds_tests.rs"]
mod line_bounds_tests;

#[cfg(test)]
#[path = "rich_text_family_metrics_tests.rs"]
mod family_metrics_tests;

#[cfg(test)]
#[path = "rich_text_fallback_scale_tests.rs"]
mod fallback_scale_tests;
#[cfg(test)]
#[path = "rich_text_reuse_tests.rs"]
mod reuse_tests;

#[cfg(test)]
#[path = "rich_text_hosted_object_tests.rs"]
mod hosted_object_tests;
