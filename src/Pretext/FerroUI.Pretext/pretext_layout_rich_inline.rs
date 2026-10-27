//! Rich inline text and atomic items, prepared once for repeated layout.

use std::collections::HashMap;
use std::rc::Rc;

use crate::{
    LayoutCursor, LayoutLineRange, PrepareOptions, PreparedTextWithSegments, PretextLayout,
};
use ferroui_pretext_contracts::PretextError;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RichInlineBreakMode {
    #[default]
    Normal,
    Never,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RichInlineItem {
    pub text: String,
    pub font: String,
    pub break_mode: RichInlineBreakMode,
    pub extra_width: f64,
}

impl RichInlineItem {
    pub fn new(text: impl Into<String>, font: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font: font.into(),
            break_mode: RichInlineBreakMode::Normal,
            extra_width: 0.0,
        }
    }

    pub fn with_break_mode(mut self, break_mode: RichInlineBreakMode) -> Self {
        self.break_mode = break_mode;
        self
    }

    pub fn with_extra_width(mut self, extra_width: f64) -> Self {
        self.extra_width = extra_width;
        self
    }
}

/// The item index addresses the prepared flow; fragment item indices address the
/// original source item list, which may contain empty or whitespace-only items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RichInlineCursor {
    pub item_index: usize,
    pub segment_index: usize,
    pub grapheme_index: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RichInlineStats {
    pub line_count: usize,
    pub max_line_width: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RichInlineFragment {
    pub item_index: usize,
    pub text: String,
    pub gap_before: f64,
    pub occupied_width: f64,
    pub start: LayoutCursor,
    pub end: LayoutCursor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RichInlineFragmentRange {
    pub item_index: usize,
    pub gap_before: f64,
    pub occupied_width: f64,
    pub start: LayoutCursor,
    pub end: LayoutCursor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RichInlineLine {
    pub fragments: Vec<RichInlineFragment>,
    pub width: f64,
    pub end: RichInlineCursor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RichInlineLineRange {
    pub fragments: Vec<RichInlineFragmentRange>,
    pub width: f64,
    pub end: RichInlineCursor,
}

/// Immutable, reusable rich inline preparation. Keep this value alive while
/// retaining any geometry-only ranges which refer to its source item indices.
#[derive(Debug)]
pub struct PreparedRichInline {
    pub(crate) items: Vec<PreparedRichInlineItem>,
    pub(crate) items_by_source_item_index: Vec<Option<usize>>,
}

impl PreparedRichInline {
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn source_item_count(&self) -> usize {
        self.items_by_source_item_index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Obtain the prepared text for an original source item. Whitespace-only
    /// source items have no prepared text, but may contribute a collapsed gap.
    pub fn prepared_item(&self, source_item_index: usize) -> Option<&Rc<PreparedTextWithSegments>> {
        self.items_by_source_item_index
            .get(source_item_index)
            .copied()
            .flatten()
            .map(|index| &self.items[index].prepared)
    }
}

#[derive(Debug)]
pub(crate) struct PreparedRichInlineItem {
    pub(crate) break_mode: RichInlineBreakMode,
    pub(crate) end_grapheme_index: usize,
    pub(crate) end_segment_index: usize,
    pub(crate) extra_width: f64,
    pub(crate) gap_before: f64,
    pub(crate) natural_width: f64,
    pub(crate) prepared: Rc<PreparedTextWithSegments>,
    pub(crate) source_item_index: usize,
}

impl PreparedRichInlineItem {
    fn end(&self) -> LayoutCursor {
        LayoutCursor {
            segment_index: self.end_segment_index,
            grapheme_index: self.end_grapheme_index,
        }
    }
}

const EMPTY_LAYOUT_CURSOR: LayoutCursor = LayoutCursor {
    segment_index: 0,
    grapheme_index: 0,
};

impl PretextLayout {
    pub fn prepare_rich_inline(
        &self,
        items: &[RichInlineItem],
    ) -> Result<Rc<PreparedRichInline>, PretextError> {
        let mut prepared_items = Vec::with_capacity(items.len());
        let mut items_by_source_item_index = vec![None; items.len()];
        let mut space_width_cache = HashMap::new();
        let mut pending_gap_width = 0.0;

        for (index, item) in items.iter().enumerate() {
            let trimmed = trim_collapsible_boundary_text(&item.text);
            if trimmed.text.is_empty() {
                if (trimmed.has_leading_whitespace || trimmed.has_trailing_whitespace)
                    && pending_gap_width == 0.0
                {
                    pending_gap_width =
                        self.collapsed_space_width(&item.font, &mut space_width_cache)?;
                }
                continue;
            }

            let gap_before = if pending_gap_width > 0.0 {
                pending_gap_width
            } else if trimmed.has_leading_whitespace {
                self.collapsed_space_width(&item.font, &mut space_width_cache)?
            } else {
                0.0
            };
            let prepared =
                self.prepare_with_segments(trimmed.text, &item.font, PrepareOptions::default())?;
            let whole_line =
                Self::layout_next_line_range(&prepared, EMPTY_LAYOUT_CURSOR, f64::INFINITY);
            if let Some(whole_line) = whole_line {
                items_by_source_item_index[index] = Some(prepared_items.len());
                prepared_items.push(PreparedRichInlineItem {
                    break_mode: item.break_mode,
                    end_grapheme_index: whole_line.end.grapheme_index,
                    end_segment_index: whole_line.end.segment_index,
                    extra_width: item.extra_width,
                    gap_before,
                    natural_width: whole_line.width,
                    prepared,
                    source_item_index: index,
                });
            }

            pending_gap_width = if trimmed.has_trailing_whitespace {
                self.collapsed_space_width(&item.font, &mut space_width_cache)?
            } else {
                0.0
            };
        }

        Ok(Rc::new(PreparedRichInline {
            items: prepared_items,
            items_by_source_item_index,
        }))
    }

    pub fn layout_next_rich_inline_line_range(
        prepared: &PreparedRichInline,
        max_width: f64,
        start: Option<RichInlineCursor>,
    ) -> Option<RichInlineLineRange> {
        let mut end = start.unwrap_or_default();
        let mut fragments = Vec::new();
        let width = step_rich_inline_line(
            prepared,
            max_width,
            &mut end,
            |item, gap_before, occupied_width, start, end| {
                fragments.push(RichInlineFragmentRange {
                    item_index: item.source_item_index,
                    gap_before,
                    occupied_width,
                    start,
                    end,
                });
            },
        )?;
        Some(RichInlineLineRange {
            fragments,
            width,
            end,
        })
    }

    /// Materialize a range produced from this same prepared flow.
    ///
    /// Panics if a caller fabricates a fragment referencing a missing source item,
    /// matching the source's invalid-operation contract for mismatched ranges.
    pub fn materialize_rich_inline_line_range(
        prepared: &PreparedRichInline,
        line: &RichInlineLineRange,
    ) -> RichInlineLine {
        let fragments = line
            .fragments
            .iter()
            .map(|fragment| {
                let item = prepared
                    .prepared_item(fragment.item_index)
                    .expect("rich inline range references a missing source item");
                let materialized = Self::materialize_line_range(
                    item,
                    &LayoutLineRange {
                        width: fragment.occupied_width,
                        start: fragment.start,
                        end: fragment.end,
                    },
                );
                RichInlineFragment {
                    item_index: fragment.item_index,
                    text: materialized.text,
                    gap_before: fragment.gap_before,
                    occupied_width: fragment.occupied_width,
                    start: fragment.start,
                    end: fragment.end,
                }
            })
            .collect();
        RichInlineLine {
            fragments,
            width: line.width,
            end: line.end,
        }
    }

    pub fn walk_rich_inline_line_ranges(
        prepared: &PreparedRichInline,
        max_width: f64,
        mut on_line: impl FnMut(RichInlineLineRange),
    ) -> usize {
        let mut cursor = RichInlineCursor::default();
        let mut line_count = 0;
        while let Some(line) =
            Self::layout_next_rich_inline_line_range(prepared, max_width, Some(cursor))
        {
            cursor = line.end;
            on_line(line);
            line_count += 1;
        }
        line_count
    }

    /// Compute metrics without allocating fragment vectors or materialized text.
    pub fn measure_rich_inline_stats(
        prepared: &PreparedRichInline,
        max_width: f64,
    ) -> RichInlineStats {
        let mut cursor = RichInlineCursor::default();
        let mut stats = RichInlineStats::default();
        while let Some(width) =
            step_rich_inline_line(prepared, max_width, &mut cursor, |_, _, _, _, _| {})
        {
            stats.line_count += 1;
            stats.max_line_width = stats.max_line_width.max(width);
        }
        stats
    }

    fn collapsed_space_width(
        &self,
        font: &str,
        cache: &mut HashMap<String, f64>,
    ) -> Result<f64, PretextError> {
        if let Some(width) = cache.get(font) {
            return Ok(*width);
        }
        let joined = self.prepare_with_segments("A A", font, PrepareOptions::default())?;
        let compact = self.prepare_with_segments("AA", font, PrepareOptions::default())?;
        let width =
            (Self::measure_natural_width(&joined) - Self::measure_natural_width(&compact)).max(0.0);
        cache.insert(font.to_owned(), width);
        Ok(width)
    }
}

struct BoundaryTrimResult<'a> {
    text: &'a str,
    has_leading_whitespace: bool,
    has_trailing_whitespace: bool,
}

fn trim_collapsible_boundary_text(text: &str) -> BoundaryTrimResult<'_> {
    let without_start = text.trim_start_matches(is_collapsible_boundary_char);
    let trimmed = without_start.trim_end_matches(is_collapsible_boundary_char);
    BoundaryTrimResult {
        text: trimmed,
        has_leading_whitespace: without_start.len() < text.len(),
        // Whitespace-only source items contribute a pending gap on either edge.
        has_trailing_whitespace: text.ends_with(is_collapsible_boundary_char),
    }
}

fn is_collapsible_boundary_char(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\u{000c}' | '\r')
}

/// Shared stepping engine. The no-op metrics callback monomorphizes away and
/// therefore shares the exact wrapping decisions without constructing fragments.
fn step_rich_inline_line(
    prepared: &PreparedRichInline,
    max_width: f64,
    cursor: &mut RichInlineCursor,
    mut on_fragment: impl FnMut(&PreparedRichInlineItem, f64, f64, LayoutCursor, LayoutCursor),
) -> Option<f64> {
    let items = &prepared.items;
    if cursor.item_index >= items.len() {
        return None;
    }
    let safe_width = if max_width > 1.0 { max_width } else { 1.0 };
    let mut line_width = 0.0;
    let mut has_fragment = false;
    let mut remaining_width = safe_width;
    let mut item_index = cursor.item_index;
    let mut text_cursor = LayoutCursor {
        segment_index: cursor.segment_index,
        grapheme_index: cursor.grapheme_index,
    };

    while let Some(item) = items.get(item_index) {
        if text_cursor != EMPTY_LAYOUT_CURSOR && text_cursor == item.end() {
            item_index += 1;
            text_cursor = EMPTY_LAYOUT_CURSOR;
            continue;
        }

        let gap_before = if line_width == 0.0 {
            0.0
        } else {
            item.gap_before
        };
        let at_item_start = text_cursor == EMPTY_LAYOUT_CURSOR;
        if item.break_mode == RichInlineBreakMode::Never {
            if !at_item_start {
                item_index += 1;
                text_cursor = EMPTY_LAYOUT_CURSOR;
                continue;
            }
            let occupied_width = item.natural_width + item.extra_width;
            let total_width = gap_before + occupied_width;
            if line_width > 0.0 && total_width > remaining_width {
                break;
            }
            on_fragment(
                item,
                gap_before,
                occupied_width,
                EMPTY_LAYOUT_CURSOR,
                item.end(),
            );
            has_fragment = true;
            line_width += total_width;
            remaining_width = (safe_width - line_width).max(0.0);
            item_index += 1;
            text_cursor = EMPTY_LAYOUT_CURSOR;
            continue;
        }

        let reserved_width = gap_before + item.extra_width;
        if line_width > 0.0 && reserved_width >= remaining_width {
            break;
        }

        if at_item_start {
            let total_width = reserved_width + item.natural_width;
            if total_width <= remaining_width {
                on_fragment(
                    item,
                    gap_before,
                    item.natural_width + item.extra_width,
                    EMPTY_LAYOUT_CURSOR,
                    item.end(),
                );
                has_fragment = true;
                line_width += total_width;
                remaining_width = (safe_width - line_width).max(0.0);
                item_index += 1;
                text_cursor = EMPTY_LAYOUT_CURSOR;
                continue;
            }
        }

        let available_width = (remaining_width - reserved_width).max(1.0);
        let Some(line) =
            PretextLayout::layout_next_line_range(&item.prepared, text_cursor, available_width)
        else {
            item_index += 1;
            text_cursor = EMPTY_LAYOUT_CURSOR;
            continue;
        };
        if text_cursor == line.end {
            item_index += 1;
            text_cursor = EMPTY_LAYOUT_CURSOR;
            continue;
        }

        if line_width > 0.0
            && at_item_start
            && gap_before > 0.0
            && line.end.segment_index == 0
            && line.end.grapheme_index > 0
        {
            let fresh_line = PretextLayout::layout_next_line_range(
                &item.prepared,
                EMPTY_LAYOUT_CURSOR,
                (safe_width - item.extra_width).max(1.0),
            );
            if fresh_line.is_some_and(|fresh| fresh.end > line.end) {
                break;
            }
        }

        on_fragment(
            item,
            gap_before,
            line.width + item.extra_width,
            text_cursor,
            line.end,
        );
        has_fragment = true;
        line_width += gap_before + line.width + item.extra_width;
        remaining_width = (safe_width - line_width).max(0.0);
        if line.end == item.end() {
            item_index += 1;
            text_cursor = EMPTY_LAYOUT_CURSOR;
            continue;
        }
        text_cursor = line.end;
        break;
    }

    // Visible source text can legitimately measure zero. The source tested
    // width == 0 here, which discarded that text; fragment presence determines
    // whether a line exists independently of its painted advance.
    if !has_fragment {
        return None;
    }
    *cursor = RichInlineCursor {
        item_index,
        segment_index: text_cursor.segment_index,
        grapheme_index: text_cursor.grapheme_index,
    };
    Some(line_width)
}
