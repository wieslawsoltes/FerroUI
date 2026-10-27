//! Prepared glyph output, safe range slicing and exact-line shaping fallback.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::pretext_layout_line_text::{build_line_text_from_range, line_has_discretionary_hyphen, segment_grapheme_offsets};
use crate::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ShapedLayoutLine {
    pub shaped_run: Rc<PretextShapedRun>,
    pub width: f64,
    pub start: LayoutCursor,
    pub end: LayoutCursor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapedRichInlineFragment {
    pub item_index: usize,
    pub shaped_run: Rc<PretextShapedRun>,
    pub gap_before: f64,
    pub occupied_width: f64,
    pub start: LayoutCursor,
    pub end: LayoutCursor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapedRichInlineLine {
    pub fragments: Vec<ShapedRichInlineFragment>,
    pub width: f64,
    pub end: RichInlineCursor,
}

/// A prepared text object and its shaping snapshot. It retains its backend, so
/// clearing or replacing an engine's backend cannot invalidate retained lines.
pub struct PreparedShapedText {
    prepared: Rc<PreparedTextWithSegments>,
    shared: Rc<PreparedShapingData>,
}

// This payload never owns the prepared document: the engine may retain it while
// the source exists, without a reference cycle through the public wrapper.
struct PreparedShapingData {
    direction: PretextTextDirection,
    text: String,
    shaped_run: Rc<PretextShapedRun>,
    grapheme_boundary_text_indexes: Vec<Vec<usize>>,
    text_utf16_length: usize,
    state: Rc<ShaperState>,
    line_runs: RefCell<HashMap<(LayoutCursor, LayoutCursor), Rc<PretextShapedRun>>>,
}

pub(crate) struct PreparedShapedCacheEntry {
    prepared: Weak<PreparedTextWithSegments>,
    shared: Rc<PreparedShapingData>,
    wrapper: Weak<PreparedShapedText>,
}

impl PreparedShapedText {
    pub fn prepared(&self) -> &Rc<PreparedTextWithSegments> { &self.prepared }
    pub fn direction(&self) -> PretextTextDirection { self.shared.direction }
    pub fn options(&self) -> PretextShapeOptions { PretextShapeOptions { direction: self.shared.direction } }
    pub fn text(&self) -> &str { &self.shared.text }
    pub fn shaped_run(&self) -> &Rc<PretextShapedRun> { &self.shared.shaped_run }

    fn get_or_create_line_run(&self, start: LayoutCursor, end: LayoutCursor) -> Result<Rc<PretextShapedRun>, PretextError> {
        validate_range(&self.prepared, start, end)?;
        let key = (start, end);
        if let Some(cached) = self.shared.line_runs.borrow().get(&key) { return Ok(cached.clone()); }
        let shaped = if can_slice_prepared_shaped_run(&self.prepared, start, end) {
            Rc::new(slice_shaped_run(&self.shared.shaped_run, self.text_index(start), self.text_index(end)))
        } else {
            let text = build_line_text_from_range(&self.prepared, start, end, false);
            self.shared.state.shape_text(&text, self.options())?
        };
        // Backend code above may re-enter the caller; no map borrow is held.
        Ok(self.shared.line_runs.borrow_mut().entry(key).or_insert(shaped).clone())
    }

    fn text_index(&self, cursor: LayoutCursor) -> usize {
        self.shared.grapheme_boundary_text_indexes.get(cursor.segment_index)
            .and_then(|boundaries| boundaries.get(cursor.grapheme_index.min(boundaries.len().saturating_sub(1))))
            .copied().unwrap_or(self.shared.text_utf16_length)
    }
}

impl PretextLayout {
    pub fn shape_text(&self, text: &str, font: &str, options: PretextShapeOptions) -> Result<Rc<PretextShapedRun>, PretextError> {
        self.get_shaper_state(font)?.shape_text(text, options)
    }

    /// Convenience for hosts that can continue without glyph output.
    /// Use [`Self::shape_text`] when the precise backend error is needed.
    pub fn try_shape_text(&self, text: &str, font: &str, options: PretextShapeOptions) -> Option<Rc<PretextShapedRun>> {
        self.shape_text(text, font, options).ok()
    }

    pub fn shape_prepared_text(&self, prepared: Rc<PreparedTextWithSegments>, options: PretextShapeOptions) -> Result<Rc<PreparedShapedText>, PretextError> {
        let key = (Rc::as_ptr(&prepared) as usize, options.direction);
        let cached = {
            let cache = self.prepared_shaped.borrow();
            cache.get(&key).and_then(|entry| {
                entry.prepared.upgrade().filter(|source| Rc::ptr_eq(source, &prepared))
                    .map(|_| (entry.shared.clone(), entry.wrapper.upgrade()))
            })
        };
        if let Some((shared, wrapper)) = cached {
            if let Some(wrapper) = wrapper { return Ok(wrapper); }
            let wrapper = Rc::new(PreparedShapedText { prepared, shared });
            if let Some(entry) = self.prepared_shaped.borrow_mut().get_mut(&key) {
                entry.wrapper = Rc::downgrade(&wrapper);
            }
            return Ok(wrapper);
        }
        let generation = self.cache_generation.get();
        let state = self.get_shaper_state(prepared.font())?;
        let (text, grapheme_boundary_text_indexes, text_utf16_length) = build_prepared_shaping_text(&prepared);
        let shaped_run = state.shape_text(&text, options)?;
        let shared = Rc::new(PreparedShapingData {
            direction: options.direction, text, shaped_run, grapheme_boundary_text_indexes,
            text_utf16_length, state, line_runs: RefCell::new(HashMap::new()),
        });
        let shaped = Rc::new(PreparedShapedText { prepared, shared });
        if generation == self.cache_generation.get() {
            let retired = {
                let mut cache = self.prepared_shaped.borrow_mut();
                let dead_keys: Vec<_> = cache.iter().filter(|(_, value)| value.prepared.strong_count() == 0).map(|(key, _)| *key).collect();
                let mut retired: Vec<_> = dead_keys.iter().filter_map(|key| cache.remove(key)).collect();
                retired.extend(cache.insert(key, PreparedShapedCacheEntry {
                    prepared: Rc::downgrade(&shaped.prepared),
                    shared: shaped.shared.clone(),
                    wrapper: Rc::downgrade(&shaped),
                }));
                retired
            };
            // Dropping a cache payload can release a user-provided backend.
            drop(retired);
        }
        Ok(shaped)
    }

    pub fn layout_next_shaped_line(prepared: &PreparedShapedText, start: LayoutCursor, max_width: f64) -> Result<Option<ShapedLayoutLine>, PretextError> {
        Self::layout_next_line_range(&prepared.prepared, start, max_width)
            .map(|line| Self::materialize_shaped_line_range(prepared, &line)).transpose()
    }

    pub fn materialize_shaped_line_range(prepared: &PreparedShapedText, line: &LayoutLineRange) -> Result<ShapedLayoutLine, PretextError> {
        Ok(ShapedLayoutLine { shaped_run: prepared.get_or_create_line_run(line.start, line.end)?, width: line.width, start: line.start, end: line.end })
    }

    /// Convenience overload for callers retaining only their prepared metrics.
    pub fn materialize_shaped_line_range_with_options(&self, prepared: Rc<PreparedTextWithSegments>, line: &LayoutLineRange, options: PretextShapeOptions) -> Result<ShapedLayoutLine, PretextError> {
        let shaped = self.shape_prepared_text(prepared, options)?;
        Self::materialize_shaped_line_range(&shaped, line)
    }

    pub fn materialize_shaped_rich_inline_line_range(&self, prepared: &PreparedRichInline, line: &RichInlineLineRange, options: PretextShapeOptions) -> Result<ShapedRichInlineLine, PretextError> {
        let mut fragments = Vec::with_capacity(line.fragments.len());
        for fragment in &line.fragments {
            let item = prepared.prepared_item(fragment.item_index)
                .ok_or_else(|| PretextError::InvalidInput("rich-inline fragment refers to a missing item".into()))?;
            let shaped = self.shape_prepared_text(item.clone(), options)?;
            fragments.push(ShapedRichInlineFragment {
                item_index: fragment.item_index,
                shaped_run: shaped.get_or_create_line_run(fragment.start, fragment.end)?,
                gap_before: fragment.gap_before,
                occupied_width: fragment.occupied_width,
                start: fragment.start,
                end: fragment.end,
            });
        }
        Ok(ShapedRichInlineLine { fragments, width: line.width, end: line.end })
    }

    fn get_shaper_state(&self, font: &str) -> Result<Rc<ShaperState>, PretextError> {
        if let Some(cached) = self.shaper_states.borrow().get(font) { return Ok(cached.clone()); }
        let generation = self.cache_generation.get();
        let factory = self.text_shaper_factory()?;
        let state = Rc::new(ShaperState { shaper: factory.create_shaper(font)?, cache: RefCell::new(HashMap::new()) });
        if generation == self.cache_generation.get() {
            self.shaper_states.borrow_mut().insert(font.to_owned(), state.clone());
        }
        Ok(state)
    }
}

pub(crate) struct ShaperState {
    shaper: Rc<dyn IPretextTextShaper>,
    // Nested by direction to allow borrowed &str lookup with no key allocation
    // on cache hits.
    cache: RefCell<HashMap<PretextTextDirection, HashMap<String, Rc<PretextShapedRun>>>>,
}

impl ShaperState {
    fn shape_text(&self, text: &str, options: PretextShapeOptions) -> Result<Rc<PretextShapedRun>, PretextError> {
        if let Some(cached) = self.cache.borrow().get(&options.direction).and_then(|cache| cache.get(text)) {
            return Ok(cached.clone());
        }
        let shaped = Rc::new(self.shaper.shape_text(text, options)?);
        Ok(self.cache.borrow_mut().entry(options.direction).or_default().entry(text.into()).or_insert(shaped).clone())
    }
}

fn build_prepared_shaping_text(prepared: &PreparedTextWithSegments) -> (String, Vec<Vec<usize>>, usize) {
    let capacity = prepared.segments.iter().enumerate().filter(|(index, _)| !is_invisible_break_segment(prepared, *index)).map(|(_, text)| text.len()).sum();
    let mut text = String::with_capacity(capacity);
    let mut boundaries = Vec::with_capacity(prepared.segments.len());
    let mut utf16_length = 0;
    for (index, segment) in prepared.segments.iter().enumerate() {
        let offsets = segment_grapheme_offsets(prepared, index);
        let mut segment_boundaries = vec![utf16_length; offsets.len()];
        if !is_invisible_break_segment(prepared, index) {
            for (grapheme_index, pair) in offsets.windows(2).enumerate() {
                segment_boundaries[grapheme_index] = utf16_length;
                let grapheme = &segment[pair[0]..pair[1]];
                text.push_str(grapheme);
                utf16_length += grapheme.encode_utf16().count();
            }
            if let Some(end) = segment_boundaries.last_mut() { *end = utf16_length; }
        }
        boundaries.push(segment_boundaries);
    }
    (text, boundaries, utf16_length)
}

fn validate_range(prepared: &PreparedTextWithSegments, start: LayoutCursor, end: LayoutCursor) -> Result<(), PretextError> {
    let valid = |cursor: LayoutCursor| {
        if cursor.segment_index == prepared.segments.len() { cursor.grapheme_index == 0 }
        else { cursor.segment_index < prepared.segments.len() && cursor.grapheme_index < segment_grapheme_offsets(prepared, cursor.segment_index).len() }
    };
    if start > end || !valid(start) || !valid(end) { Err(PretextError::InvalidInput("shaped line cursor is outside the prepared text".into())) }
    else { Ok(()) }
}

fn can_slice_prepared_shaped_run(prepared: &PreparedTextWithSegments, start: LayoutCursor, end: LayoutCursor) -> bool {
    !line_has_discretionary_hyphen(prepared, start, end.segment_index)
        && is_shaping_safe_boundary(prepared, start) && is_shaping_safe_boundary(prepared, end)
}

fn is_shaping_safe_boundary(prepared: &PreparedTextWithSegments, cursor: LayoutCursor) -> bool {
    if cursor.segment_index >= prepared.segments.len() { return true; }
    if cursor.grapheme_index == 0 { return !is_invisible_break_boundary(prepared, cursor.segment_index); }
    cursor.grapheme_index >= segment_grapheme_offsets(prepared, cursor.segment_index).len().saturating_sub(1)
        && !is_invisible_break_boundary(prepared, cursor.segment_index + 1)
}

fn is_invisible_break_boundary(prepared: &PreparedTextWithSegments, index: usize) -> bool {
    is_invisible_break_segment(prepared, index) || index.checked_sub(1).is_some_and(|previous| is_invisible_break_segment(prepared, previous))
}

fn is_invisible_break_segment(prepared: &PreparedTextWithSegments, index: usize) -> bool {
    matches!(prepared.kinds.get(index), Some(SegmentBreakKind::ZeroWidthBreak | SegmentBreakKind::SoftHyphen | SegmentBreakKind::HardBreak))
}

fn slice_shaped_run(source: &PretextShapedRun, start: usize, end: usize) -> PretextShapedRun {
    let mut glyphs = Vec::new();
    let mut font_map = HashMap::new();
    let mut font_runs: Vec<PretextShapedFontRun> = Vec::new();
    let mut origin = None;
    let mut advance_x = 0.0;
    let mut advance_y = 0.0;
    for glyph in source.glyphs().iter().filter(|glyph| glyph.cluster >= start && glyph.cluster < end) {
        let (origin_x, origin_y) = *origin.get_or_insert((glyph.x, glyph.y));
        let mapped_index = *font_map.entry(glyph.font_run_index).or_insert_with(|| {
            let index = font_runs.len();
            let font = source.font_runs().iter().find(|run| run.index == glyph.font_run_index).or_else(|| source.font_runs().first());
            font_runs.push(PretextShapedFontRun { index, font: font.map(|run| run.font.clone()).unwrap_or_default(), first_glyph_index: glyphs.len(), glyph_count: 0 });
            index
        });
        glyphs.push(PretextShapedGlyph { cluster: glyph.cluster - start, x: glyph.x - origin_x, y: glyph.y - origin_y, font_run_index: mapped_index, ..*glyph });
        font_runs[mapped_index].glyph_count += 1;
        advance_x += glyph.x_advance;
        advance_y += glyph.y_advance;
    }
    PretextShapedRun::new(source.kind(), glyphs, font_runs, advance_x, advance_y)
}
